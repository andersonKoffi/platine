import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/decks/application/deck_controls.dart';
import 'package:platine/features/decks/presentation/pitch_control.dart';

import '../helpers/fakes.dart';

void main() {
  late FakeEngineGateway gateway;
  late ProviderContainer container;

  setUp(() {
    gateway = FakeEngineGateway();
    container = ProviderContainer(overrides: testOverrides(gateway));
    addTearDown(container.dispose);
  });

  DeckControlsController deck(DeckSide side) =>
      container.read(deckControlsControllerProvider(side).notifier);
  DeckControls state(DeckSide side) =>
      container.read(deckControlsControllerProvider(side));

  test('the pitch stays within the selected range', () {
    deck(DeckSide.a).setPitch(4);
    expect(state(DeckSide.a).pitch, 4);
    expect(state(DeckSide.a).speed, 1.04);
    deck(DeckSide.a).setPitch(30);
    expect(state(DeckSide.a).pitch, 8);
    expect(gateway.calls, ['setPitch a 4.0', 'setPitch a 8.0']);
    // Deck B is independent.
    expect(state(DeckSide.b).pitch, 0);
  });

  test('the fader maps its position onto the range', () {
    deck(DeckSide.b).setPitchPosition(0.75);
    expect(state(DeckSide.b).pitch, 4);
    expect(state(DeckSide.b).pitchPosition, 0.75);
    deck(DeckSide.b).setPitchPosition(0);
    expect(state(DeckSide.b).pitch, -8);
    deck(DeckSide.b).resetPitch();
    expect(state(DeckSide.b).pitch, 0);
    expect(state(DeckSide.b).pitchPosition, 0.5);
  });

  test('changing the range keeps the pitch inside it', () {
    final controller = deck(DeckSide.a)..cycleRange();
    expect(state(DeckSide.a).pitchRange, PitchRangeOption.sixteen);
    controller.setPitch(12);
    controller
      ..cycleRange() // ±50
      ..cycleRange(); // back to ±8
    expect(state(DeckSide.a).pitchRange, PitchRangeOption.eight);
    expect(state(DeckSide.a).pitch, 8);
    expect(gateway.calls.last, 'setPitch a 8.0');
  });

  test('pitch values read like on a DJ controller', () {
    expect(formatPitch(0), '0.00%');
    expect(formatPitch(2.345), '+2.35%');
    expect(formatPitch(-1), '−1.00%');
  });

  test('sync matches the tempo heard on the other deck', () {
    expect(syncPitch(bpm: 120, targetBpm: 126), 5);
    expect(syncPitch(bpm: 128, targetBpm: 124), -3.13);
    // Half and double tempo.
    expect(syncPitch(bpm: 70, targetBpm: 140), 0);
    expect(syncPitch(bpm: 150, targetBpm: 76), 1.33);
    expect(syncPitch(bpm: 100, targetBpm: 0), isNull);
    expect(rangeFor(5), PitchRangeOption.eight);
    expect(rangeFor(-12), PitchRangeOption.sixteen);
    expect(rangeFor(30), PitchRangeOption.fifty);

    final controller = deck(DeckSide.b);
    expect(controller.syncTo(bpm: 100, targetBpm: 112), isTrue);
    expect(state(DeckSide.b).pitch, 12);
    expect(state(DeckSide.b).pitchRange, PitchRangeOption.sixteen);
    expect(gateway.calls.last, 'setPitch b 12.0');
  });

  test('keylock toggles per deck', () {
    deck(DeckSide.a).toggleKeylock();
    expect(state(DeckSide.a).keylock, isTrue);
    expect(state(DeckSide.b).keylock, isFalse);
    deck(DeckSide.a).toggleKeylock();
    expect(gateway.calls, ['setKeylock a true', 'setKeylock a false']);
  });
}
