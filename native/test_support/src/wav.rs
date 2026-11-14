//! Generated audio fixtures (16-bit PCM WAV), so that tests never need binary files in git.

use std::f64::consts::PI;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Description of a test tone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub sample_rate: u32,
    pub channels: u16,
    /// Hz; 0 writes silence.
    pub frequency: f64,
    pub seconds: f64,
    /// Linear peak amplitude, at most 1.
    pub amplitude: f64,
    /// Silence written before and after the tone, in seconds.
    pub leading_silence: f64,
    pub trailing_silence: f64,
}

impl Tone {
    /// A sine of `frequency` Hz lasting `seconds`, stereo, 44.1 kHz, amplitude 0.5.
    pub fn sine(frequency: f64, seconds: f64) -> Self {
        Self {
            sample_rate: 44_100,
            channels: 2,
            frequency,
            seconds,
            amplitude: 0.5,
            leading_silence: 0.0,
            trailing_silence: 0.0,
        }
    }

    #[must_use]
    pub fn at_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    #[must_use]
    pub fn mono(mut self) -> Self {
        self.channels = 1;
        self
    }

    #[must_use]
    pub fn with_silences(mut self, leading: f64, trailing: f64) -> Self {
        self.leading_silence = leading;
        self.trailing_silence = trailing;
        self
    }

    fn frames(&self, seconds: f64) -> u32 {
        // Fixture lengths are small and positive.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (seconds * f64::from(self.sample_rate)).round() as u32;
        frames
    }

    /// Writes the tone as a 16-bit PCM WAV file.
    ///
    /// # Errors
    /// Returns any I/O error.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let lead = self.frames(self.leading_silence);
        let body = self.frames(self.seconds);
        let tail = self.frames(self.trailing_silence);
        let frames = lead + body + tail;
        let channels = u32::from(self.channels);
        let data_len = frames * channels * 2;

        let mut out = BufWriter::new(File::create(path)?);
        out.write_all(b"RIFF")?;
        out.write_all(&(36 + data_len).to_le_bytes())?;
        out.write_all(b"WAVEfmt ")?;
        out.write_all(&16u32.to_le_bytes())?;
        out.write_all(&1u16.to_le_bytes())?; // PCM
        out.write_all(&self.channels.to_le_bytes())?;
        out.write_all(&self.sample_rate.to_le_bytes())?;
        out.write_all(&(self.sample_rate * channels * 2).to_le_bytes())?;
        out.write_all(&(u16::try_from(channels * 2).unwrap_or(4)).to_le_bytes())?;
        out.write_all(&16u16.to_le_bytes())?;
        out.write_all(b"data")?;
        out.write_all(&data_len.to_le_bytes())?;

        for frame in 0..frames {
            let in_body = frame >= lead && frame < lead + body;
            let value = if in_body && self.frequency > 0.0 {
                let n = f64::from(frame - lead);
                self.amplitude * (2.0 * PI * self.frequency * n / f64::from(self.sample_rate)).sin()
            } else {
                0.0
            };
            #[allow(clippy::cast_possible_truncation)]
            let sample = (value.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round() as i16;
            for _ in 0..channels {
                out.write_all(&sample.to_le_bytes())?;
            }
        }
        out.flush()
    }
}

/// Writes a file with a `.wav` name that is not audio at all.
///
/// # Errors
/// Returns any I/O error.
pub fn write_corrupt(path: &Path) -> io::Result<()> {
    std::fs::write(
        path,
        b"RIFF\x10\x00\x00\x00WAVEjunk that is definitely not audio",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_valid_header_and_the_expected_length() {
        let dir = std::env::temp_dir().join(format!("platine-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tone.wav");
        Tone::sine(440.0, 0.5)
            .at_rate(8_000)
            .mono()
            .with_silences(0.25, 0.25)
            .write(&path)
            .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 8_000 * 2);
        // The first sample is silent, a sample inside the tone is not.
        assert_eq!(&bytes[44..46], &[0, 0]);
        assert_ne!(&bytes[44 + 2 * 2_010..44 + 2 * 2_011], &[0, 0]);

        let corrupt = dir.join("bad.wav");
        write_corrupt(&corrupt).unwrap();
        assert!(std::fs::read(&corrupt).unwrap().len() < 100);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
