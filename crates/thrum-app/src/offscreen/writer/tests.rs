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
        let writer = ImageWriter::<Vec<u8>>::start(sink, 2, allowance).unwrap();
        assert!(writer.submit(vec![1; 8]).is_none());
        assert!(
            writer
                .finish(Duration::from_secs(1))
                .unwrap_err()
                .contains(expected)
        );
    }
}
