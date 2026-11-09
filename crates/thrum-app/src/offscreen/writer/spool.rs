//! Bounded lossless temporal-XOR intermediate; no rendering or timestamp policy.
use super::SpoolEncoding;
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

pub(super) struct Metered<W> {
    output: W,
    remaining: u64,
    pub(super) bytes: u64,
    pub(super) io_time: Duration,
}
impl<W: Write> Write for Metered<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other("spool output-byte allowance exhausted"));
        }
        let begin = Instant::now();
        let result = self.output.write(bytes);
        self.io_time += begin.elapsed();
        if let Ok(n) = result {
            self.bytes += n as u64;
            self.remaining -= n as u64;
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        let begin = Instant::now();
        let result = self.output.flush();
        self.io_time += begin.elapsed();
        result
    }
}

pub(super) enum Spool<W: Write> {
    Raw(Metered<W>),
    Delta {
        encoder: zstd::stream::write::Encoder<'static, Metered<W>>,
        previous: Vec<u8>,
        delta: Vec<u8>,
    },
}
impl<W: Write> Spool<W> {
    pub(super) fn new(output: W, allowance: u64, encoding: SpoolEncoding) -> io::Result<Self> {
        let output = Metered {
            output,
            remaining: allowance,
            bytes: 0,
            io_time: Duration::ZERO,
        };
        Ok(match encoding {
            SpoolEncoding::Raw => Self::Raw(output),
            SpoolEncoding::ZstdDelta => {
                let mut encoder = zstd::stream::write::Encoder::new(output, 1)?;
                encoder.multithread(2)?;
                encoder.window_log(22)?;
                encoder.include_checksum(true)?;
                Self::Delta {
                    encoder,
                    previous: Vec::new(),
                    delta: Vec::new(),
                }
            }
        })
    }
    pub(super) fn record(&mut self, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::Raw(output) => output.write_all(bytes),
            Self::Delta {
                encoder,
                previous,
                delta,
            } => {
                if previous.is_empty() {
                    previous.resize(bytes.len(), 0);
                    delta.resize(bytes.len(), 0);
                }
                if previous.len() != bytes.len() {
                    return Err(io::Error::other("capture record size changed"));
                }
                for ((previous, delta), current) in
                    previous.iter_mut().zip(delta.iter_mut()).zip(bytes)
                {
                    *delta = *previous ^ *current;
                    *previous = *current;
                }
                encoder.write_all(delta)
            }
        }
    }
    pub(super) fn finish(self) -> io::Result<Metered<W>> {
        let mut output = match self {
            Self::Raw(output) => output,
            Self::Delta { encoder, .. } => encoder.finish()?,
        };
        output.flush()?;
        Ok(output)
    }
}
