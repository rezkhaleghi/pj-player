use std::sync::{ Arc, Mutex };

const BYTES_PER_SAMPLE: usize = 2;
const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = BYTES_PER_SAMPLE * CHANNELS;

pub const VISUALIZATION_BAR_COUNT: usize = 10;

const RISE_SMOOTHING_FACTOR: f64 = 0.65;
const FALL_SMOOTHING_FACTOR: f64 = 0.15;
const AMPLITUDE_BOOST: f64 = 20.0;

/// Maintains the smoothed state of the audio visualization.
pub struct Visualizer {
    levels: Vec<f64>,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            levels: vec![0.0; VISUALIZATION_BAR_COUNT],
        }
    }

    /// Processes raw stereo s16le PCM data and updates the visualization.
    pub fn process(&mut self, audio_data: &[u8], visualization_data: &Arc<Mutex<Vec<u8>>>) {
        if audio_data.len() < BYTES_PER_FRAME {
            return;
        }

        let frame_count = audio_data.len() / BYTES_PER_FRAME;

        if frame_count == 0 {
            return;
        }

        let frames_per_bar = frame_count.div_ceil(VISUALIZATION_BAR_COUNT);

        let mut target_levels = vec![0.0f64; VISUALIZATION_BAR_COUNT];

        for (bar, level) in target_levels.iter_mut().enumerate() {
            let start_frame = bar * frames_per_bar;
            let end_frame = ((bar + 1) * frames_per_bar).min(frame_count);

            if start_frame >= end_frame {
                continue;
            }

            let start_byte = start_frame * BYTES_PER_FRAME;
            let end_byte = end_frame * BYTES_PER_FRAME;

            let mut sum = 0.0f64;
            let mut count = 0usize;

            for frame in audio_data[start_byte..end_byte].chunks_exact(BYTES_PER_FRAME) {
                let left = i16::from_le_bytes([frame[0], frame[1]]);
                let right = i16::from_le_bytes([frame[2], frame[3]]);

                let left = (left as f64) / (i16::MAX as f64);
                let right = (right as f64) / (i16::MAX as f64);

                // Combine the stereo channels into one amplitude value.
                let sample = (left + right) / 2.0;

                sum += sample * sample;
                count += 1;
            }

            if count == 0 {
                continue;
            }

            let rms = (sum / (count as f64)).sqrt();

            *level = (rms * AMPLITUDE_BOOST).clamp(0.0, 10.0);
        }

        let mut output_levels = vec![0u8; VISUALIZATION_BAR_COUNT];

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
            *data = output_levels;
        }
    }

    /// Resets all visualization levels to zero.
    pub fn reset(&mut self, visualization_data: &Arc<Mutex<Vec<u8>>>) {
        self.levels.fill(0.0);

        if let Ok(mut data) = visualization_data.lock() {
            data.fill(0);
        }
    }
}

impl Default for Visualizer {
    fn default() -> Self {
        Self::new()
    }
}
