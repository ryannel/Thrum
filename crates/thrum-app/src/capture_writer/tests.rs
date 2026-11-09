use super::*;
use std::{io, sync::mpsc};

struct HeldSink {
    entered: mpsc::Sender<()>,
    release: Receiver<()>,
}

impl Write for HeldSink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn slow_output_has_finite_leases_nonblocking_refusal_and_a_finalization_deadline() {
    let (entered, observed) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let writer = ImageWriter::<Vec<u8>>::start(
        HeldSink {
            entered,
            release: held,
        },
        2,
        1024,
        SpoolEncoding::Raw,
    )
    .unwrap();
    assert!(writer.submit(vec![1; 8]).is_none());
    observed.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(writer.submit(vec![2; 8]).is_none());
    assert!(writer.submit(vec![3; 8]).is_none());
    // The first CPU lease remains with the blocked I/O, two are queued, and
    // the fourth stays with the caller. No session-sized memory accumulation.
    assert_eq!(writer.submit(vec![4; 8]), Some(vec![4; 8]));
    assert!(writer.recycled().is_none());
    let begin = Instant::now();
    assert!(
        writer
            .finish(Duration::from_millis(5))
            .unwrap_err()
            .contains("timeout")
    );
    assert!(begin.elapsed() < Duration::from_secs(1));
    for _ in 0..3 {
        release.send(()).unwrap();
    }
    for _ in 0..2 {
        observed.recv_timeout(Duration::from_secs(1)).unwrap();
    }
}

struct RefusedSink {
    flush_failure: bool,
}
impl Write for RefusedSink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.flush_failure {
            Ok(data.len())
        } else {
            Err(io::Error::other("disk write refused"))
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush refused"))
    }
}

#[test]
fn write_flush_and_output_limit_failures_cannot_finalize_successfully() {
    for (sink, allowance, expected) in [
        (
            RefusedSink {
                flush_failure: false,
            },
            1024,
            "disk write refused",
        ),
        (
            RefusedSink {
                flush_failure: true,
            },
            1024,
            "flush refused",
        ),
        (
            RefusedSink {
                flush_failure: true,
            },
            7,
            "allowance exhausted",
        ),
    ] {
        for encoding in [SpoolEncoding::Raw, SpoolEncoding::ZstdDelta] {
            let writer = ImageWriter::<Vec<u8>>::start(
                RefusedSink {
                    flush_failure: sink.flush_failure,
                },
                2,
                allowance,
                encoding,
            )
            .unwrap();
            assert!(writer.submit(vec![1; 8]).is_none());
            assert!(
                writer
                    .finish(Duration::from_secs(1))
                    .unwrap_err()
                    .contains(expected)
            );
        }
    }
}

struct MemorySink(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl Write for MemorySink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn lossless_spool_reconstructs_every_byte_and_hash_and_refuses_changed_record_sizes() {
    let saved = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let originals: Vec<Vec<u8>> = (0..5)
        .map(|frame| {
            (0..8193)
                .map(|i| u8::try_from((i * 17 + frame * 11) % 256).unwrap())
                .collect()
        })
        .collect();
    let writer = ImageWriter::start(
        MemorySink(std::sync::Arc::clone(&saved)),
        8,
        1024 * 1024,
        SpoolEncoding::ZstdDelta,
    )
    .unwrap();
    for image in &originals {
        assert!(writer.submit(image.clone()).is_none());
    }
    let report = writer.finish(Duration::from_secs(5)).unwrap();
    let encoded = saved.lock().unwrap();
    assert_eq!(report.encoded_bytes, encoded.len() as u64);
    let delta = zstd::stream::decode_all(&encoded[..]).unwrap();
    let mut previous = vec![0_u8; 8193];
    for ((bytes, original), hash) in delta
        .as_chunks::<8193>()
        .0
        .iter()
        .zip(&originals)
        .zip(report.frame_hashes)
    {
        for (p, d) in previous.iter_mut().zip(bytes) {
            *p ^= *d;
        }
        assert_eq!(&previous, original);
        assert_eq!(hash, format!("{:x}", Sha256::digest(original)));
    }
    assert!(zstd::stream::decode_all(&encoded[..encoded.len() / 2]).is_err());
    drop(encoded);
    let writer =
        ImageWriter::start(MemorySink(saved), 8, 1024 * 1024, SpoolEncoding::ZstdDelta).unwrap();
    assert!(writer.submit(vec![1; 8]).is_none());
    assert!(writer.submit(vec![2; 9]).is_none());
    assert!(
        writer
            .finish(Duration::from_secs(5))
            .unwrap_err()
            .contains("size changed")
    );
}

#[test]
fn live_encoding_reports_the_disk_error_after_its_io_channel_closes() {
    // More than two encoded chunks forces the codec to observe disconnection,
    // as opposed to a small stream whose only error occurs in the final flush.
    let mut state = 17_u64;
    let pixels: Vec<u8> = (0..12 * 1024 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect();
    let writer = ImageWriter::start(
        RefusedSink {
            flush_failure: false,
        },
        2,
        32 * 1024 * 1024,
        SpoolEncoding::ZstdDelta,
    )
    .unwrap();
    assert!(writer.submit(pixels).is_none());
    assert_eq!(
        writer.finish(Duration::from_secs(10)).unwrap_err(),
        "disk write refused"
    );
}
