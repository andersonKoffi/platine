//! Recursive discovery of audio files.

use std::path::Path;

use dj_domain::ports::{DecodeError, LibraryScanner, ScanReport, SkippedFile};
use dj_domain::{Track, TrackId};
use walkdir::WalkDir;

/// File extensions handled by the decoder (lower case).
pub const AUDIO_EXTENSIONS: [&str; 10] = [
    "mp3", "flac", "wav", "wave", "aif", "aiff", "m4a", "aac", "ogg", "oga",
];

/// Returns `true` if the extension of `path` is a supported audio format.
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Scans folders recursively; hidden files (`.` prefix, e.g. macOS `._` files) are ignored.
#[derive(Debug, Clone, Copy)]
pub struct FolderScanner {
    first_id: u64,
}

impl FolderScanner {
    /// Track identifiers are assigned from `first_id` upwards, in path order.
    pub fn starting_at(first_id: u64) -> Self {
        Self { first_id }
    }
}

impl Default for FolderScanner {
    fn default() -> Self {
        Self::starting_at(1)
    }
}

impl LibraryScanner for FolderScanner {
    fn scan(&self, root: &Path) -> Result<ScanReport, DecodeError> {
        if !root.is_dir() {
            return Err(DecodeError::Io(format!(
                "{} is not a folder",
                root.display()
            )));
        }
        let mut report = ScanReport::default();
        let mut paths = Vec::new();
        let walker = WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|entry| entry.depth() == 0 || !is_hidden(entry.file_name()));
        for entry in walker {
            match entry {
                Ok(entry) if entry.file_type().is_file() && is_audio_file(entry.path()) => {
                    paths.push(entry.into_path());
                }
                Ok(_) => {}
                Err(err) => report.skipped.push(SkippedFile {
                    path: err.path().map(Path::to_path_buf).unwrap_or_default(),
                    reason: err.to_string(),
                }),
            }
        }
        paths.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
        report.tracks = (self.first_id..)
            .zip(paths)
            .map(|(id, path)| Track::from_path(TrackId(id), path))
            .collect();
        Ok(report)
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("platine-scan-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn recognises_audio_extensions_case_insensitively() {
        assert!(is_audio_file(Path::new("a/B.MP3")));
        assert!(is_audio_file(Path::new("x.flac")));
        assert!(!is_audio_file(Path::new("cover.jpg")));
        assert!(!is_audio_file(Path::new("README")));
    }

    #[test]
    fn finds_audio_files_recursively_in_alphabetical_order() {
        let dir = temp_dir("rec");
        std::fs::create_dir_all(dir.join("Sub/Deeper")).unwrap();
        for file in [
            "b.mp3",
            "A.flac",
            "Sub/c.wav",
            "Sub/Deeper/d.m4a",
            "cover.jpg",
            "._b.mp3",
        ] {
            std::fs::write(dir.join(file), b"x").unwrap();
        }
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        std::fs::write(dir.join(".hidden/e.mp3"), b"x").unwrap();

        let report = FolderScanner::starting_at(10).scan(&dir).unwrap();
        let names: Vec<String> = report
            .tracks
            .iter()
            .map(|t| {
                t.path
                    .strip_prefix(&dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(
            names,
            vec!["A.flac", "b.mp3", "Sub/c.wav", "Sub/Deeper/d.m4a"]
        );
        assert_eq!(report.tracks[0].id, TrackId(10));
        assert_eq!(report.tracks[3].id, TrackId(13));
        assert_eq!(report.skipped, Vec::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_a_missing_folder() {
        let err = FolderScanner::default()
            .scan(Path::new("/definitely/not/here"))
            .unwrap_err();
        assert!(matches!(err, DecodeError::Io(_)));
    }

    #[test]
    fn empty_folder_gives_an_empty_report() {
        let dir = temp_dir("empty");
        let report = FolderScanner::default().scan(&dir).unwrap();
        assert_eq!(report.tracks, Vec::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
