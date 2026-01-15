pub struct LowPassFilterState {
    pub prev: f32,
    pub cutoff: f32,
    pub sample_rate: u32,
}

pub fn low_pass_filter_fn(input: f32, state: &mut LowPassFilterState) -> f32 {
    let rc = 1.0 / (2.0 * std::f32::consts::PI * state.cutoff);
    let dt = 1.0 / state.sample_rate as f32;
    let alpha = dt / (rc + dt);
    let output = alpha * input + (1.0 - alpha) * state.prev;
    state.prev = output;
    output
}
