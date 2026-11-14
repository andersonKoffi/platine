//! The auto-mix state machine.

use super::{
    AutoMixAction, AutoMixInput, AutoMixSettings, AutoMixState, AutoMixStatus, TransitionPlan,
};
use crate::crossfade::effective_fade;
use crate::deck::{Deck, DeckLoad};
use crate::ports::RandomSource;
use crate::queue::{PlaybackQueue, RepeatMode};
use crate::shuffle::PlayOrder;
use crate::track::{TrackId, TrackTiming};
use crate::values::{CrossfaderPosition, DeckId};

type Actions = Vec<AutoMixAction>;

/// Drives two decks through a playback queue. See the module documentation.
#[derive(Debug)]
pub struct AutoMixScheduler<R: RandomSource> {
    queue: PlaybackQueue,
    rng: R,
    settings: AutoMixSettings,
    decks: [Deck; 2],
    state: AutoMixState,
    /// Deck that plays first when starting from `Idle`.
    lead: DeckId,
    suspended: bool,
    planned: Option<TransitionPlan>,
}

impl<R: RandomSource> AutoMixScheduler<R> {
    pub fn new(
        settings: AutoMixSettings,
        order: PlayOrder,
        repeat: RepeatMode,
        mut rng: R,
    ) -> Self {
        let queue = PlaybackQueue::new(Vec::new(), order, repeat, &mut rng);
        Self {
            queue,
            rng,
            settings,
            decks: [Deck::new(DeckId::A), Deck::new(DeckId::B)],
            state: AutoMixState::Finished,
            lead: DeckId::A,
            suspended: false,
            planned: None,
        }
    }

    // ----- queries -------------------------------------------------------

    pub fn state(&self) -> AutoMixState {
        self.state
    }

    pub fn status(&self) -> AutoMixStatus {
        match (self.settings.enabled, self.suspended) {
            (false, _) => AutoMixStatus::Off,
            (true, true) => AutoMixStatus::Suspended,
            (true, false) => AutoMixStatus::Running,
        }
    }

    pub fn settings(&self) -> AutoMixSettings {
        self.settings
    }

    pub fn queue(&self) -> &PlaybackQueue {
        &self.queue
    }

    pub fn deck(&self, id: DeckId) -> &Deck {
        &self.decks[id.index()]
    }

    pub fn planned_transition(&self) -> Option<TransitionPlan> {
        self.planned
    }

    // ----- library -------------------------------------------------------

    /// Replaces the queue with `tracks` (natural order). If nothing is
    /// playing, both decks are (re)loaded; otherwise the playing deck keeps
    /// its track and the standby deck takes the first track of the new queue.
    pub fn load_library(&mut self, tracks: Vec<TrackId>) -> Actions {
        let mut actions = Actions::new();
        self.queue.replace(tracks, &mut self.rng);
        match self.state {
            AutoMixState::Idle | AutoMixState::Finished => {
                self.cancel_plan(&mut actions);
                self.state = AutoMixState::Idle;
                self.lead = DeckId::A;
                for deck in DeckId::ALL {
                    self.load_next_into(deck, &mut actions);
                }
                self.after_idle_change(&mut actions);
            }
            AutoMixState::Playing { active } => {
                self.load_next_into(active.other(), &mut actions);
            }
            AutoMixState::WaitingForNext { deck } => {
                self.load_next_into(deck, &mut actions);
                if self.deck(deck).is_empty() {
                    self.finish(&mut actions);
                }
            }
            // The outgoing deck takes a track from the new queue when the fade ends.
            AutoMixState::Crossfading { .. } => {}
        }
        actions
    }

    /// Adds tracks to the queue ("add to queue" when a folder is dropped
    /// during playback). Empty decks are filled.
    pub fn append_to_library(&mut self, tracks: &[TrackId]) -> Actions {
        self.queue.append(tracks, &mut self.rng);
        let mut actions = Actions::new();
        self.refill(&mut actions);
        actions
    }

    /// Switches between sequential and shuffled order. Tracks already on the
    /// decks are untouched; only the waiting tracks are reordered.
    pub fn set_order(&mut self, order: PlayOrder) {
        self.queue.set_order(order, &mut self.rng);
    }

    pub fn set_repeat(&mut self, repeat: RepeatMode) -> Actions {
        self.queue.set_repeat(repeat);
        let mut actions = Actions::new();
        self.refill(&mut actions);
        actions
    }

    /// Moves a waiting track in the queue.
    ///
    /// # Errors
    /// Returns an error if an index is out of bounds.
    pub fn move_upcoming(&mut self, from: usize, to: usize) -> Result<(), crate::DomainError> {
        self.queue.move_upcoming(from, to)
    }

    // ----- events --------------------------------------------------------

    /// Feeds one event and returns the resulting commands, in order.
    pub fn handle(&mut self, input: AutoMixInput) -> Actions {
        let mut actions = Actions::new();
        match input {
            AutoMixInput::SetEnabled(enabled) => {
                let was_active = self.is_active();
                self.settings.enabled = enabled;
                self.on_activity_change(was_active, &mut actions);
            }
            AutoMixInput::ManualIntervention => {
                let was_active = self.is_active();
                self.suspended = true;
                self.on_activity_change(was_active, &mut actions);
            }
            AutoMixInput::Resume => {
                let was_active = self.is_active();
                self.suspended = false;
                self.on_activity_change(was_active, &mut actions);
            }
            AutoMixInput::SetCrossfadeDuration(duration) => {
                self.settings.crossfade = duration;
                self.replan(&mut actions);
            }
            AutoMixInput::SetCurve(curve) => {
                self.settings.curve = curve;
                self.replan(&mut actions);
            }
            AutoMixInput::TrackLoaded {
                deck,
                track,
                timing,
            } => self.on_track_loaded(deck, track, timing, &mut actions),
            AutoMixInput::TrackLoadFailed { deck, track } => {
                self.on_track_load_failed(deck, track, &mut actions);
            }
            AutoMixInput::TransitionStarted { from, to } => self.on_transition_started(from, to),
            AutoMixInput::TransitionCompleted { from, to } => {
                if self.state == (AutoMixState::Crossfading { from, to }) {
                    self.complete_transition(from, to, &mut actions);
                }
            }
            AutoMixInput::TrackEnded { deck } => self.on_track_ended(deck, &mut actions),
        }
        actions
    }

    // ----- event handlers ------------------------------------------------

    fn on_track_loaded(
        &mut self,
        deck: DeckId,
        track: TrackId,
        timing: TrackTiming,
        actions: &mut Actions,
    ) {
        if let DeckLoad::Loading(expected) = self.deck(deck).load()
            && expected != track
        {
            return; // stale report for a load that was superseded
        }
        // A deck reloaded by hand invalidates a plan computed with its old timing.
        if self
            .planned
            .is_some_and(|plan| plan.from == deck || plan.to == deck)
        {
            self.cancel_plan(actions);
        }
        self.decks[deck.index()].mark_ready(track, timing);
        match self.state {
            AutoMixState::Idle => self.maybe_start(actions),
            AutoMixState::WaitingForNext { deck: waiting }
                if waiting == deck && self.is_active() =>
            {
                self.start_playing(deck, actions);
            }
            _ => {}
        }
        self.try_plan(actions);
    }

    fn on_track_load_failed(&mut self, deck: DeckId, track: TrackId, actions: &mut Actions) {
        if self.deck(deck).load() != DeckLoad::Loading(track) {
            return;
        }
        self.queue.mark_unplayable(track);
        actions.push(AutoMixAction::SkippedUnplayable { track });
        self.load_next_into(deck, actions);
        match self.state {
            AutoMixState::WaitingForNext { deck: waiting }
                if waiting == deck && self.deck(deck).is_empty() =>
            {
                self.finish(actions);
            }
            AutoMixState::Idle => self.after_idle_change(actions),
            _ => {}
        }
    }

    fn on_transition_started(&mut self, from: DeckId, to: DeckId) {
        let expected = self
            .planned
            .is_some_and(|plan| plan.from == from && plan.to == to);
        if expected && self.state == (AutoMixState::Playing { active: from }) {
            self.planned = None;
            self.state = AutoMixState::Crossfading { from, to };
            self.lead = to;
        }
    }

    fn on_track_ended(&mut self, deck: DeckId, actions: &mut Actions) {
        match self.state {
            AutoMixState::Playing { active } if active == deck => {
                self.cancel_plan(actions);
                actions.push(AutoMixAction::Stop { deck });
                self.decks[deck.index()].clear();
                if self.is_active() {
                    self.continue_after_end(deck, actions);
                } else {
                    self.state = AutoMixState::Idle;
                    self.lead = deck.other();
                }
            }
            // A very short incoming track ended during the fade.
            AutoMixState::Crossfading { from, to } if to == deck => {
                self.complete_transition(from, to, actions);
                self.on_track_ended(deck, actions);
            }
            _ => {}
        }
    }

    /// The active deck ended without a transition: play the other deck as
    /// soon as it is ready.
    fn continue_after_end(&mut self, ended: DeckId, actions: &mut Actions) {
        let next = ended.other();
        if self.deck(next).is_empty() {
            self.load_next_into(next, actions);
        } else {
            self.load_next_into(ended, actions);
        }
        match self.deck(next).load() {
            DeckLoad::Ready { .. } => self.start_playing(next, actions),
            DeckLoad::Loading(_) => self.state = AutoMixState::WaitingForNext { deck: next },
            DeckLoad::Empty => self.finish(actions),
        }
    }

    fn complete_transition(&mut self, from: DeckId, to: DeckId, actions: &mut Actions) {
        actions.push(AutoMixAction::Stop { deck: from });
        self.decks[from.index()].clear();
        self.state = AutoMixState::Playing { active: to };
        self.lead = to;
        self.load_next_into(from, actions);
    }

    // ----- automation on/off ----------------------------------------------

    fn is_active(&self) -> bool {
        self.settings.enabled && !self.suspended
    }

    fn on_activity_change(&mut self, was_active: bool, actions: &mut Actions) {
        match (was_active, self.is_active()) {
            (true, false) => self.deactivate(actions),
            (false, true) => self.activate(actions),
            _ => {}
        }
    }

    fn deactivate(&mut self, actions: &mut Actions) {
        let crossfading = matches!(self.state, AutoMixState::Crossfading { .. });
        if self.planned.take().is_some() || crossfading {
            actions.push(AutoMixAction::CancelTransition);
        }
    }

    fn activate(&mut self, actions: &mut Actions) {
        match self.state {
            AutoMixState::Idle => self.maybe_start(actions),
            AutoMixState::Playing { active } => {
                actions.push(AutoMixAction::Play {
                    deck: active,
                    from: None,
                });
                actions.push(AutoMixAction::SetCrossfader(CrossfaderPosition::fully(
                    active,
                )));
                self.try_plan(actions);
            }
            AutoMixState::Crossfading { from, to } => {
                // Finish the interrupted fade at once on the incoming deck.
                self.complete_transition(from, to, actions);
                actions.push(AutoMixAction::SetCrossfader(CrossfaderPosition::fully(to)));
            }
            AutoMixState::WaitingForNext { deck } => {
                if self.deck(deck).ready_timing().is_some() {
                    self.start_playing(deck, actions);
                    self.try_plan(actions);
                }
            }
            AutoMixState::Finished => {}
        }
    }

    // ----- helpers ---------------------------------------------------------

    fn maybe_start(&mut self, actions: &mut Actions) {
        if self.state != AutoMixState::Idle || !self.is_active() {
            return;
        }
        let other = self.lead.other();
        if self.deck(self.lead).ready_timing().is_some() {
            self.start_playing(self.lead, actions);
        } else if self.deck(self.lead).is_empty() && self.deck(other).ready_timing().is_some() {
            self.start_playing(other, actions);
        }
        self.try_plan(actions);
    }

    /// After a change while idle: start if possible, or finish if both decks are empty.
    fn after_idle_change(&mut self, actions: &mut Actions) {
        if DeckId::ALL.iter().all(|deck| self.deck(*deck).is_empty()) {
            self.finish(actions);
        } else {
            self.maybe_start(actions);
        }
    }

    fn start_playing(&mut self, deck: DeckId, actions: &mut Actions) {
        let from = self.deck(deck).ready_timing().map(|t| t.effective_start());
        actions.push(AutoMixAction::Play { deck, from });
        actions.push(AutoMixAction::SetCrossfader(CrossfaderPosition::fully(
            deck,
        )));
        self.state = AutoMixState::Playing { active: deck };
        self.lead = deck;
        if self.deck(deck.other()).is_empty() {
            self.load_next_into(deck.other(), actions);
        }
    }

    fn try_plan(&mut self, actions: &mut Actions) {
        if !self.is_active() || self.planned.is_some() {
            return;
        }
        let AutoMixState::Playing { active } = self.state else {
            return;
        };
        let standby = active.other();
        let (Some(outgoing), Some(incoming)) = (
            self.deck(active).ready_timing(),
            self.deck(standby).ready_timing(),
        ) else {
            return;
        };
        let fade = effective_fade(self.settings.crossfade, &outgoing, &incoming);
        let plan = TransitionPlan {
            from: active,
            to: standby,
            start_at: outgoing.effective_end() - fade,
            fade,
            incoming_start: incoming.effective_start(),
            curve: self.settings.curve,
        };
        self.planned = Some(plan);
        actions.push(AutoMixAction::ScheduleTransition(plan));
    }

    /// Re-plans a scheduled (not yet started) transition after a settings change.
    fn replan(&mut self, actions: &mut Actions) {
        if self.planned.is_some() {
            self.cancel_plan(actions);
            self.try_plan(actions);
        }
    }

    fn cancel_plan(&mut self, actions: &mut Actions) {
        if self.planned.take().is_some() {
            actions.push(AutoMixAction::CancelTransition);
        }
    }

    /// Loads the next queued track into `deck`, or empties it if the queue is exhausted.
    fn load_next_into(&mut self, deck: DeckId, actions: &mut Actions) {
        if self
            .planned
            .is_some_and(|plan| plan.from == deck || plan.to == deck)
        {
            self.cancel_plan(actions);
        }
        match self.queue.next(&mut self.rng) {
            Some(track) => {
                self.decks[deck.index()].start_loading(track);
                actions.push(AutoMixAction::LoadTrack { deck, track });
            }
            None => self.decks[deck.index()].clear(),
        }
    }

    /// Fills empty decks after the queue grew (append, repeat turned on).
    fn refill(&mut self, actions: &mut Actions) {
        match self.state {
            AutoMixState::Finished => {
                if !self.queue.is_exhausted() {
                    self.state = AutoMixState::Idle;
                    self.lead = DeckId::A;
                    self.fill_empty_decks(actions);
                    self.maybe_start(actions);
                }
            }
            AutoMixState::Idle => {
                self.fill_empty_decks(actions);
                self.maybe_start(actions);
            }
            AutoMixState::Playing { active } => {
                if self.deck(active.other()).is_empty() {
                    self.load_next_into(active.other(), actions);
                }
            }
            AutoMixState::Crossfading { .. } | AutoMixState::WaitingForNext { .. } => {}
        }
    }

    fn fill_empty_decks(&mut self, actions: &mut Actions) {
        for deck in [self.lead, self.lead.other()] {
            if self.deck(deck).is_empty() {
                self.load_next_into(deck, actions);
            }
        }
    }

    fn finish(&mut self, actions: &mut Actions) {
        self.cancel_plan(actions);
        self.state = AutoMixState::Finished;
        actions.push(AutoMixAction::QueueExhausted);
    }
}
