use rodio::Source;
use std::time::Duration;

pub trait BlockProcessor: Send {
    fn process_sample(&mut self, sample: f32) -> Option<f32>;
}

pub struct ProcessedSource {
    pub samples: Vec<f32>,
    pub position: usize,
    pub channels: u16,
    pub sample_rate: u32,
    processors: Vec<Box<dyn BlockProcessor + Send>>,
}

impl ProcessedSource {
    pub fn get_samples(&self) -> &Vec<f32> {
        &self.samples
    }

    pub fn from_source<T>(source: T, processors: Vec<Box<dyn BlockProcessor + Send>>) -> Self
    where
        T: rodio::Source<Item = f32>,
    {
        let channels = source.channels();
        let sample_rate = source.sample_rate();
        let samples: Vec<f32> = source.collect();
        ProcessedSource {
            samples,
            position: 0,
            channels,
            sample_rate,
            processors,
        }
    }
}

impl Iterator for ProcessedSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.samples.len() {
            return None;
        }
        let mut sample = self.samples[self.position];
        // let output_sample = self.processor.process(input_sample);

        for proc in self.processors.iter_mut() {
            if let Some(processed_sample) = proc.process_sample(sample) {
                sample = processed_sample;
            }
        }
        self.position += 1;
        Some(sample)
    }
}

impl Source for ProcessedSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(
            self.samples.len() as f32 / self.sample_rate as f32 / self.channels as f32,
        ))
    }
}
