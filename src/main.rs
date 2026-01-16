mod filter;
mod source;
mod spectrum;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{Decoder as RodioDecoder, Source};
use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::Write;
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
    let fft_size = 4096;
    let hop_size = fft_size / 2;
    println!("Loaded audio: {} Hz, {} channels", sample_rate, channels);
    let processors: Vec<Box<dyn source::BlockProcessor + Send>> = vec![
        Box::new(filter::LowPassFilterBlockProcessor {
            prev: 0.0,
            cutoff: 500.0,
            sample_rate,
        }),
        Box::new(spectrum::SpectrumBlockProcessor {
            spectrum: spectrum::Spectrum::new(
                NUM_BANDS,
                MIN_DB,
                MAX_DB,
                0.8,
                fft_size,
                hop_size,
                sample_rate,
            ),
            stdout: stdout(),
            channels,
            fft_buffer: VecDeque::with_capacity(fft_size * channels),
        }),
    ];

    let processed_source = source::ProcessedSource::from_source(source, processors);

    let total_duration = processed_source
        .total_duration()
        .map_or(0.0, |d| d.as_secs_f32());
    let samples: Vec<f32> = processed_source.get_samples().clone();

    println!("Total samples loaded: {}", samples.len());
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
    let mixer = stream_handle.mixer();
    let sink = rodio::Sink::connect_new(mixer);

    enable_raw_mode().unwrap();
    execute!(stdout(), EnterAlternateScreen).unwrap();

    sink.append(processed_source);
    println!("Playback started...");

    let mut paused = false;

    while !sink.empty() {
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

        execute!(
            stdout(),
            crossterm::cursor::MoveTo(0, NUM_BANDS as u16 + 2),
            crossterm::style::Print(format!(
                "Current position: {} / {}",
                format_duration(sink.get_pos().as_secs_f32()),
                format_duration(total_duration)
            )),
        )
        .unwrap();

        stdout().flush().unwrap();

        sleep(Duration::from_millis(100));
    }

    sink.sleep_until_end();
    disable_raw_mode().unwrap();
    execute!(stdout(), LeaveAlternateScreen).unwrap();
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
