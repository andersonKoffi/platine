//! Helpers shared by the `dj_infra` integration tests.
#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use dj_infra::LoaderEvent;

/// A temporary directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "platine-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Waits (up to 10 s) for a loader event matching `predicate`.
pub fn wait_for(
    events: &Receiver<LoaderEvent>,
    mut predicate: impl FnMut(&LoaderEvent) -> bool,
) -> LoaderEvent {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = events
            .recv_timeout(remaining)
            .expect("timed out waiting for a loader event");
        if predicate(&event) {
            return event;
        }
    }
}

/// Amplitude of `frequency` in `samples` (I/Q demodulation).
pub fn tone_level(samples: &[f32], frequency: f64, rate: u32, first_frame: usize) -> f32 {
    let (mut i, mut q) = (0.0f64, 0.0f64);
    for (n, sample) in samples.iter().enumerate() {
        let phase =
            2.0 * std::f64::consts::PI * frequency * (first_frame + n) as f64 / f64::from(rate);
        i += f64::from(*sample) * phase.sin();
        q += f64::from(*sample) * phase.cos();
    }
    (2.0 * (i * i + q * q).sqrt() / samples.len() as f64) as f32
}

pub fn max_step(samples: &[f32]) -> f32 {
    samples
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max)
}

pub fn left(stereo: &[f32]) -> Vec<f32> {
    stereo.iter().step_by(2).copied().collect()
}
