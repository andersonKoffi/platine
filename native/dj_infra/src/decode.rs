//! Track decoding with symphonia (MP3, FLAC, WAV, AIFF, AAC/M4A, ALAC, OGG Vorbis).

use std::fs::File;
use std::path::Path;

use dj_domain::Seconds;
use dj_domain::ports::{AudioStream, DecodeError, StreamSpec, TrackDecoder};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Time, TimeBase, Timestamp};

/// Consecutive undecodable packets after which a file is declared corrupt.
const MAX_CONSECUTIVE_ERRORS: u32 = 16;

/// [`TrackDecoder`] backed by symphonia.
#[derive(Debug, Clone, Copy, Default)]
pub struct SymphoniaDecoder;

impl TrackDecoder for SymphoniaDecoder {
    type Stream = SymphoniaStream;

    fn open(&self, path: &Path) -> Result<SymphoniaStream, DecodeError> {
        SymphoniaStream::open(path)
    }
}

/// A decoded stream of interleaved `f32` samples at the file's own rate.
pub struct SymphoniaStream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    time_base: Option<TimeBase>,
    spec: StreamSpec,
    duration: Option<Seconds>,
    pending: Vec<f32>,
    pending_pos: usize,
    skip_samples: usize,
    finished: bool,
}

impl std::fmt::Debug for SymphoniaStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymphoniaStream")
            .field("track_id", &self.track_id)
            .field("spec", &self.spec)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

impl SymphoniaStream {
    /// Opens `path` and decodes its first packet (to learn the format and to
    /// reject corrupt files early).
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, is not audio, or is corrupt.
    pub fn open(path: &Path) -> Result<Self, DecodeError> {
        let file = File::open(path).map_err(|err| DecodeError::Io(err.to_string()))?;
        let source = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(extension);
        }
        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                source,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|err| DecodeError::Unsupported(err.to_string()))?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| DecodeError::Unsupported("no audio track".into()))?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or_else(|| DecodeError::Unsupported("missing audio codec parameters".into()))?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(|err| DecodeError::Unsupported(err.to_string()))?;
        let track_id = track.id;
        let time_base = track.time_base;
        let duration = match (time_base, track.num_frames) {
            (Some(base), Some(frames)) => i64::try_from(frames)
                .ok()
                .and_then(|frames| base.calc_time(Timestamp::new(frames)))
                .and_then(|time| Seconds::new(time.as_secs_f64()).ok()),
            _ => None,
        };

        let mut stream = Self {
            format,
            decoder,
            track_id,
            time_base,
            spec: StreamSpec {
                sample_rate: 0,
                channels: 0,
            },
            duration,
            pending: Vec::new(),
            pending_pos: 0,
            skip_samples: 0,
            finished: false,
        };
        if !stream.decode_next()? {
            return Err(DecodeError::Corrupt("no audio data".into()));
        }
        Ok(stream)
    }

    /// Decodes packets until one yields audio. Returns `false` at the end of the file.
    fn decode_next(&mut self) -> Result<bool, DecodeError> {
        let mut errors = 0;
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    self.finished = true;
                    return Ok(false);
                }
                Err(SymphoniaError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    self.finished = true;
                    return Ok(false);
                }
                Err(err) => return Err(DecodeError::Corrupt(err.to_string())),
            };
            if packet.track_id != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(buffer) => {
                    if buffer.frames() == 0 {
                        continue;
                    }
                    let channels = u16::try_from(buffer.spec().channels().count()).unwrap_or(2);
                    self.spec = StreamSpec {
                        sample_rate: buffer.spec().rate(),
                        channels,
                    };
                    buffer.copy_to_vec_interleaved(&mut self.pending);
                    self.pending_pos = 0;
                    return Ok(true);
                }
                Err(SymphoniaError::DecodeError(message)) => {
                    errors += 1;
                    if errors >= MAX_CONSECUTIVE_ERRORS {
                        return Err(DecodeError::Corrupt(message.to_string()));
                    }
                }
                Err(err) => return Err(DecodeError::Corrupt(err.to_string())),
            }
        }
    }

    fn frames_between(&self, from: Timestamp, to: Timestamp) -> usize {
        let delta = to.get().saturating_sub(from.get()).max(0);
        let seconds = self
            .time_base
            .and_then(|base| base.calc_time(Timestamp::new(delta)))
            .map_or(0.0, |time| time.as_secs_f64());
        // Positive and bounded by the track length.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (seconds * f64::from(self.spec.sample_rate)).round() as usize;
        frames
    }
}

impl AudioStream for SymphoniaStream {
    fn spec(&self) -> StreamSpec {
        self.spec
    }

    fn duration(&self) -> Option<Seconds> {
        self.duration
    }

    fn read(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        let mut written = 0;
        while written < out.len() {
            if self.pending_pos >= self.pending.len() {
                if self.finished || !self.decode_next()? {
                    break;
                }
                continue;
            }
            let available = &self.pending[self.pending_pos..];
            if self.skip_samples > 0 {
                let skipped = self.skip_samples.min(available.len());
                self.skip_samples -= skipped;
                self.pending_pos += skipped;
                continue;
            }
            let count = available.len().min(out.len() - written);
            out[written..written + count].copy_from_slice(&available[..count]);
            self.pending_pos += count;
            written += count;
        }
        Ok(written)
    }

    fn seek(&mut self, position: Seconds) -> Result<(), DecodeError> {
        let time = Time::try_from_secs_f64(position.as_f64())
            .ok_or_else(|| DecodeError::Corrupt("invalid seek position".into()))?;
        let seeked = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|err| DecodeError::Corrupt(err.to_string()))?;
        self.decoder.reset();
        self.pending.clear();
        self.pending_pos = 0;
        self.finished = false;
        let frames = self.frames_between(seeked.actual_ts, seeked.required_ts);
        self.skip_samples = frames * usize::from(self.spec.channels.max(1));
        Ok(())
    }
}
