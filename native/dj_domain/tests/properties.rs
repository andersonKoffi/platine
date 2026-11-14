//! Property-based tests of the domain invariants.
#![allow(clippy::unwrap_used)]

mod common;

use common::{XorShift, ids, s};
use dj_domain::{
    AutoMixInput, AutoMixScheduler, AutoMixSettings, AutoMixState, CrossfadeCurve,
    CrossfadeDuration, DeckId, FisherYates, PlayOrder, PlaybackQueue, RepeatMode, ShuffleStrategy,
    TrackId, TrackTiming, effective_fade,
};
use proptest::prelude::*;

fn library(len: u64) -> Vec<TrackId> {
    (1..=len).map(TrackId).collect()
}

proptest! {
    #[test]
    fn shuffle_is_a_permutation(len in 0u64..60, seed in any::<u64>()) {
        let mut tracks = library(len);
        FisherYates.arrange(&mut tracks, &[], &mut XorShift::new(seed));
        tracks.sort();
        prop_assert_eq!(tracks, library(len));
    }

    #[test]
    fn no_track_repeats_before_the_whole_folder_was_played(
        len in 1u64..30,
        cycles in 1usize..5,
        seed in any::<u64>(),
    ) {
        let mut rng = XorShift::new(seed);
        let mut queue = PlaybackQueue::new(library(len), PlayOrder::Shuffle, RepeatMode::All, &mut rng);
        let count = usize::try_from(len).unwrap();
        for _ in 0..cycles {
            let mut cycle: Vec<TrackId> = (0..count).filter_map(|_| queue.next(&mut rng)).collect();
            cycle.sort();
            prop_assert_eq!(cycle, library(len));
        }
    }

    #[test]
    fn new_cycle_never_starts_with_one_of_the_last_two_tracks(len in 3u64..30, seed in any::<u64>()) {
        let mut rng = XorShift::new(seed);
        let mut queue = PlaybackQueue::new(library(len), PlayOrder::Shuffle, RepeatMode::All, &mut rng);
        let count = usize::try_from(len).unwrap();
        let mut cycle: Vec<TrackId> = (0..count).filter_map(|_| queue.next(&mut rng)).collect();
        for _ in 0..3 {
            let first_of_next = queue.next(&mut rng).unwrap();
            prop_assert!(!cycle[count - 2..].contains(&first_of_next));
            cycle = std::iter::once(first_of_next)
                .chain((1..count).filter_map(|_| queue.next(&mut rng)))
                .collect();
        }
    }

    #[test]
    fn equal_power_keeps_total_power_at_one(progress in 0f32..=1.0) {
        let gains = CrossfadeCurve::EqualPower.gains(progress);
        let power = gains.outgoing.powi(2) + gains.incoming.powi(2);
        prop_assert!((power - 1.0).abs() < 1e-5);
    }

    #[test]
    fn every_curve_stays_within_unity(progress in -1f32..2.0) {
        for curve in [CrossfadeCurve::EqualPower, CrossfadeCurve::Linear, CrossfadeCurve::Cut] {
            let gains = curve.gains(progress);
            prop_assert!((0.0..=1.0).contains(&gains.outgoing));
            prop_assert!((0.0..=1.0).contains(&gains.incoming));
        }
    }

    #[test]
    fn effective_fade_never_exceeds_half_a_track(
        configured in 0f64..=30.0,
        outgoing in 0f64..600.0,
        incoming in 0f64..600.0,
    ) {
        let fade = effective_fade(
            CrossfadeDuration::new(configured).unwrap(),
            &TrackTiming::untrimmed(s(outgoing)),
            &TrackTiming::untrimmed(s(incoming)),
        );
        prop_assert!(fade.as_f64() <= configured);
        prop_assert!(fade.as_f64() <= outgoing / 2.0 + 1e-9);
        prop_assert!(fade.as_f64() <= incoming / 2.0 + 1e-9);
    }

    /// Random event sequences never break the scheduler's invariants.
    #[test]
    fn scheduler_invariants_hold_for_any_event_sequence(
        len in 0u64..6,
        seed in any::<u64>(),
        events in prop::collection::vec(event_strategy(), 0..60),
    ) {
        let mut mix = AutoMixScheduler::new(
            AutoMixSettings::default(),
            PlayOrder::Shuffle,
            RepeatMode::All,
            XorShift::new(seed),
        );
        mix.load_library(library(len));
        for event in events {
            let input = concretize(event, &mix);
            mix.handle(input);

            if let Some(plan) = mix.planned_transition() {
                prop_assert_eq!(mix.state(), AutoMixState::Playing { active: plan.from });
                prop_assert!(mix.deck(plan.from).ready_timing().is_some());
                prop_assert!(mix.deck(plan.to).ready_timing().is_some());
                prop_assert!(plan.start_at <= mix.deck(plan.from).ready_timing().unwrap().effective_end());
            }
            if let AutoMixState::Playing { active } = mix.state() {
                prop_assert!(!mix.deck(active).is_empty());
            }
        }
    }
}

/// Abstract event; the concrete track is picked from the deck state so that
/// most reports are relevant.
#[derive(Debug, Clone, Copy)]
enum Event {
    Loaded(DeckId, u32),
    Failed(DeckId),
    Started,
    Completed,
    Ended(DeckId),
    Manual,
    Resume,
    Enabled(bool),
    Duration(u8),
}

fn deck_strategy() -> impl Strategy<Value = DeckId> {
    prop_oneof![Just(DeckId::A), Just(DeckId::B)]
}

fn event_strategy() -> impl Strategy<Value = Event> {
    prop_oneof![
        3 => (deck_strategy(), 1u32..400).prop_map(|(deck, length)| Event::Loaded(deck, length)),
        1 => deck_strategy().prop_map(Event::Failed),
        2 => Just(Event::Started),
        2 => Just(Event::Completed),
        2 => deck_strategy().prop_map(Event::Ended),
        1 => Just(Event::Manual),
        1 => Just(Event::Resume),
        1 => any::<bool>().prop_map(Event::Enabled),
        1 => (0u8..=30).prop_map(Event::Duration),
    ]
}

fn concretize(event: Event, mix: &AutoMixScheduler<XorShift>) -> AutoMixInput {
    let track_on = |deck: DeckId| mix.deck(deck).track().unwrap_or(TrackId(0));
    let transition = mix
        .planned_transition()
        .map(|plan| (plan.from, plan.to))
        .or(match mix.state() {
            AutoMixState::Crossfading { from, to } => Some((from, to)),
            _ => None,
        })
        .unwrap_or((DeckId::A, DeckId::B));
    match event {
        Event::Loaded(deck, length) => AutoMixInput::TrackLoaded {
            deck,
            track: track_on(deck),
            timing: TrackTiming::untrimmed(s(f64::from(length))),
        },
        Event::Failed(deck) => AutoMixInput::TrackLoadFailed {
            deck,
            track: track_on(deck),
        },
        Event::Started => AutoMixInput::TransitionStarted {
            from: transition.0,
            to: transition.1,
        },
        Event::Completed => AutoMixInput::TransitionCompleted {
            from: transition.0,
            to: transition.1,
        },
        Event::Ended(deck) => AutoMixInput::TrackEnded { deck },
        Event::Manual => AutoMixInput::ManualIntervention,
        Event::Resume => AutoMixInput::Resume,
        Event::Enabled(on) => AutoMixInput::SetEnabled(on),
        Event::Duration(seconds) => {
            AutoMixInput::SetCrossfadeDuration(CrossfadeDuration::new(f64::from(seconds)).unwrap())
        }
    }
}

#[test]
fn helper_ids_are_consistent() {
    assert_eq!(ids(&[1, 2]), library(2));
}
