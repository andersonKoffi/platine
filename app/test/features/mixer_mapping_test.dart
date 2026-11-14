import 'package:flutter_test/flutter_test.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/features/library/presentation/library_panel.dart';
import 'package:platine/features/mixer/presentation/mixer_panel.dart';

void main() {
  test('EQ knobs are neutral in the middle and round-trip', () {
    expect(eqToKnob(1), 0.5);
    expect(eqToKnob(0), 0);
    expect(eqToKnob(2), 1);
    for (final level in [0.0, 0.3, 1.0, 1.4, 2.0]) {
      expect(knobToEq(eqToKnob(level)), closeTo(level, 1e-9));
    }
  });

  test('channel faders sit at unity at 80 % and round-trip', () {
    expect(gainToFader(1), 0.8);
    expect(gainToFader(2), 1);
    for (final gain in [0.0, 0.5, 1.0, 1.7, 2.0]) {
      expect(faderToGain(gainToFader(gain)), closeTo(gain, 1e-9));
    }
  });

  test('table helpers and clocks', () {
    expect(folderOf('/m/Set/a.mp3'), 'Set');
    expect(folderOf('a.mp3'), '');
    expect(formatOf('/m/Set/a.flac'), 'FLAC');
    expect(formatOf('/m/noext'), '');
    expect(formatPreciseClock(83.47), '1:23.4');
    expect(formatPreciseClock(-1), '0:00.0');
    expect(formatClock(61), '1:01');
  });
}
