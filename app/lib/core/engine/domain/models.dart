import 'dart:typed_data';

import 'package:flutter/foundation.dart';

/// One of the two decks.
enum DeckSide {
  a,
  b;

  DeckSide get other => this == a ? b : a;

  /// Crossfader position where only this deck is heard.
  double get crossfaderSide => this == a ? 0 : 1;

  String get label => this == a ? 'A' : 'B';
}

/// Shape of the crossfader.
enum CrossfadeCurveType { equalPower, linear, cut }

/// One of the three isolator bands.
enum EqBandType { low, mid, high }

/// What a newly chosen folder does to the queue.
enum FolderLoadMode { replace, append }

/// Automation status shown to the user.
enum AutoMixMode { off, running, suspended }

/// Where the auto-mix state machine stands.
enum AutoMixPhase { idle, playing, crossfading, waitingForNext, finished }

/// A track of the library.
@immutable
class TrackInfo {
  const new({
    required this.id,
    required this.path,
    required this.title,
    this.artist,
  });

  final int id;
  final String path;
  final String title;
  final String? artist;

  @override
  bool operator ==(Object other) =>
      other is TrackInfo &&
      other.id == id &&
      other.path == path &&
      other.title == title &&
      other.artist == artist;

  @override
  int get hashCode => Object.hash(id, path, title, artist);
}

/// A file that could not be used.
@immutable
class SkippedEntry {
  const new({required this.path, required this.reason});

  final String path;
  final String reason;
}

/// Result of scanning a folder.
@immutable
class ScanResult {
  const new({required this.tracks, required this.skipped});

  final List<TrackInfo> tracks;
  final List<SkippedEntry> skipped;
}

/// Peak levels in linear scale (1 = 0 dBFS), max of left and right.
@immutable
class LevelSnapshot {
  const new({this.deckA = 0, this.deckB = 0, this.master = 0});

  final double deckA;
  final double deckB;
  final double master;

  double of(DeckSide side) => side == DeckSide.a ? deckA : deckB;
}

/// Settings of the auto-mix and the analysis, persisted between sessions.
@immutable
class MixSettings {
  const new({
    this.automix = true,
    this.crossfadeSeconds = defaultCrossfadeSeconds,
    this.curve = CrossfadeCurveType.equalPower,
    this.shuffle = false,
    this.repeat = false,
    this.silenceThresholdDb = -50,
  });

  static const double defaultCrossfadeSeconds = 8;
  static const double maxCrossfadeSeconds = 30;
  static const List<double> crossfadePresets = [0, 4, 8, 12, 16];

  final bool automix;
  final double crossfadeSeconds;
  final CrossfadeCurveType curve;
  final bool shuffle;
  final bool repeat;
  final double silenceThresholdDb;

  MixSettings copyWith({
    bool? automix,
    double? crossfadeSeconds,
    CrossfadeCurveType? curve,
    bool? shuffle,
    bool? repeat,
    double? silenceThresholdDb,
  }) {
    return MixSettings(
      automix: automix ?? this.automix,
      crossfadeSeconds: crossfadeSeconds ?? this.crossfadeSeconds,
      curve: curve ?? this.curve,
      shuffle: shuffle ?? this.shuffle,
      repeat: repeat ?? this.repeat,
      silenceThresholdDb: silenceThresholdDb ?? this.silenceThresholdDb,
    );
  }

  @override
  bool operator ==(Object other) =>
      other is MixSettings &&
      other.automix == automix &&
      other.crossfadeSeconds == crossfadeSeconds &&
      other.curve == curve &&
      other.shuffle == shuffle &&
      other.repeat == repeat &&
      other.silenceThresholdDb == silenceThresholdDb;

  @override
  int get hashCode => Object.hash(
    automix,
    crossfadeSeconds,
    curve,
    shuffle,
    repeat,
    silenceThresholdDb,
  );
}

/// Coloured overview of a track: peaks of the low, mid and high bands per
/// window, `0..=255`.
@immutable
class WaveformData {
  const new({
    required this.pointsPerSecond,
    required this.low,
    required this.mid,
    required this.high,
  });

  final int pointsPerSecond;
  final Uint8List low;
  final Uint8List mid;
  final Uint8List high;

  int get length => low.length;

  /// Seconds covered by the waveform.
  double get duration => pointsPerSecond == 0 ? 0 : length / pointsPerSecond;
}
