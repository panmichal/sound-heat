mod spectrum;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{Decoder as RodioDecoder, Source};
use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{BufReader, stdout};
use std::thread::sleep;
use std::time::Duration;

const NUM_BANDS: usize = 32;
const MIN_DB: f32 = -100.0;
const MAX_DB: f32 = 0.0;

fn main() {
    // Collect command line arguments into a vector of strings.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        // Print usage if no file path is provided.
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];
    println!("File path provided: {}", file_path);

    let source = load_audio(file_path).unwrap();

    let sample_rate = source.sample_rate();
    let channels = source.channels() as usize;
    println!("Loaded audio: {} Hz, {} channels", sample_rate, channels);
    let samples: Vec<f32> = source.collect();
    println!("Total samples loaded: {}", samples.len());
    // let (_stream, stream_handle) = OutputStream::from_default_device().unwrap();
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
    let mixer = stream_handle.mixer();
    let sink = rodio::Sink::connect_new(mixer);
    let play_source =
        rodio::buffer::SamplesBuffer::new(channels as u16, sample_rate, samples.clone());

    let play_source_clone = play_source.clone();
    sink.append(play_source);

    println!("Playback started...");

    let fft_size = 4096;
    let hop_size = fft_size / 2;

    let mut pos = 0;

    let mut ring: VecDeque<f32> = VecDeque::with_capacity(fft_size * channels);

    let mut paused = false;
    let mut spectrum =
        spectrum::Spectrum::new(NUM_BANDS, MIN_DB, MAX_DB, 0.8, fft_size, sample_rate);

    enable_raw_mode().unwrap();
    execute!(stdout(), EnterAlternateScreen).unwrap();

    while pos < samples.len() {
        if event::poll(Duration::from_millis(10)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    ..
                }) => {
                    paused = !paused;
                    if paused {
                        sink.pause();
                    } else {
                        sink.play();
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }) => {
                    sink.stop();
                    break;
                }
                _ => {}
            }
        }

        if paused {
            sleep(Duration::from_millis(10));
            continue;
        }

        let end = (pos + hop_size * channels).min(samples.len());
        let chunk = &samples[pos..end];

        for &s in chunk {
            if ring.len() == fft_size * channels {
                ring.pop_front();
            }
            ring.push_back(s);
        }
        pos = end;

        if ring.len() == fft_size * channels {
            let mut frame: Vec<f32> = Vec::with_capacity(fft_size);
            for i in 0..fft_size {
                let mut sum = 0.0;
                for ch in 0..channels {
                    sum += ring[i * channels + ch];
                }
                frame.push(sum / channels as f32);
            }

            execute!(stdout(), Clear(ClearType::All)).unwrap();

            execute!(
                stdout(),
                crossterm::cursor::MoveTo(0, NUM_BANDS as u16 + 2),
                crossterm::style::Print(format!(
                    "Current position: {} / {}",
                    format_duration(sink.get_pos().as_secs_f32()),
                    format_duration(
                        play_source_clone
                            .total_duration()
                            .map_or(0.0, |d| d.as_secs_f32())
                    )
                )),
            )
            .unwrap();
            spectrum.render(&frame, &mut stdout());
        }

        sleep(Duration::from_secs_f32(
            hop_size as f32 / sample_rate as f32,
        ));
    }
    execute!(stdout(), LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
    sink.sleep_until_end();
}

fn load_audio(file_path: &str) -> Result<RodioDecoder<BufReader<File>>, String> {
    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    let decoder =
        RodioDecoder::new(reader).map_err(|e| format!("Failed to decode audio: {}", e))?;
    Ok(decoder)
}

fn format_duration(seconds: f32) -> String {
    let mins = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}", mins, secs)
}
