//! Decoding real files generated on the fly.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod common;

use common::TempDir;
use dj_domain::Seconds;
use dj_domain::ports::{AudioStream, DecodeError, TrackDecoder};
use dj_infra::SymphoniaDecoder;
use test_support::wav::{Tone, write_corrupt};

fn read_all(stream: &mut impl AudioStream) -> Vec<f32> {
    let mut all = Vec::new();
    let mut buffer = vec![0.0; 4096];
    loop {
        let read = stream.read(&mut buffer).unwrap();
        if read == 0 {
            return all;
        }
        all.extend_from_slice(&buffer[..read]);
    }
}

#[test]
fn decodes_a_stereo_wav_file() {
    let dir = TempDir::new("decode-stereo");
    let path = dir.file("a.wav");
    Tone::sine(440.0, 1.0).write(&path).unwrap();

    let mut stream = SymphoniaDecoder.open(&path).unwrap();
    assert_eq!(stream.spec().sample_rate, 44_100);
    assert_eq!(stream.spec().channels, 2);
    let duration = stream.duration().unwrap().as_f64();
    assert!((duration - 1.0).abs() < 1e-3, "{duration}");
    assert!(format!("{stream:?}").contains("SymphoniaStream"));

    let samples = read_all(&mut stream);
    assert_eq!(samples.len(), 44_100 * 2);
    let peak = samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!((peak - 0.5).abs() < 0.01);
}

#[test]
fn decodes_a_mono_file() {
    let dir = TempDir::new("decode-mono");
    let path = dir.file("mono.wav");
    Tone::sine(220.0, 0.5)
        .mono()
        .at_rate(48_000)
        .write(&path)
        .unwrap();
    let mut stream = SymphoniaDecoder.open(&path).unwrap();
    assert_eq!(stream.spec().channels, 1);
    assert_eq!(read_all(&mut stream).len(), 24_000);
}

#[test]
fn seeks_to_a_position() {
    let dir = TempDir::new("decode-seek");
    let path = dir.file("seek.wav");
    Tone::sine(440.0, 2.0).write(&path).unwrap();
    let mut stream = SymphoniaDecoder.open(&path).unwrap();
    stream.seek(Seconds::saturating(1.5)).unwrap();
    let remaining = read_all(&mut stream).len() / 2;
    assert!(remaining.abs_diff(22_050) <= 64, "{remaining}");
}

#[test]
fn rejects_corrupt_and_missing_files() {
    let dir = TempDir::new("decode-bad");
    let corrupt = dir.file("corrupt.wav");
    write_corrupt(&corrupt).unwrap();
    assert!(SymphoniaDecoder.open(&corrupt).is_err());

    let text = dir.file("notes.mp3");
    std::fs::write(&text, "not music at all").unwrap();
    assert!(SymphoniaDecoder.open(&text).is_err());

    let missing = dir.file("missing.flac");
    assert!(matches!(
        SymphoniaDecoder.open(&missing),
        Err(DecodeError::Io(_))
    ));
    assert!(dir.path().exists());
}
