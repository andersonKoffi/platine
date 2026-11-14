//! The audio graph: two decks, crossfader with automation, master, metering.

use dj_domain::{CrossfadeCurve, CrossfaderPosition, DeckId, Seconds};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::chunk::{CHUNK_FRAMES, DeckFeed, chunk_ring};
use crate::dsp::Smoother;
use crate::protocol::{EngineCommand, EngineEvent, EngineTransition, Levels};
use crate::voice::DeckVoice;

/// Sizing of the engine's queues and timings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    pub sample_rate: u32,
    /// Capacity of each deck's chunk ring.
    pub feed_capacity_chunks: usize,
    pub command_capacity: usize,
    pub event_capacity: usize,
    /// Frames between two position/level reports (~21 ms at 48 kHz).
    pub report_interval_frames: u32,
    /// Length of parameter ramps, in milliseconds.
    pub ramp_ms: f32,
}

impl EngineConfig {
    /// Look-ahead kept by the decoders, in seconds (ADR 0003: at least 2 s).
    pub const LOOK_AHEAD_SECONDS: u32 = 3;

    pub fn new(sample_rate: u32) -> Self {
        let frames = sample_rate as usize * Self::LOOK_AHEAD_SECONDS as usize;
        Self {
            sample_rate,
            feed_capacity_chunks: frames.div_ceil(CHUNK_FRAMES),
            command_capacity: 256,
            event_capacity: 1024,
            report_interval_frames: 1024,
            ramp_ms: 10.0,
        }
    }

    fn ramp_frames(&self) -> u32 {
        (self.ramp_ms / 1000.0 * self.sample_rate as f32)
            .round()
            .max(1.0) as u32
    }
}

/// Error returned when the command queue is full.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandQueueFull(pub EngineCommand);

impl std::fmt::Display for CommandQueueFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "engine command queue is full, dropped {:?}", self.0)
    }
}

impl std::error::Error for CommandQueueFull {}

/// Control-side handle: sends commands, receives events. Not real-time.
#[derive(Debug)]
pub struct EngineHandle {
    commands: Producer<EngineCommand>,
    events: Consumer<EngineEvent>,
    sample_rate: u32,
}

impl EngineHandle {
    /// # Errors
    /// Returns the command if the queue is full (the audio thread is stalled).
    pub fn send(&mut self, command: EngineCommand) -> Result<(), CommandQueueFull> {
        self.commands
            .push(command)
            .map_err(|rtrb::PushError::Full(command)| CommandQueueFull(command))
    }

    pub fn try_recv(&mut self) -> Option<EngineEvent> {
        self.events.pop().ok()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Converts a duration to engine frames.
    pub fn frames(&self, seconds: Seconds) -> u64 {
        (seconds.as_f64() * f64::from(self.sample_rate)).round() as u64
    }

    /// Converts engine frames to a duration.
    pub fn seconds(&self, frames: u64) -> Seconds {
        Seconds::saturating(frames as f64 / f64::from(self.sample_rate))
    }
}

#[derive(Debug, Clone, Copy)]
struct Automation {
    from: DeckId,
    to: DeckId,
    fade_frames: u64,
    elapsed: u64,
    curve: CrossfadeCurve,
}

impl Automation {
    fn progress(&self) -> f32 {
        if self.fade_frames == 0 {
            1.0
        } else {
            (self.elapsed as f64 / self.fade_frames as f64).min(1.0) as f32
        }
    }

    /// Equivalent manual crossfader position.
    fn position(&self) -> f32 {
        match self.from {
            DeckId::A => self.progress(),
            DeckId::B => 1.0 - self.progress(),
        }
    }
}

/// The real-time part. Call [`Engine::render`] from the audio callback.
#[derive(Debug)]
pub struct Engine {
    decks: [DeckVoice; 2],
    commands: Consumer<EngineCommand>,
    events: Producer<EngineEvent>,
    crossfader: Smoother,
    crossfader_curve: CrossfadeCurve,
    master: Smoother,
    pending: Option<EngineTransition>,
    automation: Option<Automation>,
    clock: u64,
    report_interval: u32,
    until_report: u32,
    master_peak: [f32; 2],
    /// Left and right summed to both channels (single-channel PA).
    mono: bool,
}

impl Engine {
    /// Builds an engine, its control handle and the feeds of decks A and B.
    pub fn new(config: EngineConfig) -> (Self, EngineHandle, [DeckFeed; 2]) {
        let ramp = config.ramp_frames();
        let (feed_a, consumer_a) = chunk_ring(config.feed_capacity_chunks);
        let (feed_b, consumer_b) = chunk_ring(config.feed_capacity_chunks);
        let (command_tx, command_rx) = RingBuffer::new(config.command_capacity.max(1));
        let (event_tx, event_rx) = RingBuffer::new(config.event_capacity.max(1));
        let engine = Self {
            decks: [
                DeckVoice::new(DeckId::A, consumer_a, config.sample_rate, ramp),
                DeckVoice::new(DeckId::B, consumer_b, config.sample_rate, ramp),
            ],
            commands: command_rx,
            events: event_tx,
            crossfader: Smoother::new(CrossfaderPosition::CENTER.value(), ramp),
            crossfader_curve: CrossfadeCurve::default(),
            master: Smoother::new(1.0, ramp),
            pending: None,
            automation: None,
            clock: 0,
            report_interval: config.report_interval_frames.max(1),
            until_report: config.report_interval_frames.max(1),
            master_peak: [0.0; 2],
            mono: false,
        };
        let handle = EngineHandle {
            commands: command_tx,
            events: event_rx,
            sample_rate: config.sample_rate,
        };
        (engine, handle, [feed_a, feed_b])
    }

    /// Frames rendered since creation.
    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Renders interleaved audio with `channels` channels into `out`.
    ///
    /// Real-time safe: no allocation, no lock, no I/O, no panic.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        self.drain_commands();
        if channels == 0 {
            return;
        }
        for deck in &mut self.decks {
            deck.discard_stale();
        }
        for frame in out.chunks_exact_mut(channels) {
            let mixed = self.next_frame();
            match frame {
                [mono] => *mono = f32::midpoint(mixed[0], mixed[1]),
                [left, right, rest @ ..] => {
                    *left = mixed[0];
                    *right = mixed[1];
                    rest.fill(0.0);
                }
                [] => {}
            }
        }
    }

    /// Wraps the engine into an output callback for `channels` channels.
    pub fn into_render_callback(mut self, channels: usize) -> dj_domain::ports::RenderCallback {
        Box::new(move |out: &mut [f32]| self.render(out, channels))
    }

    #[inline]
    fn next_frame(&mut self) -> [f32; 2] {
        self.start_pending_transition();
        let gains = self.deck_gains();
        let a = self.decks[0].next_frame();
        let b = self.decks[1].next_frame();
        let master = self.master.next_value();
        let mut mixed = [
            (a[0] * gains[0] + b[0] * gains[1]) * master,
            (a[1] * gains[0] + b[1] * gains[1]) * master,
        ];
        if self.mono {
            let sum = f32::midpoint(mixed[0], mixed[1]);
            mixed = [sum, sum];
        }
        self.master_peak[0] = self.master_peak[0].max(mixed[0].abs());
        self.master_peak[1] = self.master_peak[1].max(mixed[1].abs());
        self.advance_automation();
        self.clock += 1;
        self.report();
        mixed
    }

    fn drain_commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            self.apply(command);
        }
    }

    fn apply(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::Cue {
                deck,
                generation,
                position_frame,
            } => self.voice(deck).cue(generation, position_frame),
            EngineCommand::Play { deck } => self.voice(deck).play(),
            EngineCommand::Pause { deck } => self.voice(deck).pause(),
            EngineCommand::Eject { deck } => self.voice(deck).eject(),
            EngineCommand::SetDeckGain { deck, gain } => self.voice(deck).set_gain(gain.linear()),
            EngineCommand::SetEq { deck, band, gain } => {
                self.voice(deck).set_eq(band, gain.linear());
            }
            EngineCommand::SetPitch { deck, pitch } => self.voice(deck).set_rate(pitch.ratio()),
            EngineCommand::SetKeylock { deck, enabled } => self.voice(deck).set_keylock(enabled),
            EngineCommand::SetCrossfader(position) => {
                self.freeze_automation();
                self.crossfader.set_target(position.value());
            }
            EngineCommand::SetCrossfaderCurve(curve) => self.crossfader_curve = curve,
            EngineCommand::SetMasterGain(gain) => self.master.set_target(gain.linear()),
            EngineCommand::SetMonoOutput(mono) => self.mono = mono,
            EngineCommand::ScheduleTransition(transition) => self.pending = Some(transition),
            EngineCommand::CancelTransition => {
                self.pending = None;
                self.freeze_automation();
            }
        }
    }

    fn voice(&mut self, deck: DeckId) -> &mut DeckVoice {
        &mut self.decks[deck.index()]
    }

    fn start_pending_transition(&mut self) {
        let Some(plan) = self.pending else { return };
        let outgoing = &self.decks[plan.from.index()];
        if outgoing.position() < plan.start_at_frame && !outgoing.has_ended() {
            return;
        }
        self.pending = None;
        self.voice(plan.to).play();
        self.automation = Some(Automation {
            from: plan.from,
            to: plan.to,
            fade_frames: plan.fade_frames,
            elapsed: 0,
            curve: plan.curve,
        });
        self.emit(EngineEvent::TransitionStarted {
            from: plan.from,
            to: plan.to,
            at_frame: self.clock,
        });
    }

    /// Crossfader gains of decks A and B.
    #[inline]
    fn deck_gains(&mut self) -> [f32; 2] {
        if let Some(automation) = self.automation {
            let gains = automation.curve.gains(automation.progress());
            return match automation.from {
                DeckId::A => [gains.outgoing, gains.incoming],
                DeckId::B => [gains.incoming, gains.outgoing],
            };
        }
        let gains = self.crossfader_curve.gains(self.crossfader.next_value());
        [gains.outgoing, gains.incoming]
    }

    fn advance_automation(&mut self) {
        let Some(automation) = self.automation.as_mut() else {
            return;
        };
        automation.elapsed += 1;
        if automation.elapsed < automation.fade_frames {
            return;
        }
        let (from, to) = (automation.from, automation.to);
        self.automation = None;
        self.crossfader
            .set_immediate(CrossfaderPosition::fully(to).value());
        self.emit(EngineEvent::TransitionCompleted {
            from,
            to,
            at_frame: self.clock + 1,
        });
    }

    /// Leaves the crossfader where a running automation currently is.
    fn freeze_automation(&mut self) {
        if let Some(automation) = self.automation.take() {
            self.crossfader.set_immediate(automation.position());
            self.crossfader_curve = automation.curve;
        }
    }

    fn report(&mut self) {
        for index in 0..2 {
            let deck = self.decks[index].id();
            if let Some(generation) = self.decks[index].take_end() {
                self.emit(EngineEvent::TrackEnded { deck, generation });
            }
            if self.decks[index].take_starvation() {
                self.emit(EngineEvent::Underrun { deck });
            }
        }
        self.until_report -= 1;
        if self.until_report > 0 {
            return;
        }
        self.until_report = self.report_interval;
        for index in 0..2 {
            let voice = &self.decks[index];
            let event = EngineEvent::Position {
                deck: voice.id(),
                generation: voice.generation(),
                frame: voice.position(),
                playing: voice.is_playing(),
            };
            self.emit(event);
        }
        let levels = Levels {
            deck_a: self.decks[0].take_peak(),
            deck_b: self.decks[1].take_peak(),
            master: std::mem::take(&mut self.master_peak),
        };
        self.emit(EngineEvent::Levels(levels));
    }

    /// Emits an event; drops it if the control side does not keep up.
    fn emit(&mut self, event: EngineEvent) {
        let _ = self.events.push(event);
    }
}
