//! Stereo sample-rate conversion to the engine rate (rubato FFT resampler).

use dj_domain::ports::DecodeError;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};

const CHANNELS: usize = 2;
/// Input frames per resampler call.
const CHUNK: usize = 1024;

/// Converts interleaved stereo from one rate to another, streaming.
pub struct StereoResampler {
    inner: Option<Fft<f32>>,
    input: Vec<f32>,
    output: Vec<f32>,
    /// Output frames still to drop (the resampler's start-up delay).
    delay: usize,
    ratio: f64,
    frames_in: u64,
    frames_out: u64,
}

impl std::fmt::Debug for StereoResampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StereoResampler")
            .field("ratio", &self.ratio)
            .field("passthrough", &self.inner.is_none())
            .finish_non_exhaustive()
    }
}

impl StereoResampler {
    /// # Errors
    /// Returns an error if the rates are not supported by the resampler.
    pub fn new(source_rate: u32, target_rate: u32) -> Result<Self, DecodeError> {
        let ratio = f64::from(target_rate) / f64::from(source_rate.max(1));
        if source_rate == target_rate {
            return Ok(Self {
                inner: None,
                input: Vec::new(),
                output: Vec::new(),
                delay: 0,
                ratio,
                frames_in: 0,
                frames_out: 0,
            });
        }
        let inner = Fft::<f32>::new(
            source_rate as usize,
            target_rate as usize,
            CHUNK,
            CHANNELS,
            FixedSync::Input,
        )
        .map_err(|err| DecodeError::Unsupported(err.to_string()))?;
        let output = vec![0.0; inner.output_frames_max() * CHANNELS];
        let delay = inner.output_delay();
        Ok(Self {
            inner: Some(inner),
            input: Vec::with_capacity(CHUNK * CHANNELS * 2),
            output,
            delay,
            ratio,
            frames_in: 0,
            frames_out: 0,
        })
    }

    /// Appends the resampled version of `stereo` (interleaved) to `out`.
    ///
    /// # Errors
    /// Returns an error if the resampler fails.
    pub fn process(&mut self, stereo: &[f32], out: &mut Vec<f32>) -> Result<(), DecodeError> {
        if self.inner.is_none() {
            out.extend_from_slice(stereo);
            return Ok(());
        }
        self.frames_in += (stereo.len() / CHANNELS) as u64;
        self.input.extend_from_slice(stereo);
        while self.input.len() >= CHUNK * CHANNELS {
            self.run(CHUNK, out)?;
            self.input.drain(..CHUNK * CHANNELS);
        }
        Ok(())
    }

    /// Flushes buffered input and the resampler's internal delay.
    ///
    /// # Errors
    /// Returns an error if the resampler fails.
    pub fn finish(&mut self, out: &mut Vec<f32>) -> Result<(), DecodeError> {
        if self.inner.is_none() {
            return Ok(());
        }
        // Rounded expected length of the whole output.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let expected = (self.frames_in as f64 * self.ratio).round() as u64;
        let mut guard = 0;
        while self.frames_out < expected && guard < 64 {
            let partial = self.input.len() / CHANNELS;
            self.input.resize(CHUNK * CHANNELS, 0.0);
            self.run_partial(partial, out)?;
            self.input.clear();
            guard += 1;
        }
        let excess = self.frames_out.saturating_sub(expected);
        let keep = out
            .len()
            .saturating_sub(usize::try_from(excess).unwrap_or(0) * CHANNELS);
        out.truncate(keep);
        Ok(())
    }

    fn run(&mut self, frames: usize, out: &mut Vec<f32>) -> Result<(), DecodeError> {
        self.run_with(frames, None, out)
    }

    fn run_partial(&mut self, valid: usize, out: &mut Vec<f32>) -> Result<(), DecodeError> {
        self.run_with(CHUNK, Some(valid), out)
    }

    fn run_with(
        &mut self,
        frames: usize,
        partial: Option<usize>,
        out: &mut Vec<f32>,
    ) -> Result<(), DecodeError> {
        let Some(inner) = self.inner.as_mut() else {
            return Ok(());
        };
        let fail = |err: &dyn std::fmt::Display| DecodeError::Corrupt(format!("resampler: {err}"));
        let input = InterleavedSlice::new(&self.input[..frames * CHANNELS], CHANNELS, frames)
            .map_err(|err| fail(&err))?;
        let capacity = self.output.len() / CHANNELS;
        let mut output = InterleavedSlice::new_mut(&mut self.output, CHANNELS, capacity)
            .map_err(|err| fail(&err))?;
        let indexing = partial.map(|valid| Indexing {
            input_offset: 0,
            output_offset: 0,
            partial_len: Some(valid),
            active_channels_mask: None,
        });
        let (_, produced) = inner
            .process_into_buffer(&input, &mut output, indexing.as_ref())
            .map_err(|err| fail(&err))?;
        let skipped = self.delay.min(produced);
        self.delay -= skipped;
        let kept = produced - skipped;
        out.extend_from_slice(&self.output[skipped * CHANNELS..produced * CHANNELS]);
        self.frames_out += kept as u64;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, frequency: f64, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|n| {
                let v = (2.0 * std::f64::consts::PI * frequency * n as f64 / f64::from(rate)).sin()
                    as f32
                    * 0.5;
                [v, v]
            })
            .collect()
    }

    #[test]
    fn same_rate_passes_through() {
        let mut resampler = StereoResampler::new(48_000, 48_000).unwrap();
        let mut out = Vec::new();
        resampler.process(&[0.1, 0.2], &mut out).unwrap();
        resampler.finish(&mut out).unwrap();
        assert_eq!(out, vec![0.1, 0.2]);
        assert!(format!("{resampler:?}").contains("passthrough: true"));
    }

    #[test]
    fn converts_length_and_keeps_the_pitch() {
        let input = sine(44_100, 1_000.0, 44_100);
        let mut resampler = StereoResampler::new(44_100, 48_000).unwrap();
        let mut out = Vec::new();
        for block in input.chunks(700) {
            resampler.process(block, &mut out).unwrap();
        }
        resampler.finish(&mut out).unwrap();
        assert_eq!(out.len() / 2, 48_000);

        // Count rising zero crossings on the left channel in the middle part.
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        let crossings = left[12_000..36_000]
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        assert!((crossings as i64 - 500).abs() <= 1, "{crossings}");
        let peak = left[12_000..36_000]
            .iter()
            .fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.5).abs() < 0.02, "{peak}");
    }
}
