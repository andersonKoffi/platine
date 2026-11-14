import 'package:flutter_test/flutter_test.dart';
import 'package:platine/features/library/application/library_controller.dart';

import '../helpers/fakes.dart';

void main() {
  test('search matches title, artist and path, case-insensitively', () {
    expect(filterTracks(tracks, const LibraryQuery(text: 'GROO')).single.id, 2);
    expect(filterTracks(tracks, const LibraryQuery(text: 'dj')).single.id, 1);
    expect(filterTracks(tracks, const LibraryQuery(text: 'set')).length, 3);
    expect(filterTracks(tracks, const LibraryQuery(text: 'zzz')), isEmpty);
  });

  test('tracks can be sorted by title', () {
    final sorted = filterTracks(
      tracks,
      const LibraryQuery(sort: LibrarySort.title),
    );
    expect(sorted.map((t) => t.title), ['Finale', 'Groove', 'Intro']);
    expect(filterTracks(tracks, const LibraryQuery()).map((t) => t.id), [
      1,
      2,
      3,
    ]);
  });

  test('tracks can be sorted by tempo, unknown tempos last', () {
    final withTempos = [
      tracks[0].withTempo(bpm: 128, firstBeat: 0),
      tracks[1],
      tracks[2].withTempo(bpm: 96.5, firstBeat: 0.2),
    ];
    final sorted = filterTracks(
      withTempos,
      const LibraryQuery(sort: LibrarySort.bpm),
    );
    expect(sorted.map((t) => t.id), [3, 1, 2]);
  });
}
