mod decode;

use rustfft::{FftPlanner, num_complex::Complex};
use std::env;
use std::fs::File;

// Define the sample rate (Hz) for the analysis. Most MP3s use 44100 Hz.
// This can be made dynamic if needed.
const SAMPLE_RATE: usize = 44100;

// Define the frequency bands for analysis as (name, low, high) in Hz.
// Edit this array to change the bands.
const BANDS: &[(&str, f32, f32)] = &[
    ("Low-end", 20.0, 120.0),
    ("Low-mid", 120.0, 500.0),
    ("Mid", 500.0, 2000.0),
    ("Top-end", 2000.0, 20000.0),
];
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
    // Wrap the file in a buffered reader and create an MP3 decoder.
    // let mut decoder = Decoder::new(BufReader::new(file));

    // // Collect all decoded audio samples (i16) and convert to f32 in the range [-1.0, 1.0].
    // let mut samples: Vec<f32> = Vec::new();
    // while let Ok(Frame { data, .. }) = decoder.next_frame() {
    //     samples.extend(data.iter().map(|&s| s as f32 / 32768.0));
    // }

    // println!("Decoded {} samples.", samples.len());

    let samples = decode::decode(file).expect("Failed to decode audio");

    let audio_duration = samples.len() as f32 / SAMPLE_RATE as f32;
    println!(
        "Audio duration: {:.0}m {:.0}s.",
        (audio_duration / 60.0).floor(),
        audio_duration % 60.0
    );

    // Set the FFT size (must be a power of 2, e.g., 4096).
    let fft_size = 4096;
    if samples.len() < fft_size {
        // Not enough data for FFT analysis.
        eprintln!("Not enough samples for FFT.");
        return;
    }

    // Prepare the input for FFT: take fft_size samples from the middle of the vector and convert to complex numbers.
    let mid = samples.len() / 2;
    let start = if mid >= fft_size / 2 {
        mid - fft_size / 2
    } else {
        0
    };
    let end = (start + fft_size).min(samples.len());
    let input: Vec<Complex<f32>> = samples[start..end]
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();

    // Create an FFT planner and plan a forward FFT of the chosen size.
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = input.clone();
    // Perform the FFT in-place.
    fft.process(&mut buffer);

    // Calculate the magnitude (absolute value) of each FFT output bin.
    let spectrum: Vec<f32> = buffer.iter().map(|c| c.norm()).collect();

    // For each frequency band, compute the average magnitude in the corresponding FFT bins.
    println!("\nAverage frequency content per band:");
    for &(name, low, high) in BANDS {
        // Convert frequency range to FFT bin indices.
        let low_bin = ((low as f32 / SAMPLE_RATE as f32) * fft_size as f32).floor() as usize;
        let high_bin = ((high as f32 / SAMPLE_RATE as f32) * fft_size as f32).ceil() as usize;
        // Get the slice of the spectrum for this band.
        let band_bins = &spectrum[low_bin..high_bin.min(spectrum.len())];
        // Compute the average magnitude for the band.
        let avg = if !band_bins.is_empty() {
            band_bins.iter().sum::<f32>() / band_bins.len() as f32 / fft_size as f32
        } else {
            0.0
        };
        let epsilon = 1e-10; // Small value to avoid log(0)
        let avg_db = 20.0 * (avg + epsilon).log10();

        println!("{} ({}-{} Hz): {:.4} dB", name, low, high, avg_db);
    }

    // Number of bands for the spectrum chart
    let num_bands = 32;
    let max_db = 0.0; // 0 dBFS (full scale)
    let min_db = -100.0; // Minimum dB to display

    println!("\nSpectrum Analyzer:");
    for band in 0..num_bands {
        // Calculate frequency range for this band
        let low_freq = band as f32 * SAMPLE_RATE as f32 / 2.0 / num_bands as f32;
        let high_freq = (band + 1) as f32 * SAMPLE_RATE as f32 / 2.0 / num_bands as f32;
        let low_bin = ((low_freq / SAMPLE_RATE as f32) * fft_size as f32).floor() as usize;
        let high_bin = ((high_freq / SAMPLE_RATE as f32) * fft_size as f32).ceil() as usize;

        // Average magnitude for the band, normalized
        let band_bins = &spectrum[low_bin..high_bin.min(spectrum.len())];
        let avg = if !band_bins.is_empty() {
            band_bins.iter().sum::<f32>() / band_bins.len() as f32 / fft_size as f32
        } else {
            0.0
        };
        let epsilon = 1e-10;
        let db = 20.0 * (avg + epsilon).log10();

        // Map dB to bar length
        let bar_len = (((db - min_db) / (max_db - min_db)) * 50.0).max(0.0) as usize;
        let bar = "█".repeat(bar_len);

        // Print band
        println!(
            "{:4.0} Hz - {:4.0} Hz | {:>4.1} dB | {}",
            low_freq, high_freq, db, bar
        );
    }
}
