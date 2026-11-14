//! One decoder thread per deck: decodes, converts to stereo at the engine
//! rate and keeps the deck's chunk ring filled ahead of playback.

use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, unbounded};
use dj_domain::ports::{AudioStream, DecodeError, TrackDecoder};
use dj_domain::{DeckId, Seconds};
use dj_engine::{AudioChunk, CHUNK_FRAMES, DeckFeed, ENGINE_CHANNELS};

use crate::resample::StereoResampler;

/// How long the thread sleeps when the ring is full and nothing happens.
const IDLE_WAIT: Duration = Duration::from_millis(5);
/// Source frames read per decode step.
const READ_FRAMES: usize = 2048;

/// Requests sent to a deck's loader thread.
#[derive(Debug, Clone, PartialEq)]
pub enum LoaderRequest {
    /// Decodes `path` from `start`, tagging chunks with `generation`.
    Load {
        path: PathBuf,
        generation: u32,
        start: Seconds,
    },
    /// Restarts the current track at `position` under a new generation.
    Seek {
        generation: u32,
        position: Seconds,
    },
    Unload,
    Shutdown,
}

/// Facts about a track known once it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadedTrackInfo {
    /// Duration announced by the container, if any.
    pub duration: Option<Seconds>,
    pub source_rate: u32,
    pub source_channels: u16,
}

/// Reports from the loader threads.
#[derive(Debug, Clone, PartialEq)]
pub enum LoaderEvent {
    /// Enough audio is buffered to start playing.
    Ready {
        deck: DeckId,
        generation: u32,
        info: LoadedTrackInfo,
    },
    /// The file could not be opened or decoded; nothing was queued.
    Failed {
        deck: DeckId,
        generation: u32,
        error: DecodeError,
    },
    /// The whole track (from the requested position) is in the ring.
    Finished { deck: DeckId, generation: u32 },
}

/// Handle on a deck's loader thread. Dropping it stops the thread.
#[derive(Debug)]
pub struct DeckLoader {
    requests: Sender<LoaderRequest>,
    thread: Option<JoinHandle<()>>,
}

impl DeckLoader {
    /// Starts the loader thread of `deck`.
    ///
    /// # Errors
    /// Returns an error if the thread cannot be spawned.
    pub fn spawn<D>(
        deck: DeckId,
        decoder: D,
        feed: DeckFeed,
        engine_rate: u32,
        events: Sender<LoaderEvent>,
    ) -> std::io::Result<Self>
    where
        D: TrackDecoder + Send + 'static,
    {
        let (requests, inbox) = unbounded();
        let worker = Worker {
            deck,
            decoder,
            feed,
            engine_rate,
            events,
            inbox,
            job: None,
        };
        let thread = thread::Builder::new()
            .name(format!("platine-loader-{deck}"))
            .spawn(move || worker.run())?;
        Ok(Self {
            requests,
            thread: Some(thread),
        })
    }

    pub fn load(&self, path: PathBuf, generation: u32, start: Seconds) {
        self.send(LoaderRequest::Load {
            path,
            generation,
            start,
        });
    }

    pub fn seek(&self, generation: u32, position: Seconds) {
        self.send(LoaderRequest::Seek {
            generation,
            position,
        });
    }

    pub fn unload(&self) {
        self.send(LoaderRequest::Unload);
    }

    fn send(&self, request: LoaderRequest) {
        // The thread only stops on drop, so the channel stays open meanwhile.
        let _ = self.requests.send(request);
    }
}

impl Drop for DeckLoader {
    fn drop(&mut self) {
        let _ = self.requests.send(LoaderRequest::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A track being decoded.
struct Job<S> {
    stream: S,
    path: PathBuf,
    generation: u32,
    info: LoadedTrackInfo,
    resampler: StereoResampler,
    /// Next track position to emit, in engine frames.
    next_frame: u64,
    /// Resampled stereo waiting to be chunked.
    buffered: Vec<f32>,
    source_done: bool,
    ready_sent: bool,
    finished: bool,
    frames_pushed: u64,
    scratch: Vec<f32>,
    stereo: Vec<f32>,
}

struct Worker<D: TrackDecoder> {
    deck: DeckId,
    decoder: D,
    feed: DeckFeed,
    engine_rate: u32,
    events: Sender<LoaderEvent>,
    inbox: Receiver<LoaderRequest>,
    job: Option<Job<D::Stream>>,
}

impl<D: TrackDecoder> Worker<D> {
    fn run(mut self) {
        loop {
            match self.inbox.try_recv() {
                Ok(LoaderRequest::Shutdown) | Err(TryRecvError::Disconnected) => return,
                Ok(request) => {
                    self.handle(request);
                    continue;
                }
                Err(TryRecvError::Empty) => {}
            }
            if self.has_work() {
                self.step();
                continue;
            }
            match self.inbox.recv_timeout(IDLE_WAIT) {
                Ok(LoaderRequest::Shutdown) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(request) => self.handle(request),
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    fn has_work(&self) -> bool {
        self.job
            .as_ref()
            .is_some_and(|job| !job.finished && self.feed.free_slots() > 0)
    }

    fn handle(&mut self, request: LoaderRequest) {
        match request {
            LoaderRequest::Load {
                path,
                generation,
                start,
            } => self.open(path, generation, start),
            LoaderRequest::Seek {
                generation,
                position,
            } => {
                if let Some(path) = self.job.as_ref().map(|job| job.path.clone()) {
                    self.open(path, generation, position);
                }
            }
            LoaderRequest::Unload => self.job = None,
            LoaderRequest::Shutdown => {}
        }
    }

    fn open(&mut self, path: PathBuf, generation: u32, start: Seconds) {
        self.job = None;
        match self.start_job(path, generation, start) {
            Ok(job) => self.job = Some(job),
            Err(error) => self.emit(LoaderEvent::Failed {
                deck: self.deck,
                generation,
                error,
            }),
        }
    }

    fn start_job(
        &self,
        path: PathBuf,
        generation: u32,
        start: Seconds,
    ) -> Result<Job<D::Stream>, DecodeError> {
        let mut stream = self.decoder.open(&path)?;
        if start > Seconds::ZERO {
            stream.seek(start)?;
        }
        let spec = stream.spec();
        let resampler = StereoResampler::new(spec.sample_rate, self.engine_rate)?;
        // Seconds to frames: positive and bounded by the track length.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let next_frame = (start.as_f64() * f64::from(self.engine_rate)).round() as u64;
        Ok(Job {
            info: LoadedTrackInfo {
                duration: stream.duration(),
                source_rate: spec.sample_rate,
                source_channels: spec.channels,
            },
            stream,
            path,
            generation,
            resampler,
            next_frame,
            buffered: Vec::with_capacity(CHUNK_FRAMES * ENGINE_CHANNELS * 4),
            source_done: false,
            ready_sent: false,
            finished: false,
            frames_pushed: 0,
            scratch: vec![0.0; READ_FRAMES * usize::from(spec.channels.max(1))],
            stereo: Vec::with_capacity(READ_FRAMES * ENGINE_CHANNELS),
        })
    }

    /// Produces and pushes one chunk.
    fn step(&mut self) {
        let deck = self.deck;
        let prefill = u64::from(self.engine_rate) / 2;
        let Some(job) = self.job.as_mut() else { return };
        if let Err(error) = fill(job) {
            let generation = job.generation;
            if job.ready_sent {
                // Mid-track corruption: end the track where decoding stopped.
                job.source_done = true;
            } else {
                self.job = None;
                self.emit(LoaderEvent::Failed {
                    deck,
                    generation,
                    error,
                });
                return;
            }
        }
        let Some(job) = self.job.as_mut() else { return };
        let chunk = next_chunk(job);
        if !self.feed.try_push(&chunk) {
            return; // cannot happen: `has_work` checked for a free slot
        }
        job.next_frame += u64::from(chunk.frames);
        job.frames_pushed += u64::from(chunk.frames);
        let mut events = Vec::new();
        if !job.ready_sent && (job.frames_pushed >= prefill || chunk.end_of_stream) {
            job.ready_sent = true;
            events.push(LoaderEvent::Ready {
                deck,
                generation: job.generation,
                info: job.info,
            });
        }
        if chunk.end_of_stream {
            job.finished = true;
            events.push(LoaderEvent::Finished {
                deck,
                generation: job.generation,
            });
        }
        for event in events {
            self.emit(event);
        }
    }

    fn emit(&self, event: LoaderEvent) {
        let _ = self.events.send(event);
    }
}

/// Decodes until a full chunk is buffered or the source is exhausted.
fn fill<S: AudioStream>(job: &mut Job<S>) -> Result<(), DecodeError> {
    let needed = CHUNK_FRAMES * ENGINE_CHANNELS;
    let channels = usize::from(job.info.source_channels.max(1));
    while job.buffered.len() < needed && !job.source_done {
        let read = job.stream.read(&mut job.scratch)?;
        if read == 0 {
            job.source_done = true;
            job.resampler.finish(&mut job.buffered)?;
            break;
        }
        to_stereo(&job.scratch[..read], channels, &mut job.stereo);
        job.resampler.process(&job.stereo, &mut job.buffered)?;
    }
    Ok(())
}

/// Takes the next chunk out of the buffer.
fn next_chunk<S>(job: &mut Job<S>) -> AudioChunk {
    let available = job.buffered.len() / ENGINE_CHANNELS;
    let frames = available.min(CHUNK_FRAMES);
    let mut chunk = AudioChunk {
        generation: job.generation,
        start_frame: job.next_frame,
        frames: u16::try_from(frames).unwrap_or(0),
        end_of_stream: job.source_done && available <= CHUNK_FRAMES,
        ..AudioChunk::EMPTY
    };
    chunk.samples[..frames * ENGINE_CHANNELS]
        .copy_from_slice(&job.buffered[..frames * ENGINE_CHANNELS]);
    job.buffered.drain(..frames * ENGINE_CHANNELS);
    chunk
}

/// Converts interleaved audio with `channels` channels to stereo.
fn to_stereo(input: &[f32], channels: usize, out: &mut Vec<f32>) {
    out.clear();
    match channels {
        1 => out.extend(input.iter().flat_map(|sample| [*sample, *sample])),
        2 => out.extend_from_slice(input),
        _ => out.extend(
            input
                .chunks_exact(channels)
                .flat_map(|frame| [frame[0], frame[1]]),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::to_stereo;

    #[test]
    fn converts_any_layout_to_stereo() {
        let mut out = Vec::new();
        to_stereo(&[0.1, 0.2], 1, &mut out);
        assert_eq!(out, vec![0.1, 0.1, 0.2, 0.2]);
        to_stereo(&[0.1, 0.2], 2, &mut out);
        assert_eq!(out, vec![0.1, 0.2]);
        to_stereo(&[0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 3, &mut out);
        assert_eq!(out, vec![0.1, 0.2, 0.4, 0.5]);
    }
}
