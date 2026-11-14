//! Tracks, their audible boundaries, and playlists.

use std::path::{Path, PathBuf};

use crate::error::DomainError;
use crate::values::{Bpm, Seconds};

/// Stable identifier of a track inside a library session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrackId(pub u64);

/// Metadata of an audio file known to the library.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: TrackId,
    pub path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub duration: Option<Seconds>,
    pub bpm: Option<Bpm>,
    pub key: Option<String>,
}

impl Track {
    /// A track known only by its path; the title defaults to the file stem.
    pub fn from_path(id: TrackId, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let title = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        Self {
            id,
            path,
            title,
            artist: None,
            duration: None,
            bpm: None,
            key: None,
        }
    }

    /// Key used for the alphabetical (natural) order: the full path, case-insensitive.
    pub fn sort_key(&self) -> String {
        path_sort_key(&self.path)
    }
}

fn path_sort_key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

/// Audible boundaries of a decoded track.
///
/// `effective_start` skips the leading silence and `effective_end` drops the
/// trailing silence, so that auto-mix never fades into or out of silence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackTiming {
    duration: Seconds,
    effective_start: Seconds,
    effective_end: Seconds,
}

impl TrackTiming {
    /// Builds a timing from explicit boundaries.
    ///
    /// # Errors
    /// Returns an error unless `start <= end <= duration`.
    pub fn new(
        duration: Seconds,
        effective_start: Seconds,
        effective_end: Seconds,
    ) -> Result<Self, DomainError> {
        if effective_start > effective_end {
            return Err(DomainError::InvalidTiming {
                reason: "effective start is after effective end",
            });
        }
        if effective_end > duration {
            return Err(DomainError::InvalidTiming {
                reason: "effective end is after the end of the track",
            });
        }
        Ok(Self {
            duration,
            effective_start,
            effective_end,
        })
    }

    /// A timing without any silence trimming.
    pub fn untrimmed(duration: Seconds) -> Self {
        Self {
            duration,
            effective_start: Seconds::ZERO,
            effective_end: duration,
        }
    }

    /// Builds a timing from detected leading and trailing silences.
    ///
    /// A track that is silent from start to end is kept untrimmed.
    pub fn from_silences(duration: Seconds, leading: Seconds, trailing: Seconds) -> Self {
        if leading + trailing >= duration {
            return Self::untrimmed(duration);
        }
        Self {
            duration,
            effective_start: leading,
            effective_end: duration - trailing,
        }
    }

    pub fn duration(&self) -> Seconds {
        self.duration
    }

    pub fn effective_start(&self) -> Seconds {
        self.effective_start
    }

    pub fn effective_end(&self) -> Seconds {
        self.effective_end
    }

    /// Length of the audible part.
    pub fn playable(&self) -> Seconds {
        self.effective_end - self.effective_start
    }
}

/// An ordered, named list of tracks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Playlist {
    pub name: String,
    tracks: Vec<Track>,
}

impl Playlist {
    /// Builds a playlist sorted in natural (alphabetical path) order.
    pub fn alphabetical(name: impl Into<String>, mut tracks: Vec<Track>) -> Self {
        tracks.sort_by_cached_key(Track::sort_key);
        Self {
            name: name.into(),
            tracks,
        }
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Identifiers in playlist order.
    pub fn track_ids(&self) -> Vec<TrackId> {
        self.tracks.iter().map(|track| track.id).collect()
    }

    pub fn find(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Adds tracks and restores the natural order.
    pub fn extend(&mut self, tracks: Vec<Track>) {
        self.tracks.extend(tracks);
        self.tracks.sort_by_cached_key(Track::sort_key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(value: f64) -> Seconds {
        Seconds::saturating(value)
    }

    #[test]
    fn title_defaults_to_file_stem() {
        let track = Track::from_path(TrackId(1), "/music/House/Intro Mix.flac");
        assert_eq!(track.title, "Intro Mix");
        assert_eq!(track.sort_key(), "/music/house/intro mix.flac");
        assert_eq!(Track::from_path(TrackId(2), "/").title, "");
    }

    #[test]
    fn timing_validates_its_boundaries() {
        assert!(TrackTiming::new(secs(10.0), secs(5.0), secs(4.0)).is_err());
        assert!(TrackTiming::new(secs(10.0), secs(1.0), secs(11.0)).is_err());
        let timing = TrackTiming::new(secs(10.0), secs(1.0), secs(9.0)).unwrap();
        assert_eq!(timing.duration(), secs(10.0));
        assert_eq!(timing.playable(), secs(8.0));
    }

    #[test]
    fn timing_from_silences_trims_both_ends() {
        let timing = TrackTiming::from_silences(secs(200.0), secs(0.5), secs(3.0));
        assert_eq!(timing.effective_start(), secs(0.5));
        assert_eq!(timing.effective_end(), secs(197.0));
    }

    #[test]
    fn fully_silent_tracks_are_not_trimmed() {
        let timing = TrackTiming::from_silences(secs(5.0), secs(3.0), secs(3.0));
        assert_eq!(timing, TrackTiming::untrimmed(secs(5.0)));
    }

    #[test]
    fn playlist_sorts_alphabetically_case_insensitively() {
        let playlist = Playlist::alphabetical(
            "set",
            vec![
                Track::from_path(TrackId(1), "/m/b.mp3"),
                Track::from_path(TrackId(2), "/m/A.mp3"),
                Track::from_path(TrackId(3), "/m/c.mp3"),
            ],
        );
        assert_eq!(
            playlist.track_ids(),
            vec![TrackId(2), TrackId(1), TrackId(3)]
        );
        assert_eq!(playlist.len(), 3);
        assert!(!playlist.is_empty());
        assert_eq!(
            playlist.find(TrackId(3)).map(|t| t.title.as_str()),
            Some("c")
        );
        assert!(playlist.find(TrackId(9)).is_none());
    }

    #[test]
    fn playlist_extension_keeps_natural_order() {
        let mut playlist = Playlist::alphabetical("set", vec![Track::from_path(TrackId(1), "b")]);
        playlist.extend(vec![Track::from_path(TrackId(2), "a")]);
        assert_eq!(playlist.track_ids(), vec![TrackId(2), TrackId(1)]);
        assert_eq!(playlist.tracks()[0].title, "a");
        assert!(Playlist::default().is_empty());
    }
}
