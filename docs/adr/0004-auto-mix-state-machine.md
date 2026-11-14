# 0004 — Auto-mix as a pure, event-driven state machine

- Status: Accepted
- Date: 2026-11-14

## Context

Auto-mix is the core feature: two decks, an automatic crossfade before the
end of each track, roles swapping after every transition, shuffle without
repetition, and many edge cases (short tracks, empty folder, decode errors,
manual interventions, settings changed mid-fade). It must be testable
deterministically, without audio, files or wall-clock time.

## Decision

- `AutoMixScheduler<R: RandomSource>` in `dj_domain` is a **pure state
  machine** (`Idle → Playing → Crossfading → Playing → … → Finished`, plus
  `WaitingForNext`). It consumes `AutoMixInput` events (settings, engine and
  decoder reports, manual actions) and returns `AutoMixAction` commands. It
  owns the `PlaybackQueue` and the injected random source.
- **Transitions are planned ahead** (`ScheduleTransition`) as soon as both
  decks are ready, using positions on the outgoing deck's timeline
  (`start_at = effective_end − fade`). The engine executes them
  sample-accurately (ADR 0003). The scheduler never polls positions.
- Fade length: `min(configured, outgoing playable / 2, incoming playable / 2)`.
  A configured fade of 0 switches exactly at `effective_end`.
- Changing the duration or curve re-plans a *pending* transition; a running
  fade keeps its length.
- "Automation active" = enabled **and** not suspended. A manual intervention
  suspends it (the pending plan is cancelled, a running fade is frozen);
  resuming either re-plans or finishes the interrupted fade at once.
- Decode errors remove the track from the queue for good and load the next
  one into the same deck without stopping the music.
- Shuffle: Fisher-Yates over the whole library per cycle; a new cycle never
  starts with one of the last two tracks when the library has 3 tracks or
  more (best effort below that). Toggling shuffle only reorders waiting tracks.
- Crossfade curves are a Strategy (`EqualPower` default, `Linear`, `Cut`);
  gains are clamped to `[0, 1]`.

## Consequences

- 100+ deterministic tests, including property tests (permutation, no
  repetition before exhaustion, equal power, random event sequences keep the
  invariants) and an end-to-end simulation driven by a manual clock.
- The controller (P3) is thin glue: forward events in, execute commands out.
- Some timing precision depends on the engine honouring `start_at`; the
  simulation tolerates one step of jitter per transition, the engine must not.
