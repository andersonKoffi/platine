//! Behaviour of the auto-mix scheduler, event by event.

#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod common;

use common::{ids, loaded, plan_of, s, scheduler, t, trimmed, untrimmed};
use dj_domain::{
    AutoMixAction as A, AutoMixInput as In, AutoMixSettings, AutoMixState, AutoMixStatus,
    CrossfadeCurve, CrossfadeDuration, CrossfaderPosition, DeckId, PlayOrder, RepeatMode,
};

use DeckId::{A as DA, B as DB};

/// No command expected.
const NONE: Vec<A> = Vec::new();

fn settings() -> AutoMixSettings {
    AutoMixSettings::default()
}

fn fade(seconds: f64) -> CrossfadeDuration {
    CrossfadeDuration::new(seconds).unwrap()
}

/// Library of tracks 1..=n, both first decks loaded with untrimmed 180 s tracks.
fn playing_a(n: u64) -> dj_domain::AutoMixScheduler<common::XorShift> {
    let mut mix = scheduler(settings(), RepeatMode::Off);
    mix.load_library((1..=n).map(t).collect());
    mix.handle(loaded(DA, 1, untrimmed(180.0)));
    if n > 1 {
        mix.handle(loaded(DB, 2, untrimmed(200.0)));
    }
    mix
}

mod loading {
    use super::*;

    #[test]
    fn empty_folder_finishes_immediately() {
        let mut mix = scheduler(settings(), RepeatMode::All);
        assert_eq!(mix.load_library(Vec::new()), vec![A::QueueExhausted]);
        assert_eq!(mix.state(), AutoMixState::Finished);
    }

    #[test]
    fn deck_a_loads_track_one_and_deck_b_track_two() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        let actions = mix.load_library(ids(&[1, 2, 3]));
        assert_eq!(
            actions,
            vec![
                A::LoadTrack {
                    deck: DA,
                    track: t(1)
                },
                A::LoadTrack {
                    deck: DB,
                    track: t(2)
                },
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Idle);
        assert_eq!(mix.deck(DA).track(), Some(t(1)));
        assert_eq!(mix.queue().upcoming().collect::<Vec<_>>(), ids(&[3]));
    }

    #[test]
    fn deck_a_starts_after_its_leading_silence() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        let actions = mix.handle(loaded(DA, 1, trimmed(180.0, 1.5, 0.0)));
        assert_eq!(
            actions,
            vec![
                A::Play {
                    deck: DA,
                    from: Some(s(1.5))
                },
                A::SetCrossfader(CrossfaderPosition::fully(DA)),
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DA });
    }

    #[test]
    fn deck_b_ready_first_waits_for_deck_a() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        assert_eq!(mix.handle(loaded(DB, 2, untrimmed(200.0))), NONE);
        let actions = mix.handle(loaded(DA, 1, untrimmed(180.0)));
        assert!(matches!(actions[0], A::Play { deck: DA, .. }));
        assert!(plan_of(&actions).is_some());
    }

    #[test]
    fn nothing_plays_while_auto_mix_is_off() {
        let off = AutoMixSettings {
            enabled: false,
            ..settings()
        };
        let mut mix = scheduler(off, RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        assert_eq!(mix.handle(loaded(DA, 1, untrimmed(180.0))), NONE);
        assert_eq!(mix.handle(loaded(DB, 2, untrimmed(180.0))), NONE);
        assert_eq!(mix.status(), AutoMixStatus::Off);

        let actions = mix.handle(In::SetEnabled(true));
        assert!(matches!(actions[0], A::Play { deck: DA, .. }));
        assert!(plan_of(&actions).is_some());
        assert_eq!(mix.status(), AutoMixStatus::Running);
    }

    #[test]
    fn stale_load_reports_are_ignored() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        assert_eq!(mix.handle(loaded(DA, 9, untrimmed(180.0))), NONE);
        assert!(mix.deck(DA).is_loading());
    }

    #[test]
    fn single_track_without_repeat_leaves_deck_b_empty_then_stops() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        assert_eq!(
            mix.load_library(ids(&[1])),
            vec![A::LoadTrack {
                deck: DA,
                track: t(1)
            }]
        );
        mix.handle(loaded(DA, 1, untrimmed(60.0)));
        assert!(mix.deck(DB).is_empty());
        assert_eq!(
            mix.handle(In::TrackEnded { deck: DA }),
            vec![A::Stop { deck: DA }, A::QueueExhausted]
        );
        assert_eq!(mix.state(), AutoMixState::Finished);
    }

    #[test]
    fn single_track_with_repeat_loops_through_deck_b() {
        let mut mix = scheduler(settings(), RepeatMode::All);
        let actions = mix.load_library(ids(&[1]));
        assert_eq!(
            actions,
            vec![
                A::LoadTrack {
                    deck: DA,
                    track: t(1)
                },
                A::LoadTrack {
                    deck: DB,
                    track: t(1)
                },
            ]
        );
    }
}

mod transitions {
    use super::*;

    #[test]
    fn schedules_the_fade_at_effective_end_minus_duration() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        mix.handle(loaded(DA, 1, trimmed(180.0, 0.0, 4.0)));
        let plan = plan_of(&mix.handle(loaded(DB, 2, trimmed(200.0, 2.0, 0.0)))).unwrap();
        assert_eq!(plan.from, DA);
        assert_eq!(plan.to, DB);
        assert_eq!(plan.start_at, s(168.0)); // 180 - 4 (silence) - 8 (fade)
        assert_eq!(plan.fade, s(8.0));
        assert_eq!(plan.incoming_start, s(2.0));
        assert_eq!(plan.curve, CrossfadeCurve::EqualPower);
        assert_eq!(mix.planned_transition(), Some(plan));
    }

    #[test]
    fn short_tracks_get_a_fade_of_half_their_length() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        mix.handle(loaded(DA, 1, untrimmed(10.0)));
        let plan = plan_of(&mix.handle(loaded(DB, 2, untrimmed(200.0)))).unwrap();
        assert_eq!(plan.fade, s(5.0));
        assert_eq!(plan.start_at, s(5.0));
    }

    #[test]
    fn zero_fade_switches_at_effective_end() {
        let instant = AutoMixSettings {
            crossfade: fade(0.0),
            ..settings()
        };
        let mut mix = scheduler(instant, RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        mix.handle(loaded(DA, 1, trimmed(100.0, 0.0, 3.0)));
        let plan = plan_of(&mix.handle(loaded(DB, 2, untrimmed(100.0)))).unwrap();
        assert_eq!(plan.fade, s(0.0));
        assert_eq!(plan.start_at, s(97.0));
    }

    #[test]
    fn completed_transition_swaps_roles_and_loads_the_next_track() {
        let mut mix = playing_a(3);
        assert_eq!(mix.handle(In::TransitionStarted { from: DA, to: DB }), NONE);
        assert_eq!(mix.state(), AutoMixState::Crossfading { from: DA, to: DB });
        assert_eq!(mix.planned_transition(), None);

        let actions = mix.handle(In::TransitionCompleted { from: DA, to: DB });
        assert_eq!(
            actions,
            vec![
                A::Stop { deck: DA },
                A::LoadTrack {
                    deck: DA,
                    track: t(3)
                }
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DB });

        let plan = plan_of(&mix.handle(loaded(DA, 3, untrimmed(120.0)))).unwrap();
        assert_eq!((plan.from, plan.to), (DB, DA));
        assert_eq!(plan.start_at, s(192.0));
    }

    #[test]
    fn unexpected_transition_reports_are_ignored() {
        let mut mix = playing_a(3);
        assert_eq!(mix.handle(In::TransitionStarted { from: DB, to: DA }), NONE);
        assert_eq!(
            mix.handle(In::TransitionCompleted { from: DA, to: DB }),
            NONE
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DA });
    }

    #[test]
    fn end_of_queue_stops_after_the_last_track() {
        let mut mix = playing_a(2);
        mix.handle(In::TransitionStarted { from: DA, to: DB });
        let actions = mix.handle(In::TransitionCompleted { from: DA, to: DB });
        assert_eq!(actions, vec![A::Stop { deck: DA }]);
        assert!(mix.deck(DA).is_empty());
        assert_eq!(
            mix.handle(In::TrackEnded { deck: DB }),
            vec![A::Stop { deck: DB }, A::QueueExhausted]
        );
        assert_eq!(mix.state(), AutoMixState::Finished);
    }

    #[test]
    fn track_end_before_next_is_ready_waits_then_plays_it() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2, 3]));
        mix.handle(loaded(DA, 1, untrimmed(60.0)));
        let actions = mix.handle(In::TrackEnded { deck: DA });
        assert_eq!(
            actions,
            vec![
                A::Stop { deck: DA },
                A::LoadTrack {
                    deck: DA,
                    track: t(3)
                }
            ]
        );
        assert_eq!(mix.state(), AutoMixState::WaitingForNext { deck: DB });

        let actions = mix.handle(loaded(DB, 2, untrimmed(60.0)));
        assert_eq!(
            actions[0],
            A::Play {
                deck: DB,
                from: Some(s(0.0))
            }
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DB });
    }

    #[test]
    fn incoming_track_ending_during_the_fade_moves_on() {
        let mut mix = playing_a(3);
        mix.handle(In::TransitionStarted { from: DA, to: DB });
        let actions = mix.handle(In::TrackEnded { deck: DB });
        assert_eq!(actions[0], A::Stop { deck: DA });
        assert!(actions.contains(&A::Stop { deck: DB }));
        assert_eq!(mix.state(), AutoMixState::WaitingForNext { deck: DA });
    }

    #[test]
    fn end_reports_for_idle_decks_are_ignored() {
        let mut mix = playing_a(3);
        assert_eq!(mix.handle(In::TrackEnded { deck: DB }), NONE);
    }
}

mod failures {
    use super::*;

    #[test]
    fn decode_error_on_standby_skips_to_the_following_track() {
        let mut mix = scheduler(settings(), RepeatMode::All);
        mix.load_library(ids(&[1, 2, 3]));
        mix.handle(loaded(DA, 1, untrimmed(180.0)));
        let actions = mix.handle(In::TrackLoadFailed {
            deck: DB,
            track: t(2),
        });
        assert_eq!(
            actions,
            vec![
                A::SkippedUnplayable { track: t(2) },
                A::LoadTrack {
                    deck: DB,
                    track: t(3)
                },
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DA });
        assert!(!mix.queue().upcoming().any(|track| track == t(2)));
    }

    #[test]
    fn decode_error_on_the_first_deck_lets_the_other_deck_start() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        let actions = mix.handle(In::TrackLoadFailed {
            deck: DA,
            track: t(1),
        });
        assert_eq!(actions, vec![A::SkippedUnplayable { track: t(1) }]);
        let actions = mix.handle(loaded(DB, 2, untrimmed(100.0)));
        assert_eq!(
            actions[0],
            A::Play {
                deck: DB,
                from: Some(s(0.0))
            }
        );
    }

    #[test]
    fn every_track_failing_finishes_the_queue() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        mix.handle(In::TrackLoadFailed {
            deck: DA,
            track: t(1),
        });
        let actions = mix.handle(In::TrackLoadFailed {
            deck: DB,
            track: t(2),
        });
        assert_eq!(
            actions,
            vec![A::SkippedUnplayable { track: t(2) }, A::QueueExhausted]
        );
        assert_eq!(mix.state(), AutoMixState::Finished);
    }

    #[test]
    fn failure_while_waiting_for_the_last_track_finishes() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        mix.handle(loaded(DA, 1, untrimmed(60.0)));
        mix.handle(In::TrackEnded { deck: DA });
        let actions = mix.handle(In::TrackLoadFailed {
            deck: DB,
            track: t(2),
        });
        assert_eq!(
            actions,
            vec![A::SkippedUnplayable { track: t(2) }, A::QueueExhausted]
        );
    }

    #[test]
    fn stale_failure_reports_are_ignored() {
        let mut mix = playing_a(3);
        assert_eq!(
            mix.handle(In::TrackLoadFailed {
                deck: DB,
                track: t(2)
            }),
            NONE
        );
    }
}

mod manual_control {
    use super::*;

    #[test]
    fn manual_intervention_suspends_and_cancels_the_plan() {
        let mut mix = playing_a(3);
        assert_eq!(
            mix.handle(In::ManualIntervention),
            vec![A::CancelTransition]
        );
        assert_eq!(mix.status(), AutoMixStatus::Suspended);
        assert_eq!(mix.planned_transition(), None);
        assert_eq!(mix.handle(In::ManualIntervention), NONE);
    }

    #[test]
    fn resume_restores_playback_and_reschedules() {
        let mut mix = playing_a(3);
        mix.handle(In::ManualIntervention);
        let actions = mix.handle(In::Resume);
        assert_eq!(
            actions[0],
            A::Play {
                deck: DA,
                from: None
            }
        );
        assert_eq!(actions[1], A::SetCrossfader(CrossfaderPosition::fully(DA)));
        assert!(plan_of(&actions).is_some());
        assert_eq!(mix.status(), AutoMixStatus::Running);
        assert_eq!(mix.handle(In::Resume), NONE);
    }

    #[test]
    fn intervention_during_a_fade_freezes_it_and_resume_finishes_it() {
        let mut mix = playing_a(3);
        mix.handle(In::TransitionStarted { from: DA, to: DB });
        assert_eq!(
            mix.handle(In::ManualIntervention),
            vec![A::CancelTransition]
        );
        let actions = mix.handle(In::Resume);
        assert_eq!(
            actions,
            vec![
                A::Stop { deck: DA },
                A::LoadTrack {
                    deck: DA,
                    track: t(3)
                },
                A::SetCrossfader(CrossfaderPosition::fully(DB)),
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DB });
    }

    #[test]
    fn track_end_while_suspended_resumes_instead_of_leaving_silence() {
        let mut mix = playing_a(3);
        mix.handle(In::ManualIntervention);
        let actions = mix.handle(In::TrackEnded { deck: DA });
        assert_eq!(actions[0], A::Stop { deck: DA });
        assert!(actions.contains(&A::Play {
            deck: DB,
            from: Some(s(0.0))
        }));
        assert_eq!(mix.status(), AutoMixStatus::Running);
        assert_eq!(mix.state(), AutoMixState::Playing { active: DB });
    }

    #[test]
    fn track_end_with_automix_off_stops() {
        let mut mix = playing_a(3);
        mix.handle(In::SetEnabled(false));
        assert_eq!(
            mix.handle(In::TrackEnded { deck: DA }),
            vec![A::Stop { deck: DA }]
        );
        assert_eq!(mix.state(), AutoMixState::Idle);
    }

    #[test]
    fn manual_load_on_standby_becomes_the_next_track() {
        let mut mix = playing_a(4);
        let actions = mix.handle(In::ManualLoad {
            deck: DB,
            track: t(4),
        });
        assert_eq!(
            actions,
            vec![
                A::CancelTransition,
                A::LoadTrack {
                    deck: DB,
                    track: t(4)
                }
            ]
        );
        assert_eq!(mix.status(), AutoMixStatus::Running);
        // The displaced track 2 comes back first; track 4 left the queue.
        assert_eq!(mix.queue().upcoming().collect::<Vec<_>>(), vec![t(2), t(3)]);
        let plan = plan_of(&mix.handle(loaded(DB, 4, untrimmed(60.0)))).unwrap();
        assert_eq!(plan.to, DB);
    }

    #[test]
    fn manual_load_on_air_suspends() {
        let mut mix = playing_a(3);
        let actions = mix.handle(In::ManualLoad {
            deck: DA,
            track: t(3),
        });
        assert_eq!(actions[0], A::CancelTransition);
        assert_eq!(mix.status(), AutoMixStatus::Suspended);
    }

    #[test]
    fn manual_load_while_suspended_is_accepted() {
        let mut mix = playing_a(3);
        mix.handle(In::ManualIntervention);
        assert_eq!(mix.handle(loaded(DB, 42, untrimmed(90.0))), NONE);
        assert_eq!(mix.deck(DB).track(), Some(t(42)));
        let plan = plan_of(&mix.handle(In::Resume)).unwrap();
        assert_eq!(plan.to, DB);
    }

    #[test]
    fn reloading_a_deck_replans_with_its_new_timing() {
        let mut mix = playing_a(3);
        let actions = mix.handle(loaded(DB, 2, untrimmed(10.0)));
        assert_eq!(actions[0], A::CancelTransition);
        assert_eq!(plan_of(&actions).unwrap().fade, s(5.0));
    }

    #[test]
    fn resume_while_waiting_starts_the_ready_deck() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2, 3]));
        mix.handle(loaded(DA, 1, untrimmed(60.0)));
        mix.handle(In::TrackEnded { deck: DA });
        mix.handle(In::ManualIntervention);
        assert_eq!(mix.handle(loaded(DB, 2, untrimmed(60.0))), NONE);
        let actions = mix.handle(In::Resume);
        assert_eq!(
            actions[0],
            A::Play {
                deck: DB,
                from: Some(s(0.0))
            }
        );
    }

    #[test]
    fn disabling_cancels_and_reenabling_reschedules() {
        let mut mix = playing_a(3);
        assert_eq!(mix.handle(In::SetEnabled(false)), vec![A::CancelTransition]);
        assert_eq!(mix.status(), AutoMixStatus::Off);
        assert!(plan_of(&mix.handle(In::SetEnabled(true))).is_some());
    }
}

mod settings_changes {
    use super::*;

    #[test]
    fn new_duration_replans_a_pending_transition() {
        let mut mix = playing_a(3);
        let actions = mix.handle(In::SetCrossfadeDuration(fade(12.0)));
        assert_eq!(actions[0], A::CancelTransition);
        let plan = plan_of(&actions).unwrap();
        assert_eq!(plan.fade, s(12.0));
        assert_eq!(plan.start_at, s(168.0));
    }

    #[test]
    fn running_fade_keeps_its_duration_and_the_next_one_uses_the_new_value() {
        let mut mix = playing_a(3);
        mix.handle(In::TransitionStarted { from: DA, to: DB });
        assert_eq!(mix.handle(In::SetCrossfadeDuration(fade(16.0))), NONE);
        mix.handle(In::TransitionCompleted { from: DA, to: DB });
        let plan = plan_of(&mix.handle(loaded(DA, 3, untrimmed(120.0)))).unwrap();
        assert_eq!(plan.fade, s(16.0));
        assert_eq!(mix.settings().crossfade, fade(16.0));
    }

    #[test]
    fn new_curve_replans_a_pending_transition() {
        let mut mix = playing_a(3);
        let plan = plan_of(&mix.handle(In::SetCurve(CrossfadeCurve::Linear))).unwrap();
        assert_eq!(plan.curve, CrossfadeCurve::Linear);
        mix.handle(In::ManualIntervention);
        assert_eq!(mix.handle(In::SetCurve(CrossfadeCurve::Cut)), NONE);
    }
}

mod library_changes {
    use super::*;

    #[test]
    fn replacing_the_library_during_playback_reloads_the_standby_deck() {
        let mut mix = playing_a(3);
        let actions = mix.load_library(ids(&[7, 8]));
        assert_eq!(
            actions,
            vec![
                A::CancelTransition,
                A::LoadTrack {
                    deck: DB,
                    track: t(7)
                }
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Playing { active: DA });
    }

    #[test]
    fn replacing_the_library_while_idle_reloads_both_decks() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        let actions = mix.load_library(ids(&[5, 6]));
        assert_eq!(
            actions,
            vec![
                A::LoadTrack {
                    deck: DA,
                    track: t(5)
                },
                A::LoadTrack {
                    deck: DB,
                    track: t(6)
                },
            ]
        );
    }

    #[test]
    fn replacing_during_a_fade_waits_for_its_end() {
        let mut mix = playing_a(3);
        mix.handle(In::TransitionStarted { from: DA, to: DB });
        assert_eq!(mix.load_library(ids(&[7])), NONE);
        let actions = mix.handle(In::TransitionCompleted { from: DA, to: DB });
        assert_eq!(
            actions[1],
            A::LoadTrack {
                deck: DA,
                track: t(7)
            }
        );
    }

    #[test]
    fn replacing_while_waiting_reloads_the_awaited_deck() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1, 2]));
        mix.handle(loaded(DA, 1, untrimmed(60.0)));
        mix.handle(In::TrackEnded { deck: DA });
        assert_eq!(
            mix.load_library(ids(&[9])),
            vec![A::LoadTrack {
                deck: DB,
                track: t(9)
            }]
        );
        assert_eq!(mix.load_library(Vec::new()), vec![A::QueueExhausted]);
    }

    #[test]
    fn appending_after_the_end_starts_again() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(Vec::new());
        let actions = mix.append_to_library(&ids(&[4, 5]));
        assert_eq!(
            actions,
            vec![
                A::LoadTrack {
                    deck: DA,
                    track: t(4)
                },
                A::LoadTrack {
                    deck: DB,
                    track: t(5)
                },
            ]
        );
        assert_eq!(mix.state(), AutoMixState::Idle);
    }

    #[test]
    fn appending_fills_an_empty_standby_deck() {
        let mut mix = playing_a(1);
        assert_eq!(
            mix.append_to_library(&ids(&[2])),
            vec![A::LoadTrack {
                deck: DB,
                track: t(2)
            }]
        );
    }

    #[test]
    fn appending_while_idle_fills_the_empty_deck() {
        let mut mix = scheduler(settings(), RepeatMode::Off);
        mix.load_library(ids(&[1]));
        assert_eq!(
            mix.append_to_library(&ids(&[2])),
            vec![A::LoadTrack {
                deck: DB,
                track: t(2)
            }]
        );
    }

    #[test]
    fn turning_repeat_on_after_the_end_starts_a_new_cycle() {
        let mut mix = playing_a(1);
        mix.handle(In::TrackEnded { deck: DA });
        assert_eq!(mix.state(), AutoMixState::Finished);
        let actions = mix.set_repeat(RepeatMode::All);
        assert!(actions.contains(&A::LoadTrack {
            deck: DA,
            track: t(1)
        }));
    }

    #[test]
    fn shuffle_toggle_keeps_loaded_tracks() {
        let mut mix = playing_a(6);
        mix.set_order(PlayOrder::Shuffle);
        assert_eq!(mix.deck(DA).track(), Some(t(1)));
        assert_eq!(mix.deck(DB).track(), Some(t(2)));
        let mut waiting: Vec<_> = mix.queue().upcoming().collect();
        waiting.sort();
        assert_eq!(waiting, ids(&[3, 4, 5, 6]));
        assert_eq!(mix.queue().order(), PlayOrder::Shuffle);
    }

    #[test]
    fn waiting_tracks_can_be_reordered() {
        let mut mix = playing_a(5);
        mix.move_upcoming(2, 0).unwrap();
        assert_eq!(mix.queue().upcoming().collect::<Vec<_>>(), ids(&[5, 3, 4]));
        assert!(mix.move_upcoming(3, 0).is_err());
    }
}

mod mix_now {
    use super::*;

    #[test]
    fn starts_the_transition_at_the_current_position() {
        let mut mix = playing_a(3);
        // A transition is already planned for the end of the track: it is replaced.
        let actions = mix.handle(In::MixNow { position: s(42.0) });
        assert_eq!(actions[0], A::CancelTransition);
        let plan = plan_of(&actions).unwrap();
        assert_eq!((plan.from, plan.to), (DA, DB));
        assert_eq!(plan.start_at, s(42.0));
        assert_eq!(plan.fade, s(8.0));
        assert_eq!(mix.planned_transition(), Some(plan));
    }

    #[test]
    fn near_the_end_the_fade_shrinks_to_what_is_left() {
        let mut mix = playing_a(3);
        let actions = mix.handle(In::MixNow { position: s(177.0) });
        assert_eq!(plan_of(&actions).unwrap().fade, s(3.0));
    }

    #[test]
    fn resumes_a_suspended_automation() {
        let mut mix = playing_a(3);
        mix.handle(In::ManualIntervention);
        assert_eq!(mix.status(), AutoMixStatus::Suspended);
        let actions = mix.handle(In::MixNow { position: s(10.0) });
        assert_eq!(plan_of(&actions).unwrap().start_at, s(10.0));
        assert_eq!(mix.status(), AutoMixStatus::Running);
    }

    #[test]
    fn does_nothing_without_a_track_waiting_or_when_disabled() {
        let mut mix = playing_a(1);
        assert_eq!(mix.handle(In::MixNow { position: s(10.0) }), NONE);

        let mut mix = playing_a(3);
        mix.handle(In::SetEnabled(false));
        assert_eq!(mix.handle(In::MixNow { position: s(10.0) }), NONE);

        let mut mix = playing_a(3);
        mix.handle(In::MixNow { position: s(10.0) });
        mix.handle(In::TransitionStarted { from: DA, to: DB });
        assert_eq!(mix.handle(In::MixNow { position: s(11.0) }), NONE);
    }
}
