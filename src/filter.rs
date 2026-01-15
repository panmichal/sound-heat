use crate::source::BlockProcessor;

pub struct LowPassFilterBlockProcessor {
    pub prev: f32,
    pub cutoff: f32,
    pub sample_rate: u32,
}

impl BlockProcessor for LowPassFilterBlockProcessor {
    fn process_sample(&mut self, input: f32) -> Option<f32> {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff);
        let dt = 1.0 / self.sample_rate as f32;
        let alpha = dt / (rc + dt);
        let output = alpha * input + (1.0 - alpha) * self.prev;
        self.prev = output;
        Some(output)
    }
}
