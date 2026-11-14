//! Shared helpers for the integration tests of `dj_domain`.
#![allow(dead_code, unreachable_pub)]

use std::collections::{HashMap, HashSet};

use dj_domain::ports::Clock;
use dj_domain::{
    AutoMixAction, AutoMixInput, AutoMixScheduler, AutoMixSettings, AutoMixState, DeckId,
    PlayOrder, RepeatMode, Seconds, TrackId, TrackTiming, TransitionPlan,
};

pub use test_support::{ManualClock, SeededRandom as XorShift};

pub fn s(value: f64) -> Seconds {
    Seconds::saturating(value)
}

pub fn t(id: u64) -> TrackId {
    TrackId(id)
}

pub fn ids(values: &[u64]) -> Vec<TrackId> {
    values.iter().copied().map(TrackId).collect()
}

pub fn untrimmed(duration: f64) -> TrackTiming {
    TrackTiming::untrimmed(s(duration))
}

pub fn trimmed(duration: f64, leading: f64, trailing: f64) -> TrackTiming {
    TrackTiming::from_silences(s(duration), s(leading), s(trailing))
}

pub fn scheduler(settings: AutoMixSettings, repeat: RepeatMode) -> AutoMixScheduler<XorShift> {
    AutoMixScheduler::new(settings, PlayOrder::Sequential, repeat, XorShift::new(1))
}

pub fn loaded(deck: DeckId, track: u64, timing: TrackTiming) -> AutoMixInput {
    AutoMixInput::TrackLoaded {
        deck,
        track: t(track),
        timing,
    }
}

pub fn plan_of(actions: &[AutoMixAction]) -> Option<TransitionPlan> {
    actions.iter().find_map(|action| match action {
        AutoMixAction::ScheduleTransition(plan) => Some(*plan),
        _ => None,
    })
}

/// Event observed by the simulated engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimEvent {
    Started {
        deck: DeckId,
        track: TrackId,
        at: f64,
    },
    FadeStarted {
        from: DeckId,
        to: DeckId,
        at: f64,
        fade: f64,
    },
    Exhausted {
        at: f64,
    },
}

#[derive(Debug, Clone, Copy)]
struct PlayingDeck {
    track: TrackId,
    /// Track position at `since`.
    position: f64,
    since: f64,
}

/// A minimal engine driven by a [`ManualClock`]: it executes the scheduler's
/// commands with a load latency and reports events back at the right time.
pub struct SimEngine {
    pub clock: ManualClock,
    pub scheduler: AutoMixScheduler<XorShift>,
    pub events: Vec<SimEvent>,
    timings: HashMap<TrackId, TrackTiming>,
    corrupt: HashSet<TrackId>,
    load_latency: f64,
    pending_loads: Vec<(DeckId, TrackId, f64)>,
    loaded: [Option<TrackId>; 2],
    playing: [Option<PlayingDeck>; 2],
    plan: Option<TransitionPlan>,
    fade_end: Option<(DeckId, DeckId, f64)>,
}

impl SimEngine {
    pub fn new(
        settings: AutoMixSettings,
        order: PlayOrder,
        repeat: RepeatMode,
        timings: HashMap<TrackId, TrackTiming>,
    ) -> Self {
        Self {
            clock: ManualClock::default(),
            scheduler: AutoMixScheduler::new(settings, order, repeat, XorShift::new(99)),
            events: Vec::new(),
            timings,
            corrupt: HashSet::new(),
            load_latency: 0.5,
            pending_loads: Vec::new(),
            loaded: [None, None],
            playing: [None, None],
            plan: None,
            fade_end: None,
        }
    }

    pub fn with_corrupt(mut self, tracks: &[TrackId]) -> Self {
        self.corrupt.extend(tracks.iter().copied());
        self
    }

    fn now(&self) -> f64 {
        self.clock.now().as_f64()
    }

    pub fn load_library(&mut self, tracks: Vec<TrackId>) {
        let actions = self.scheduler.load_library(tracks);
        self.apply(&actions);
    }

    pub fn feed(&mut self, input: AutoMixInput) {
        let actions = self.scheduler.handle(input);
        self.apply(&actions);
    }

    fn position(&self, deck: DeckId) -> Option<f64> {
        self.playing[deck.index()].map(|p| p.position + (self.now() - p.since))
    }

    fn apply(&mut self, actions: &[AutoMixAction]) {
        let now = self.now();
        for action in actions {
            match *action {
                AutoMixAction::LoadTrack { deck, track } => {
                    self.pending_loads.retain(|(d, _, _)| *d != deck);
                    self.pending_loads
                        .push((deck, track, now + self.load_latency));
                }
                AutoMixAction::Play { deck, from } => {
                    let Some(track) = self.loaded[deck.index()] else {
                        continue;
                    };
                    let position = from
                        .map(Seconds::as_f64)
                        .or_else(|| self.position(deck))
                        .unwrap_or(0.0);
                    self.playing[deck.index()] = Some(PlayingDeck {
                        track,
                        position,
                        since: now,
                    });
                    self.events.push(SimEvent::Started {
                        deck,
                        track,
                        at: now,
                    });
                }
                AutoMixAction::Stop { deck } => self.playing[deck.index()] = None,
                AutoMixAction::ScheduleTransition(plan) => self.plan = Some(plan),
                AutoMixAction::CancelTransition => {
                    self.plan = None;
                    self.fade_end = None;
                }
                AutoMixAction::QueueExhausted => self.events.push(SimEvent::Exhausted { at: now }),
                AutoMixAction::SetCrossfader(_) | AutoMixAction::SkippedUnplayable { .. } => {}
            }
        }
    }

    /// Advances time by `dt` and delivers every event that became due.
    pub fn step(&mut self, dt: f64) {
        self.clock.advance(dt);
        let now = self.now();

        let due: Vec<_> = self
            .pending_loads
            .iter()
            .copied()
            .filter(|(_, _, at)| *at <= now)
            .collect();
        self.pending_loads.retain(|(_, _, at)| *at > now);
        for (deck, track, _) in due {
            if self.corrupt.contains(&track) {
                self.feed(AutoMixInput::TrackLoadFailed { deck, track });
            } else {
                self.loaded[deck.index()] = Some(track);
                let timing = self.timings[&track];
                self.feed(AutoMixInput::TrackLoaded {
                    deck,
                    track,
                    timing,
                });
            }
        }

        if let Some(plan) = self.plan
            && self
                .position(plan.from)
                .is_some_and(|p| p >= plan.start_at.as_f64())
        {
            self.plan = None;
            let track = self.loaded[plan.to.index()];
            if let Some(track) = track {
                let position = plan.incoming_start.as_f64();
                self.playing[plan.to.index()] = Some(PlayingDeck {
                    track,
                    position,
                    since: now,
                });
                self.events.push(SimEvent::Started {
                    deck: plan.to,
                    track,
                    at: now,
                });
            }
            self.events.push(SimEvent::FadeStarted {
                from: plan.from,
                to: plan.to,
                at: now,
                fade: plan.fade.as_f64(),
            });
            self.fade_end = Some((plan.from, plan.to, now + plan.fade.as_f64()));
            self.feed(AutoMixInput::TransitionStarted {
                from: plan.from,
                to: plan.to,
            });
        }

        if let Some((from, to, end)) = self.fade_end
            && now >= end
        {
            self.fade_end = None;
            self.feed(AutoMixInput::TransitionCompleted { from, to });
        }

        for deck in DeckId::ALL {
            let Some(playing) = self.playing[deck.index()] else {
                continue;
            };
            let end = self.timings[&playing.track].effective_end().as_f64();
            let in_fade = self.fade_end.is_some_and(|(from, _, _)| from == deck);
            if !in_fade && self.position(deck).is_some_and(|p| p >= end) {
                self.feed(AutoMixInput::TrackEnded { deck });
            }
        }
    }

    /// Runs until the queue is exhausted or `max_seconds` elapse.
    pub fn run(&mut self, max_seconds: f64, dt: f64) {
        while self.now() < max_seconds && self.scheduler.state() != AutoMixState::Finished {
            self.step(dt);
        }
    }

    pub fn started_tracks(&self) -> Vec<(DeckId, TrackId)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                SimEvent::Started { deck, track, .. } => Some((*deck, *track)),
                _ => None,
            })
            .collect()
    }

    pub fn fades(&self) -> Vec<(f64, f64)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                SimEvent::FadeStarted { at, fade, .. } => Some((*at, *fade)),
                _ => None,
            })
            .collect()
    }
}
