//! Audio output through cpal (CoreAudio on macOS).
//!
//! A supervisor thread owns the cpal stream (streams are not `Send` on every
//! platform). Streams built on the default device follow route changes by
//! themselves; if the device disappears, the supervisor rebuilds the stream
//! on the new default device, reusing the same render callback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, ErrorKind, StreamConfig};
use crossbeam_channel::{Sender, bounded};
use dj_domain::ports::{AudioOutput, OutputError, RenderCallback, StreamSpec};

/// How often the supervisor checks the stream's health.
const HEALTH_CHECK: Duration = Duration::from_millis(250);
/// Delay before retrying when no device is available.
const RETRY_DELAY: Duration = Duration::from_millis(500);

/// [`AudioOutput`] on the system's default output device.
#[derive(Debug)]
pub struct CpalOutput {
    spec: StreamSpec,
    supervisor: Option<(Sender<()>, JoinHandle<()>)>,
}

impl CpalOutput {
    /// Uses the default device with its preferred sample rate and channel count.
    ///
    /// # Errors
    /// Returns [`OutputError::NoDevice`] if there is no output device.
    pub fn open_default() -> Result<Self, OutputError> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(OutputError::NoDevice)?;
        let config = device
            .default_output_config()
            .map_err(|err| OutputError::Device(err.to_string()))?;
        Ok(Self {
            spec: StreamSpec {
                sample_rate: config.sample_rate(),
                channels: config.channels(),
            },
            supervisor: None,
        })
    }
}

impl AudioOutput for CpalOutput {
    fn spec(&self) -> StreamSpec {
        self.spec
    }

    fn start(&mut self, render: RenderCallback) -> Result<(), OutputError> {
        self.stop();
        let render = Arc::new(Mutex::new(render));
        let spec = self.spec;
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let (ready_tx, ready_rx) = bounded::<Result<(), OutputError>>(1);
        let handle = thread::Builder::new()
            .name("platine-audio-output".into())
            .spawn(move || {
                let mut first = Some(ready_tx);
                loop {
                    let failed = Arc::new(AtomicBool::new(false));
                    match build_stream(spec, &render, &failed) {
                        Ok(stream) => {
                            if let Some(tx) = first.take() {
                                let _ = tx.send(Ok(()));
                            }
                            while !failed.load(Ordering::Relaxed) {
                                if stop_rx.recv_timeout(HEALTH_CHECK).is_ok() {
                                    drop(stream);
                                    return;
                                }
                            }
                            drop(stream);
                        }
                        Err(err) => {
                            if let Some(tx) = first.take() {
                                let _ = tx.send(Err(err));
                                return;
                            }
                        }
                    }
                    if stop_rx.recv_timeout(RETRY_DELAY).is_ok() {
                        return;
                    }
                }
            })
            .map_err(|err| OutputError::Device(err.to_string()))?;
        ready_rx
            .recv()
            .map_err(|_| OutputError::Device("audio thread stopped".into()))??;
        self.supervisor = Some((stop_tx, handle));
        Ok(())
    }

    fn stop(&mut self) {
        if let Some((stop, handle)) = self.supervisor.take() {
            let _ = stop.send(());
            let _ = handle.join();
        }
    }
}

impl Drop for CpalOutput {
    fn drop(&mut self) {
        self.stop();
    }
}

fn build_stream(
    spec: StreamSpec,
    render: &Arc<Mutex<RenderCallback>>,
    failed: &Arc<AtomicBool>,
) -> Result<cpal::Stream, OutputError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(OutputError::NoDevice)?;
    let config = StreamConfig {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        buffer_size: BufferSize::Default,
    };
    let render = Arc::clone(render);
    let failed = Arc::clone(failed);
    let stream = device
        .build_output_stream::<f32, _, _>(
            config,
            move |data: &mut [f32], _| {
                // Never contended: only the current stream uses the callback.
                if let Ok(mut render) = render.try_lock() {
                    render(data);
                } else {
                    data.fill(0.0);
                }
            },
            move |err| {
                if err.kind() != ErrorKind::DeviceChanged {
                    failed.store(true, Ordering::Relaxed);
                }
            },
            None,
        )
        .map_err(|err| OutputError::Device(err.to_string()))?;
    stream
        .play()
        .map_err(|err| OutputError::Device(err.to_string()))?;
    Ok(stream)
}
