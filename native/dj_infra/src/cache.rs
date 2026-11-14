//! Persistent cache of track analyses (SQLite).
//!
//! Analysing a track decodes the whole file; the result only depends on the
//! file content and the analysis options. [`CachedAnalyzer`] looks the result
//! up by path and reuses it while the file size, its modification time, the
//! options and the algorithm version are unchanged. The cache is an
//! optimisation: any cache error falls back to a fresh analysis.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use dj_domain::ports::{AnalysisRequest, DecodeError, TrackAnalysis, TrackAnalyzer};
use dj_domain::{BeatGrid, Bpm, Seconds, TrackTiming, Waveform};
use rusqlite::{Connection, OptionalExtension, params};

/// Bumped whenever the analysis algorithms change, to invalidate old entries.
pub const ANALYSIS_VERSION: i64 = 2;

/// File name of the cache inside the application data folder.
const FILE_NAME: &str = "analysis-cache.sqlite3";

/// Identity of a file version: an entry is valid while it is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    size: i64,
    modified_ns: i64,
}

impl FileStamp {
    fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        Some(Self {
            size: i64::try_from(metadata.len()).ok()?,
            modified_ns: i64::try_from(modified.as_nanos()).ok()?,
        })
    }
}

/// SQLite store of analyses, keyed by file path.
#[derive(Debug)]
pub struct SqliteAnalysisCache {
    connection: Connection,
}

impl SqliteAnalysisCache {
    /// Opens (or creates) the cache at `path`.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or initialised.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::init(Connection::open(path)?)
    }

    /// A cache that lives in memory only (tests).
    ///
    /// # Errors
    /// Returns an error if SQLite cannot be initialised.
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    /// Default location: `<data dir>/Platine/analysis-cache.sqlite3`
    /// (`~/Library/Application Support` on macOS, inside the sandbox container).
    pub fn default_path() -> Option<PathBuf> {
        dirs::data_dir().map(|dir| dir.join("Platine").join(FILE_NAME))
    }

    fn init(connection: Connection) -> rusqlite::Result<Self> {
        connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS analysis (
                 path        TEXT PRIMARY KEY,
                 size        INTEGER NOT NULL,
                 modified_ns INTEGER NOT NULL,
                 options     TEXT NOT NULL,
                 version     INTEGER NOT NULL,
                 duration    REAL NOT NULL,
                 start       REAL NOT NULL,
                 end         REAL NOT NULL,
                 bpm         REAL,
                 first_beat  REAL,
                 points_per_second INTEGER NOT NULL,
                 low  BLOB NOT NULL,
                 mid  BLOB NOT NULL,
                 high BLOB NOT NULL
             );",
        )?;
        Ok(Self { connection })
    }

    fn load(&self, path: &str, stamp: &FileStamp, options: &str) -> Option<TrackAnalysis> {
        self.connection
            .query_row(
                "SELECT duration, start, end, bpm, first_beat, points_per_second, low, mid, high
                 FROM analysis
                 WHERE path = ?1 AND size = ?2 AND modified_ns = ?3 AND options = ?4
                   AND version = ?5",
                params![
                    path,
                    stamp.size,
                    stamp.modified_ns,
                    options,
                    ANALYSIS_VERSION
                ],
                |row| {
                    Ok((
                        row.get::<_, f64>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, Option<f64>>(3)?,
                        row.get::<_, Option<f64>>(4)?,
                        row.get::<_, u16>(5)?,
                        row.get::<_, Vec<u8>>(6)?,
                        row.get::<_, Vec<u8>>(7)?,
                        row.get::<_, Vec<u8>>(8)?,
                    ))
                },
            )
            .optional()
            .ok()
            .flatten()
            .and_then(
                |(duration, start, end, bpm, first_beat, points, low, mid, high)| {
                    let timing = TrackTiming::new(
                        Seconds::new(duration).ok()?,
                        Seconds::new(start).ok()?,
                        Seconds::new(end).ok()?,
                    )
                    .ok()?;
                    let beat_grid = match (bpm, first_beat) {
                        // Stored from an f32 tempo: no truncation.
                        #[allow(clippy::cast_possible_truncation)]
                        (Some(bpm), Some(first_beat)) => Some(BeatGrid {
                            bpm: Bpm::new(bpm as f32).ok()?,
                            first_beat: Seconds::new(first_beat).ok()?,
                        }),
                        _ => None,
                    };
                    Some(TrackAnalysis {
                        timing,
                        beat_grid,
                        waveform: Waveform::from_bands(points, low, mid, high),
                    })
                },
            )
    }

    fn store(&self, path: &str, stamp: &FileStamp, options: &str, analysis: &TrackAnalysis) {
        let timing = analysis.timing;
        let grid = analysis.beat_grid;
        let waveform = &analysis.waveform;
        let _ = self.connection.execute(
            "INSERT OR REPLACE INTO analysis
                 (path, size, modified_ns, options, version, duration, start, end, bpm,
                  first_beat, points_per_second, low, mid, high)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                path,
                stamp.size,
                stamp.modified_ns,
                options,
                ANALYSIS_VERSION,
                timing.duration().as_f64(),
                timing.effective_start().as_f64(),
                timing.effective_end().as_f64(),
                grid.map(|grid| f64::from(grid.bpm.value())),
                grid.map(|grid| grid.first_beat.as_f64()),
                waveform.points_per_second(),
                waveform.low(),
                waveform.mid(),
                waveform.high(),
            ],
        );
    }

    /// Number of stored analyses.
    pub fn len(&self) -> usize {
        self.connection
            .query_row("SELECT COUNT(*) FROM analysis", [], |row| {
                row.get::<_, i64>(0)
            })
            .ok()
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Options that change the result of an analysis, as a cache key part.
fn options_key(request: AnalysisRequest) -> String {
    format!(
        "silence={:.1};bpm={}",
        request.silence_threshold_db, request.detect_bpm
    )
}

/// Wraps an analyzer with an optional persistent cache.
#[derive(Debug)]
pub struct CachedAnalyzer<A> {
    inner: A,
    cache: Option<SqliteAnalysisCache>,
}

impl<A> CachedAnalyzer<A> {
    /// `cache: None` analyses every time.
    pub fn new(inner: A, cache: Option<SqliteAnalysisCache>) -> Self {
        Self { inner, cache }
    }

    pub fn cache(&self) -> Option<&SqliteAnalysisCache> {
        self.cache.as_ref()
    }
}

impl<A: TrackAnalyzer> TrackAnalyzer for CachedAnalyzer<A> {
    fn analyze(
        &self,
        path: &Path,
        request: &AnalysisRequest,
    ) -> Result<TrackAnalysis, DecodeError> {
        let Some(cache) = &self.cache else {
            return self.inner.analyze(path, request);
        };
        let key = path.to_string_lossy();
        let options = options_key(*request);
        let stamp = FileStamp::of(path);
        if let Some(stamp) = &stamp
            && let Some(hit) = cache.load(&key, stamp, &options)
        {
            return Ok(hit);
        }
        let analysis = self.inner.analyze(path, request)?;
        if let Some(stamp) = &stamp {
            cache.store(&key, stamp, &options, &analysis);
        }
        Ok(analysis)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// Counts calls and returns a fixed analysis.
    struct Counting {
        calls: Cell<u32>,
    }

    impl TrackAnalyzer for Counting {
        fn analyze(
            &self,
            _path: &Path,
            request: &AnalysisRequest,
        ) -> Result<TrackAnalysis, DecodeError> {
            self.calls.set(self.calls.get() + 1);
            Ok(TrackAnalysis {
                timing: TrackTiming::new(
                    Seconds::new(10.0).unwrap(),
                    Seconds::new(0.5).unwrap(),
                    Seconds::new(9.0).unwrap(),
                )
                .unwrap(),
                beat_grid: request.detect_bpm.then(|| BeatGrid {
                    bpm: Bpm::new(124.5).unwrap(),
                    first_beat: Seconds::new(0.25).unwrap(),
                }),
                waveform: Waveform::from_bands(100, vec![1, 2], vec![3, 4], vec![5, 6]),
            })
        }
    }

    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("platine-cache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn second_analysis_of_an_unchanged_file_comes_from_the_cache() {
        let path = temp_file("a.wav", b"audio");
        let analyzer = CachedAnalyzer::new(
            Counting {
                calls: Cell::new(0),
            },
            Some(SqliteAnalysisCache::in_memory().unwrap()),
        );
        let request = AnalysisRequest::default();
        let first = analyzer.analyze(&path, &request).unwrap();
        let second = analyzer.analyze(&path, &request).unwrap();
        assert_eq!(first, second);
        assert_eq!(analyzer.inner.calls.get(), 1);
        assert_eq!(analyzer.cache().unwrap().len(), 1);
        assert_eq!(second.beat_grid.unwrap().bpm.value(), 124.5);
        assert_eq!(second.waveform.mid(), &[3, 4]);
    }

    #[test]
    fn changed_files_and_options_are_analysed_again() {
        let path = temp_file("b.wav", b"audio");
        let analyzer = CachedAnalyzer::new(
            Counting {
                calls: Cell::new(0),
            },
            Some(SqliteAnalysisCache::in_memory().unwrap()),
        );
        let request = AnalysisRequest::default();
        analyzer.analyze(&path, &request).unwrap();
        // Another silence threshold changes the result.
        let quieter = AnalysisRequest {
            silence_threshold_db: -60.0,
            ..request
        };
        analyzer.analyze(&path, &quieter).unwrap();
        assert_eq!(analyzer.inner.calls.get(), 2);
        // A different size means a different file version.
        std::fs::write(&path, b"longer audio").unwrap();
        analyzer.analyze(&path, &request).unwrap();
        assert_eq!(analyzer.inner.calls.get(), 3);
        // Without BPM detection, the stored grid is absent.
        let without = analyzer
            .analyze(
                &path,
                &AnalysisRequest {
                    detect_bpm: false,
                    ..request
                },
            )
            .unwrap();
        assert_eq!(without.beat_grid, None);
    }

    #[test]
    fn the_cache_persists_on_disk_and_works_without_a_cache() {
        let audio = temp_file("c.wav", b"audio");
        let db = std::env::temp_dir().join(format!(
            "platine-cache-{}/nested/cache.sqlite3",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&db);
        {
            let analyzer = CachedAnalyzer::new(
                Counting {
                    calls: Cell::new(0),
                },
                Some(SqliteAnalysisCache::open(&db).unwrap()),
            );
            analyzer
                .analyze(&audio, &AnalysisRequest::default())
                .unwrap();
        }
        let reopened = CachedAnalyzer::new(
            Counting {
                calls: Cell::new(0),
            },
            Some(SqliteAnalysisCache::open(&db).unwrap()),
        );
        reopened
            .analyze(&audio, &AnalysisRequest::default())
            .unwrap();
        assert_eq!(reopened.inner.calls.get(), 0);

        let uncached = CachedAnalyzer::new(
            Counting {
                calls: Cell::new(0),
            },
            None,
        );
        uncached
            .analyze(&audio, &AnalysisRequest::default())
            .unwrap();
        uncached
            .analyze(&audio, &AnalysisRequest::default())
            .unwrap();
        assert_eq!(uncached.inner.calls.get(), 2);
        assert!(SqliteAnalysisCache::default_path().is_some());
        assert!(!reopened.cache().unwrap().is_empty());
    }
}
