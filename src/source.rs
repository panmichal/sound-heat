use rodio::Source;
use std::time::Duration;

pub struct SampleProcessor<S, F>
where
    F: FnMut(f32, &mut S) -> f32,
{
    pub state: S,
    pub process_fn: F,
}

impl<S, F> SampleProcessor<S, F>
where
    F: FnMut(f32, &mut S) -> f32,
{
    pub fn process(&mut self, input: f32) -> f32 {
        (self.process_fn)(input, &mut self.state)
    }

    pub fn new(state: S, process_fn: F) -> Self {
        SampleProcessor { state, process_fn }
    }
}

pub struct ProcessedSource<S, F>
where
    F: FnMut(f32, &mut S) -> f32,
{
    pub samples: Vec<f32>,
    pub position: usize,
    pub channels: u16,
    pub sample_rate: u32,
    pub processor: SampleProcessor<S, F>,
}

impl<S, F> ProcessedSource<S, F>
where
    F: FnMut(f32, &mut S) -> f32,
{
    pub fn get_samples(&self) -> &Vec<f32> {
        &self.samples
    }

    pub fn from_source<T>(source: T, processor: SampleProcessor<S, F>) -> Self
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
            processor,
        }
    }
}

impl<S, F> Iterator for ProcessedSource<S, F>
where
    F: FnMut(f32, &mut S) -> f32,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.samples.len() {
            return None;
        }
        let input_sample = self.samples[self.position];
        let output_sample = self.processor.process(input_sample);
        self.position += 1;
        Some(output_sample)
    }
}

impl<S, F> Source for ProcessedSource<S, F>
where
    F: FnMut(f32, &mut S) -> f32,
{
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
