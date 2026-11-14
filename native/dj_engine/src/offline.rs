//! Offline output: renders into memory, for tests and analysis.

use dj_domain::ports::{AudioOutput, OutputError, RenderCallback, StreamSpec};

/// An [`AudioOutput`] that renders on demand instead of following a clock.
pub struct OfflineOutput {
    spec: StreamSpec,
    render: Option<RenderCallback>,
}

impl std::fmt::Debug for OfflineOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfflineOutput")
            .field("spec", &self.spec)
            .field("started", &self.render.is_some())
            .finish()
    }
}

impl OfflineOutput {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            spec: StreamSpec {
                sample_rate,
                channels,
            },
            render: None,
        }
    }

    /// Renders `frames` frames in blocks of `block_frames`, like a sound card would.
    /// Returns silence if the output was not started.
    pub fn render(&mut self, frames: usize, block_frames: usize) -> Vec<f32> {
        let channels = usize::from(self.spec.channels);
        let mut out = vec![0.0; frames * channels];
        if let Some(render) = self.render.as_mut() {
            for block in out.chunks_mut(block_frames.max(1) * channels) {
                render(block);
            }
        }
        out
    }
}

impl AudioOutput for OfflineOutput {
    fn spec(&self) -> StreamSpec {
        self.spec
    }

    fn start(&mut self, render: RenderCallback) -> Result<(), OutputError> {
        self.render = Some(render);
        Ok(())
    }

    fn stop(&mut self) {
        self.render = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_through_the_callback_in_blocks() {
        let mut output = OfflineOutput::new(48_000, 2);
        assert_eq!(output.spec().channels, 2);
        assert_eq!(output.render(4, 2), vec![0.0; 8]);
        let mut calls = 0;
        output
            .start(Box::new(move |block: &mut [f32]| {
                calls += 1;
                block.fill(calls as f32);
            }))
            .unwrap();
        assert_eq!(
            output.render(4, 2),
            vec![1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0]
        );
        assert!(format!("{output:?}").contains("started: true"));
        output.stop();
        assert_eq!(output.render(1, 1), vec![0.0, 0.0]);
    }
}
