use std::io::Stdout;

use crossterm::execute;
use rustfft::{FftPlanner, num_complex::Complex};
use std::io::Write;

pub struct Spectrum {
    pub bands: usize,
    pub min_db: f32,
    pub max_db: f32,
    pub smooth_factor: f32,
    pub smoothed_by_band: Vec<f32>,
    pub fft_size: usize,
    pub fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    pub sample_rate: u32,
}

impl Spectrum {
    pub fn new(
        bands: usize,
        min_db: f32,
        max_db: f32,
        smooth_factor: f32,
        fft_size: usize,
        sample_rate: u32,
    ) -> Self {
        Spectrum {
            bands,
            min_db,
            max_db,
            smooth_factor,
            smoothed_by_band: vec![min_db; bands],
            fft_size,
            fft: FftPlanner::<f32>::new().plan_fft_forward(fft_size),
            sample_rate,
        }
    }

    pub fn render(&mut self, samples: &[f32], stdout: &mut Stdout) {
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let hann = 0.5
                    * (1.0
                        - (2.0 * std::f32::consts::PI * i as f32 / (samples.len() as f32 - 1.0))
                            .cos());
                Complex {
                    re: s * hann,
                    im: 0.0,
                }
            })
            .collect();
        self.fft.process(&mut buffer);

        let spectrum: Vec<f32> = buffer
            .iter()
            .map(|c| c.norm() / self.fft_size as f32)
            .collect();

        let min_freq: f32 = 20.0;
        let max_freq: f32 = self.sample_rate as f32 / 2.0;
        let log_min = min_freq.ln();
        let log_max = max_freq.ln();

        for band in 0..self.bands {
            let log_low = log_min + (log_max - log_min) * (band as f32) / (self.bands as f32);
            let log_high =
                log_min + (log_max - log_min) * ((band + 1) as f32) / (self.bands as f32);
            let low_freq = log_low.exp();
            let high_freq = log_high.exp();

            let low_bin =
                ((low_freq / self.sample_rate as f32) * self.fft_size as f32).floor() as usize;
            let high_bin =
                ((high_freq / self.sample_rate as f32) * self.fft_size as f32).ceil() as usize;
            let band_bins = &spectrum[low_bin..high_bin.min(spectrum.len())];
            let avg = if !band_bins.is_empty() {
                band_bins.iter().sum::<f32>() / band_bins.len() as f32
            } else {
                0.0
            };
            let epsilon = 1e-10;
            let db = 20.0 * (avg + epsilon).log10();
            self.smoothed_by_band[band] =
                self.smooth_factor * self.smoothed_by_band[band] + (1.0 - self.smooth_factor) * db;
            let bar_len = (((self.smoothed_by_band[band] - self.min_db)
                / (self.max_db - self.min_db))
                * 150.0)
                .max(0.0) as usize;
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
}
