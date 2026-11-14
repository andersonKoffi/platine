//! Keylock: plays a deck faster or slower without changing its key.
//!
//! The source is consumed at `rate` source frames per output frame and
//! time-stretched back to the output length by Signalsmith Stretch (MIT), a
//! phase-vocoder designed for real-time use. Processing happens in blocks;
//! every buffer is allocated up front, so pulling frames never allocates on
//! the Rust side, and the library does not allocate while processing.

use signalsmith_stretch::Stretch;

/// Output frames produced per stretch call (~5 ms at 48 kHz).
pub const BLOCK_FRAMES: usize = 256;

/// Fastest supported speed: bounds the input buffer.
const MAX_RATE: f64 = 1.5;

const CHANNELS: usize = 2;

/// Time-stretcher for one stereo deck.
pub struct Keylock {
    stretch: Stretch,
    input: Vec<f32>,
    output: Vec<f32>,
    /// Frames of `output` already returned.
    read: usize,
    /// Source frames owed to the next block (fractional part of `rate`).
    carry: f64,
    rate: f64,
}

impl std::fmt::Debug for Keylock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keylock")
            .field("read", &self.read)
            .field("rate", &self.rate)
            .finish_non_exhaustive()
    }
}

impl Keylock {
    /// Allocates the stretcher and its buffers (not real-time safe).
    pub fn new(sample_rate: u32) -> Self {
        let mut stretch = Stretch::preset_cheaper(2, sample_rate.max(1));
        stretch.set_transpose_factor(1.0, None);
        // The largest input block, plus one frame for the fractional carry.
        // Block sizes are small positive numbers.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let max_input = (BLOCK_FRAMES as f64 * MAX_RATE).ceil() as usize + 1;
        Self {
            stretch,
            input: vec![0.0; max_input * CHANNELS],
            output: vec![0.0; BLOCK_FRAMES * CHANNELS],
            read: BLOCK_FRAMES,
            carry: 0.0,
            rate: 1.0,
        }
    }

    /// Forgets buffered audio (after a seek, a load or switching keylock on).
    pub fn reset(&mut self) {
        self.stretch.reset();
        self.read = BLOCK_FRAMES;
        self.carry = 0.0;
    }

    /// Next output frame, pulling source frames from `source` as needed.
    #[inline]
    pub fn next_frame(&mut self, rate: f32, source: &mut impl FnMut() -> [f32; 2]) -> [f32; 2] {
        if self.read >= BLOCK_FRAMES {
            self.refill(f64::from(rate), source);
        }
        let index = self.read * CHANNELS;
        self.read += 1;
        [self.output[index], self.output[index + 1]]
    }

    fn refill(&mut self, rate: f64, source: &mut impl FnMut() -> [f32; 2]) {
        self.rate = rate.clamp(1.0 / MAX_RATE, MAX_RATE);
        // Block sizes are small; `wanted` is positive.
        #[allow(clippy::cast_precision_loss)]
        let wanted = BLOCK_FRAMES as f64 * self.rate + self.carry;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (wanted.floor() as usize).min(self.input.len() / CHANNELS);
        // Small integers.
        #[allow(clippy::cast_precision_loss)]
        let consumed = frames as f64;
        self.carry = wanted - consumed;
        for frame in 0..frames {
            let [left, right] = source();
            self.input[frame * CHANNELS] = left;
            self.input[frame * CHANNELS + 1] = right;
        }
        self.stretch
            .process(&self.input[..frames * CHANNELS], &mut self.output[..]);
        self.read = 0;
    }

    /// Source frames between what was read and what is heard now.
    pub fn latency_frames(&self) -> u64 {
        let unread = BLOCK_FRAMES.saturating_sub(self.read) + self.stretch.output_latency();
        // Latencies are a few thousand frames.
        #[allow(clippy::cast_precision_loss)]
        let source = self.stretch.input_latency() as f64 + unread as f64 * self.rate;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = source.round() as u64;
        frames
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    /// A stereo sine source at `frequency` Hz (48 kHz).
    fn sine(frequency: f64) -> impl FnMut() -> [f32; 2] {
        let mut n = 0u64;
        move || {
            #[allow(clippy::cast_possible_truncation)]
            let value =
                (std::f64::consts::TAU * frequency * n as f64 / 48_000.0).sin() as f32 * 0.5;
            n += 1;
            [value, value]
        }
    }

    fn frequency_of(samples: &[f32]) -> f64 {
        let crossings = samples
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        crossings as f64 * 48_000.0 / samples.len() as f64
    }

    #[test]
    fn keeps_the_key_while_playing_faster() {
        let mut keylock = Keylock::new(48_000);
        let mut source = sine(440.0);
        let mut pulled = 0u64;
        let mut counting = || {
            pulled += 1;
            source()
        };
        let out: Vec<f32> = (0..48_000)
            .map(|_| keylock.next_frame(1.25, &mut counting)[0])
            .collect();
        // One second of output consumed 1.25 s of source...
        assert!((pulled as f64 - 60_000.0).abs() < 300.0, "{pulled}");
        // ...and still sounds at 440 Hz (skip the start-up latency).
        let heard = frequency_of(&out[24_000..]);
        assert!((heard - 440.0).abs() < 10.0, "heard {heard} Hz");
        assert!(keylock.latency_frames() > 0);
    }

    #[test]
    fn slower_speeds_consume_less_source() {
        let mut keylock = Keylock::new(48_000);
        let mut source = sine(220.0);
        let mut pulled = 0u64;
        let mut counting = || {
            pulled += 1;
            source()
        };
        let out: Vec<f32> = (0..24_000)
            .map(|_| keylock.next_frame(0.8, &mut counting)[0])
            .collect();
        assert!((pulled as f64 - 19_200.0).abs() < 300.0, "{pulled}");
        let heard = frequency_of(&out[12_000..]);
        assert!((heard - 220.0).abs() < 10.0, "heard {heard} Hz");
        keylock.reset();
        assert_eq!(keylock.read, BLOCK_FRAMES);
    }
}
