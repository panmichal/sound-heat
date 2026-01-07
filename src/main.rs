use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{Decoder as RodioDecoder, OutputStream, Source};
use rustfft::{FftPlanner, num_complex::Complex};
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

    // Open the MP3 file for reading.
    let file = File::open(file_path).expect("Failed to open file");

    let source = RodioDecoder::new(BufReader::new(file)).unwrap();
    let sample_rate = source.sample_rate();
    let channels = source.channels() as usize;
    println!("Loaded audio: {} Hz, {} channels", sample_rate, channels);
    let samples: Vec<f32> = source.convert_samples::<f32>().collect();
    println!("Total samples loaded: {}", samples.len());
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    let play_source =
        rodio::buffer::SamplesBuffer::new(channels as u16, sample_rate, samples.clone());
    sink.append(play_source);
    println!("Playback started...");

    let fft_size = 4096;
    let hop_size = fft_size / 2;

    let mut pos = 0;

    let mut ring: VecDeque<f32> = VecDeque::with_capacity(fft_size * channels);
    let mut smoothed_by_band = vec![MIN_DB; NUM_BANDS];

    let mut paused = false;

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
            draw_spectrum(&frame, sample_rate, fft_size, &mut smoothed_by_band);
        }

        sleep(Duration::from_secs_f32(
            hop_size as f32 / sample_rate as f32,
        ));
    }
    execute!(stdout(), LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
    sink.sleep_until_end();
}

fn draw_spectrum(
    samples: &[f32],
    sample_rate: u32,
    fft_size: usize,
    smoothed_by_band: &mut Vec<f32>,
) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let hann = 0.5
                * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (samples.len() as f32 - 1.0)).cos());
            Complex {
                re: s * hann,
                im: 0.0,
            }
        })
        .collect();
    fft.process(&mut buffer);

    let spectrum: Vec<f32> = buffer.iter().map(|c| c.norm() / fft_size as f32).collect();

    execute!(stdout(), Clear(ClearType::All)).unwrap();

    let smooth_factor = 0.8;

    let min_freq: f32 = 20.0;
    let max_freq: f32 = sample_rate as f32 / 2.0;
    let log_min = min_freq.ln();
    let log_max = max_freq.ln();

    let mut stdout = stdout();
    for band in 0..NUM_BANDS {
        let log_low = log_min + (log_max - log_min) * (band as f32) / (NUM_BANDS as f32);
        let log_high = log_min + (log_max - log_min) * ((band + 1) as f32) / (NUM_BANDS as f32);
        let low_freq = log_low.exp();
        let high_freq = log_high.exp();

        let low_bin = ((low_freq / sample_rate as f32) * fft_size as f32).floor() as usize;
        let high_bin = ((high_freq / sample_rate as f32) * fft_size as f32).ceil() as usize;
        let band_bins = &spectrum[low_bin..high_bin.min(spectrum.len())];
        let avg = if !band_bins.is_empty() {
            band_bins.iter().sum::<f32>() / band_bins.len() as f32
        } else {
            0.0
        };
        let epsilon = 1e-10;
        let db = 20.0 * (avg + epsilon).log10();
        smoothed_by_band[band] =
            smooth_factor * smoothed_by_band[band] + (1.0 - smooth_factor) * db;
        let bar_len =
            (((smoothed_by_band[band] - MIN_DB) / (MAX_DB - MIN_DB)) * 150.0).max(0.0) as usize;
        let bar = "█".repeat(bar_len);
        // println!(
        //     "{:4.0} Hz - {:4.0} Hz | {:>4.1} dB | {}",
        //     low_freq, high_freq, db, bar
        // );
        execute!(
            stdout,
            crossterm::cursor::MoveTo(0, band as u16),
            crossterm::style::Print(format!(
                "{:4.0} Hz - {:4.0} Hz | {:>4.1} dB | {}",
                low_freq, high_freq, db, bar
            )),
        )
        .unwrap();
    }

    stdout.flush().unwrap();
}
