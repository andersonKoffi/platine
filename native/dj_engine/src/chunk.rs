//! Decoded audio travelling from a decoder thread to the audio thread.

use rtrb::{Consumer, Producer, PushError, RingBuffer};

/// Frames per chunk.
pub const CHUNK_FRAMES: usize = 512;
/// The engine works in stereo internally.
pub const ENGINE_CHANNELS: usize = 2;

/// A block of interleaved stereo samples at the engine sample rate.
///
/// `generation` identifies the load or seek that produced the chunk: the deck
/// ignores chunks from older generations, so a seek never plays stale audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioChunk {
    pub generation: u32,
    /// Track position of the first frame, in engine frames.
    pub start_frame: u64,
    /// Number of valid frames in `samples`.
    pub frames: u16,
    /// No more audio follows for this generation.
    pub end_of_stream: bool,
    pub samples: [f32; CHUNK_FRAMES * ENGINE_CHANNELS],
}

impl AudioChunk {
    pub const EMPTY: Self = Self {
        generation: 0,
        start_frame: 0,
        frames: 0,
        end_of_stream: false,
        samples: [0.0; CHUNK_FRAMES * ENGINE_CHANNELS],
    };

    /// A chunk that only marks the end of a stream.
    pub fn end_of_stream(generation: u32, start_frame: u64) -> Self {
        Self {
            generation,
            start_frame,
            end_of_stream: true,
            ..Self::EMPTY
        }
    }

    /// The `index`-th stereo frame.
    #[inline]
    pub fn frame(&self, index: usize) -> [f32; 2] {
        let base = index * ENGINE_CHANNELS;
        [self.samples[base], self.samples[base + 1]]
    }
}

/// Writing side of a deck's chunk ring, owned by the decoder thread.
#[derive(Debug)]
pub struct DeckFeed {
    producer: Producer<AudioChunk>,
}

impl DeckFeed {
    /// Free slots in the ring.
    pub fn free_slots(&self) -> usize {
        self.producer.slots()
    }

    /// Copies a chunk into the ring. Returns `false` if the ring is full.
    pub fn try_push(&mut self, chunk: &AudioChunk) -> bool {
        match self.producer.push(*chunk) {
            Ok(()) => true,
            Err(PushError::Full(_)) => false,
        }
    }
}

/// Creates the ring between a decoder (feed) and a deck voice.
pub(crate) fn chunk_ring(capacity: usize) -> (DeckFeed, Consumer<AudioChunk>) {
    let (producer, consumer) = RingBuffer::new(capacity.max(1));
    (DeckFeed { producer }, consumer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_reports_full_ring() {
        let (mut feed, mut consumer) = chunk_ring(1);
        assert_eq!(feed.free_slots(), 1);
        assert!(feed.try_push(&AudioChunk::EMPTY));
        assert!(!feed.try_push(&AudioChunk::end_of_stream(3, 9)));
        assert_eq!(consumer.pop().unwrap(), AudioChunk::EMPTY);
    }

    #[test]
    fn frames_are_read_interleaved() {
        let mut chunk = AudioChunk::EMPTY;
        chunk.samples[2] = 0.5;
        chunk.samples[3] = -0.5;
        assert_eq!(chunk.frame(1), [0.5, -0.5]);
        assert!(AudioChunk::end_of_stream(1, 0).end_of_stream);
    }
}
