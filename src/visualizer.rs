use std::sync::{Arc, Mutex};

const BYTES_PER_SAMPLE: usize = 2;
const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = BYTES_PER_SAMPLE * CHANNELS;

pub const VISUALIZATION_BAR_COUNT: usize = 10;

const RISE_SMOOTHING_FACTOR: f64 = 0.8;
const FALL_SMOOTHING_FACTOR: f64 = 0.22;
const RMS_SENSITIVITY: f64 = 32.0;
const PEAK_SENSITIVITY: f64 = 2.0;

/// Maintains the smoothed state of the audio visualization.
pub struct Visualizer {
    levels: [f64; VISUALIZATION_BAR_COUNT],
    partial_frame: [u8; BYTES_PER_FRAME],
    partial_frame_len: usize,
    frame_energies: Vec<(f64, f64)>,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            levels: [0.0; VISUALIZATION_BAR_COUNT],
            partial_frame: [0; BYTES_PER_FRAME],
            partial_frame_len: 0,
            frame_energies: Vec::with_capacity(4096),
        }
    }

    /// Processes raw stereo s16le PCM data and updates the visualization.
    pub fn process(&mut self, audio_data: &[u8], visualization_data: &Arc<Mutex<Vec<u8>>>) {
        self.frame_energies.clear();

        let mut input = audio_data;

        if self.partial_frame_len > 0 {
            let bytes_needed = BYTES_PER_FRAME - self.partial_frame_len;

            if input.len() < bytes_needed {
                self.partial_frame[self.partial_frame_len..self.partial_frame_len + input.len()]
                    .copy_from_slice(input);

                self.partial_frame_len += input.len();
                return;
            }

            self.partial_frame[self.partial_frame_len..].copy_from_slice(&input[..bytes_needed]);

            self.frame_energies.push(frame_energy(&self.partial_frame));

            self.partial_frame_len = 0;
            input = &input[bytes_needed..];
        }

        let complete_bytes = input.len() - (input.len() % BYTES_PER_FRAME);

        for frame in input[..complete_bytes].chunks_exact(BYTES_PER_FRAME) {
            self.frame_energies.push(frame_energy(frame));
        }

        let trailing_bytes = &input[complete_bytes..];

        if !trailing_bytes.is_empty() {
            self.partial_frame[..trailing_bytes.len()].copy_from_slice(trailing_bytes);

            self.partial_frame_len = trailing_bytes.len();
        }

        if self.frame_energies.is_empty() {
            return;
        }

        let frames_per_bar = self.frame_energies.len().div_ceil(VISUALIZATION_BAR_COUNT);

        let mut target_levels = [0.0f64; VISUALIZATION_BAR_COUNT];

        for (bar, level) in target_levels.iter_mut().enumerate() {
            let start_frame = bar * frames_per_bar;
            let end_frame = ((bar + 1) * frames_per_bar).min(self.frame_energies.len());

            if start_frame >= end_frame {
                continue;
            }

            let samples = &self.frame_energies[start_frame..end_frame];

            let rms = (samples.iter().map(|energy| energy.0).sum::<f64>() / (samples.len() as f64))
                .sqrt();

            let peak = samples.iter().map(|energy| energy.1).fold(0.0f64, f64::max);

            *level = (rms * RMS_SENSITIVITY + peak * PEAK_SENSITIVITY).clamp(0.0, 10.0);
        }

        let mut output_levels = [0u8; VISUALIZATION_BAR_COUNT];

        for (index, target) in target_levels.iter().enumerate() {
            let current = self.levels[index];

            let smoothing_factor = if *target > current {
                RISE_SMOOTHING_FACTOR
            } else {
                FALL_SMOOTHING_FACTOR
            };

            let smoothed = current + (*target - current) * smoothing_factor;

            self.levels[index] = smoothed;
            output_levels[index] = smoothed.round().clamp(0.0, 10.0) as u8;
        }

        if let Ok(mut data) = visualization_data.lock() {
            if data.len() != VISUALIZATION_BAR_COUNT {
                data.resize(VISUALIZATION_BAR_COUNT, 0);
            }

            data.copy_from_slice(&output_levels);
        }
    }

    /// Resets all visualization levels to zero.
    pub fn reset(&mut self, visualization_data: &Arc<Mutex<Vec<u8>>>) {
        self.levels.fill(0.0);
        self.partial_frame_len = 0;
        self.frame_energies.clear();

        if let Ok(mut data) = visualization_data.lock() {
            data.fill(0);
        }
    }
}

fn frame_energy(frame: &[u8]) -> (f64, f64) {
    let left = (i16::from_le_bytes([frame[0], frame[1]]) as f64) / 32768.0;

    let right = (i16::from_le_bytes([frame[2], frame[3]]) as f64) / 32768.0;

    let peak = left.abs().max(right.abs());

    // Keep both channels' energy so opposite stereo signals do not cancel out.
    ((left * left + right * right) * 0.5, peak)
}

impl Default for Visualizer {
    fn default() -> Self {
        Self::new()
    }
}
