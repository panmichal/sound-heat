use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use rodio::{Decoder as RodioDecoder, OutputStream, Source};
use rustfft::{FftPlanner, num_complex::Complex};
use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{BufReader, stdout};
use std::thread::sleep;
use std::time::Duration;

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

    while pos < samples.len() {
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
            let frame: Vec<f32> = ring.iter().cloned().collect();
            draw_spectrum(&frame, sample_rate, fft_size);
        }

        sleep(Duration::from_secs_f32(
            hop_size as f32 / sample_rate as f32,
        ));
    }
    sink.sleep_until_end();
}

fn draw_spectrum(samples: &[f32], sample_rate: u32, fft_size: usize) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();
    fft.process(&mut buffer);

    let spectrum: Vec<f32> = buffer.iter().map(|c| c.norm() / fft_size as f32).collect();

    // Draw chart (reuse your previous code, but clear terminal first)
    execute!(stdout(), Clear(ClearType::All)).unwrap();
    let num_bands = 32;
    let min_db = -100.0;
    let max_db = 0.0;
    for band in 0..num_bands {
        let low_freq = band as f32 * sample_rate as f32 / 2.0 / num_bands as f32;
        let high_freq = (band + 1) as f32 * sample_rate as f32 / 2.0 / num_bands as f32;
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
        let bar_len = (((db - min_db) / (max_db - min_db)) * 50.0).max(0.0) as usize;
        let bar = "█".repeat(bar_len);
        println!(
            "{:4.0} Hz - {:4.0} Hz | {:>4.1} dB | {}",
            low_freq, high_freq, db, bar
        );
    }
}
