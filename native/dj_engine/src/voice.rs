//! One deck inside the audio thread: reads chunks, applies transport, gain and EQ.

use dj_domain::DeckId;
use rtrb::Consumer;

use crate::chunk::AudioChunk;
use crate::dsp::{Isolator, Smoother};
use crate::protocol::EqBand;

/// Real-time state of a deck. Everything is preallocated.
// Independent state flags of the real-time voice; an enum would not be clearer.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub(crate) struct DeckVoice {
    id: DeckId,
    feed: Consumer<AudioChunk>,
    generation: u32,
    chunk: AudioChunk,
    has_chunk: bool,
    cursor: usize,
    position: u64,
    playing: bool,
    /// 0..1 envelope so that play, pause and seeks never click.
    transport: Smoother,
    /// Seek waiting for the fade-out to finish.
    pending_cue: Option<(u32, u64)>,
    gain: Smoother,
    eq_gains: [Smoother; 3],
    eq: Isolator,
    ended: bool,
    end_reported: bool,
    starved: bool,
    starvation_reported: bool,
    peak: [f32; 2],
}

impl DeckVoice {
    pub(crate) fn new(
        id: DeckId,
        feed: Consumer<AudioChunk>,
        sample_rate: u32,
        ramp_frames: u32,
    ) -> Self {
        Self {
            id,
            feed,
            generation: 0,
            chunk: AudioChunk::EMPTY,
            has_chunk: false,
            cursor: 0,
            position: 0,
            playing: false,
            transport: Smoother::new(0.0, ramp_frames),
            pending_cue: None,
            gain: Smoother::new(1.0, ramp_frames),
            eq_gains: [Smoother::new(1.0, ramp_frames); 3],
            eq: Isolator::new(sample_rate),
            // Nothing is loaded: behave as an ended, already reported track.
            ended: true,
            end_reported: true,
            starved: false,
            starvation_reported: false,
            peak: [0.0; 2],
        }
    }

    pub(crate) fn id(&self) -> DeckId {
        self.id
    }

    pub(crate) fn generation(&self) -> u32 {
        self.generation
    }

    pub(crate) fn position(&self) -> u64 {
        self.position
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(crate) fn has_ended(&self) -> bool {
        self.ended
    }

    // ----- commands --------------------------------------------------------

    pub(crate) fn cue(&mut self, generation: u32, position: u64) {
        if self.playing && self.transport.current() > 0.0 {
            self.pending_cue = Some((generation, position));
            self.transport.set_target(0.0);
        } else {
            self.apply_cue(generation, position);
        }
    }

    fn apply_cue(&mut self, generation: u32, position: u64) {
        self.generation = generation;
        self.position = position;
        self.has_chunk = false;
        self.cursor = 0;
        self.ended = false;
        self.end_reported = false;
        self.starved = false;
        self.starvation_reported = false;
        self.eq.reset();
    }

    pub(crate) fn play(&mut self) {
        self.playing = true;
        self.transport.set_target(1.0);
    }

    pub(crate) fn pause(&mut self) {
        self.transport.set_target(0.0);
    }

    pub(crate) fn eject(&mut self) {
        self.playing = false;
        self.pending_cue = None;
        self.transport.set_immediate(0.0);
        self.has_chunk = false;
        self.ended = true;
        self.end_reported = true;
        self.generation = self.generation.wrapping_add(1);
        self.eq.reset();
    }

    pub(crate) fn set_gain(&mut self, gain: f32) {
        self.gain.set_target(gain);
    }

    pub(crate) fn set_eq(&mut self, band: EqBand, gain: f32) {
        self.eq_gains[band.index()].set_target(gain);
    }

    // ----- rendering ---------------------------------------------------------

    /// Renders one frame (post gain and EQ, pre crossfader).
    #[inline]
    pub(crate) fn next_frame(&mut self) -> [f32; 2] {
        if !self.playing {
            return [0.0; 2];
        }
        let envelope = self.transport.next_value();
        if self.transport.is_settled() && envelope <= 0.0 {
            self.on_silent();
            return [0.0; 2];
        }
        let raw = self.read_raw();
        let gains = [
            self.eq_gains[0].next_value(),
            self.eq_gains[1].next_value(),
            self.eq_gains[2].next_value(),
        ];
        let filtered = self.eq.process(raw, gains);
        let level = self.gain.next_value() * envelope;
        let out = [filtered[0] * level, filtered[1] * level];
        self.peak[0] = self.peak[0].max(out[0].abs());
        self.peak[1] = self.peak[1].max(out[1].abs());
        out
    }

    /// The envelope reached zero: finish a pending seek or the pause.
    fn on_silent(&mut self) {
        if let Some((generation, position)) = self.pending_cue.take() {
            self.apply_cue(generation, position);
            self.transport.set_target(1.0);
        } else {
            self.playing = false;
        }
    }

    fn read_raw(&mut self) -> [f32; 2] {
        if self.ended {
            return [0.0; 2];
        }
        if (!self.has_chunk || self.cursor >= usize::from(self.chunk.frames)) && !self.next_chunk()
        {
            return [0.0; 2];
        }
        if self.cursor >= usize::from(self.chunk.frames) {
            return [0.0; 2]; // an end-of-stream marker without audio
        }
        let frame = self.chunk.frame(self.cursor);
        self.cursor += 1;
        self.position = self.chunk.start_frame + self.cursor as u64;
        frame
    }

    /// Moves to the next chunk of the current generation. Returns `false` if
    /// none is available (end of stream or starvation).
    fn next_chunk(&mut self) -> bool {
        if self.has_chunk && self.chunk.end_of_stream {
            self.ended = true;
            return false;
        }
        loop {
            let Ok(next) = self.feed.peek() else {
                self.starved = true;
                return false;
            };
            if next.generation < self.generation {
                let _ = self.feed.pop(); // stale audio from a previous load or seek
                continue;
            }
            if next.generation > self.generation {
                self.starved = true; // the matching cue command has not arrived yet
                return false;
            }
            if let Ok(chunk) = self.feed.pop() {
                self.chunk = chunk;
            }
            self.has_chunk = true;
            self.cursor = 0;
            self.position = self.chunk.start_frame;
            self.starved = false;
            self.starvation_reported = false;
            if self.chunk.frames == 0 && self.chunk.end_of_stream {
                self.ended = true;
                return false;
            }
            return true;
        }
    }

    // ----- reports -------------------------------------------------------------

    /// Returns the generation that just ended, once.
    pub(crate) fn take_end(&mut self) -> Option<u32> {
        if self.ended && !self.end_reported {
            self.end_reported = true;
            return Some(self.generation);
        }
        None
    }

    /// Returns `true` once when a playing deck starves.
    pub(crate) fn take_starvation(&mut self) -> bool {
        if self.starved && self.playing && !self.ended && !self.starvation_reported {
            self.starvation_reported = true;
            return true;
        }
        false
    }

    /// Peak since the last call, then resets it.
    pub(crate) fn take_peak(&mut self) -> [f32; 2] {
        std::mem::take(&mut self.peak)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{CHUNK_FRAMES, DeckFeed, chunk_ring};

    fn voice() -> (DeckVoice, DeckFeed) {
        let (feed, consumer) = chunk_ring(8);
        (DeckVoice::new(DeckId::A, consumer, 48_000, 4), feed)
    }

    fn chunk(generation: u32, start: u64, value: f32, eos: bool) -> AudioChunk {
        let mut chunk = AudioChunk {
            generation,
            start_frame: start,
            frames: CHUNK_FRAMES as u16,
            end_of_stream: eos,
            ..AudioChunk::EMPTY
        };
        chunk.samples.fill(value);
        chunk
    }

    #[test]
    fn idle_voice_is_silent_and_reports_nothing() {
        let (mut v, _) = voice();
        assert_eq!(v.next_frame(), [0.0; 2]);
        assert_eq!(v.take_end(), None);
        assert!(!v.take_starvation());
        assert_eq!(v.id(), DeckId::A);
    }

    #[test]
    fn plays_chunks_of_its_generation_and_skips_stale_ones() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&chunk(0, 0, 0.9, false)));
        assert!(feed.try_push(&chunk(1, 100, 0.0, false)));
        v.cue(1, 100);
        v.play();
        // Low frequencies pass the isolator: a DC level of 0 stays 0.
        for _ in 0..10 {
            v.next_frame();
        }
        assert_eq!(v.generation(), 1);
        assert_eq!(v.position(), 110);
        assert!(v.is_playing());
    }

    #[test]
    fn waits_for_chunks_of_a_newer_generation() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&chunk(2, 0, 0.5, false)));
        v.cue(1, 0);
        v.play();
        v.next_frame();
        assert!(v.take_starvation());
        assert!(!v.take_starvation(), "reported once");
        v.cue(2, 0);
        for _ in 0..20 {
            v.next_frame();
        }
        assert_eq!(v.generation(), 2);
        assert!(v.position() > 0);
    }

    #[test]
    fn reports_the_end_of_stream_once() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&chunk(1, 0, 0.1, true)));
        v.cue(1, 0);
        v.play();
        for _ in 0..=CHUNK_FRAMES {
            v.next_frame();
        }
        assert!(v.has_ended());
        assert_eq!(v.take_end(), Some(1));
        assert_eq!(v.take_end(), None);
    }

    #[test]
    fn empty_end_marker_ends_immediately() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&AudioChunk::end_of_stream(1, 0)));
        v.cue(1, 0);
        v.play();
        v.next_frame();
        assert_eq!(v.take_end(), Some(1));
    }

    #[test]
    fn pause_fades_out_then_stops() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&chunk(1, 0, 0.5, false)));
        v.cue(1, 0);
        v.play();
        for _ in 0..20 {
            v.next_frame();
        }
        v.pause();
        for _ in 0..10 {
            v.next_frame();
        }
        assert!(!v.is_playing());
        assert!(v.take_peak()[0] > 0.0);
        assert_eq!(v.take_peak(), [0.0; 2]);
    }

    #[test]
    fn seek_while_playing_fades_out_first() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&chunk(1, 0, 0.5, false)));
        assert!(feed.try_push(&chunk(2, 1000, 0.5, false)));
        v.cue(1, 0);
        v.play();
        for _ in 0..20 {
            v.next_frame();
        }
        v.cue(2, 1000);
        assert_eq!(v.generation(), 1, "old audio fades out first");
        for _ in 0..10 {
            v.next_frame();
        }
        assert_eq!(v.generation(), 2);
        assert!(v.position() >= 1000);
    }

    #[test]
    fn eject_silences_the_deck() {
        let (mut v, mut feed) = voice();
        assert!(feed.try_push(&chunk(1, 0, 0.5, false)));
        v.cue(1, 0);
        v.play();
        v.next_frame();
        v.eject();
        assert_eq!(v.next_frame(), [0.0; 2]);
        assert_eq!(v.take_end(), None);
    }

    #[test]
    fn gain_and_eq_targets_are_applied() {
        let (mut v, _) = voice();
        v.set_gain(0.5);
        v.set_eq(EqBand::High, 0.0);
        assert_eq!(v.gain.target(), 0.5);
        assert_eq!(v.eq_gains[2].target(), 0.0);
    }
}
