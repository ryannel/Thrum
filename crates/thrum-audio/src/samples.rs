use std::{
    cell::UnsafeCell,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::Error;

pub(crate) struct Clip {
    samples: Arc<[i16]>,
    pub(crate) frames: u32,
    pub(crate) channels: u8,
}

impl Clip {
    pub(crate) fn new(samples: Arc<[i16]>, channels: u8) -> Result<Self, Error> {
        if !(1..=2).contains(&channels) {
            return Err(Error::Channels);
        }
        let channels_count = usize::from(channels);
        if samples.is_empty() {
            return Err(Error::EmptySound);
        }
        if !samples.len().is_multiple_of(channels_count) {
            return Err(Error::Frames);
        }
        let frames = u32::try_from(samples.len() / channels_count).map_err(|_| Error::Frames)?;
        Ok(Self {
            samples,
            frames,
            channels,
        })
    }

    pub(crate) fn sample(&self, frame: u32, channel: usize) -> f32 {
        let channels = usize::from(self.channels);
        f32::from(self.samples[frame as usize * channels + channel.min(channels - 1)])
            * (1.0 / 32_768.0)
    }
}

struct Slot {
    clip: UnsafeCell<Option<Clip>>,
    pins: AtomicUsize,
}

impl Slot {
    fn new() -> Self {
        Self {
            clip: UnsafeCell::new(None),
            pins: AtomicUsize::new(0),
        }
    }
}

#[expect(
    unsafe_code,
    reason = "one producer publishes immutable PCM, which is retired off the mix thread"
)]
mod bank {
    use super::{Arc, Clip, Ordering, Slot};

    // SAFETY: Only Samples writes a slot. It publishes before minting a lease and never changes the
    // slot until all leases have been dropped. Release/Acquire on pins separates readers from reuse.
    unsafe impl Sync for Slot {}

    pub(crate) struct Bank(Box<[Slot]>);

    pub(crate) struct Lease {
        bank: Arc<Bank>,
        index: usize,
    }

    impl Lease {
        pub(crate) fn clip(&self) -> &Clip {
            let slot = &self.bank.0[self.index];
            // SAFETY: This lease pins the slot for the lifetime of the returned borrow. Its
            // publication preceded the lease and is visible through the command queue.
            unsafe { &*slot.clip.get() }
                .as_ref()
                .expect("a pinned slot contains a clip")
        }
    }

    impl Drop for Lease {
        fn drop(&mut self) {
            self.bank.0[self.index].pins.fetch_sub(1, Ordering::Release);
        }
    }

    pub(crate) struct Samples {
        bank: Arc<Bank>,
        free: Vec<usize>,
        retired: Vec<usize>,
    }

    impl Samples {
        pub(crate) fn new(capacity: usize) -> Self {
            Self {
                bank: Arc::new(Bank((0..capacity).map(|_| Slot::new()).collect())),
                free: (0..capacity).rev().collect(),
                retired: Vec::with_capacity(capacity),
            }
        }

        pub(crate) fn bank(&self) -> Arc<Bank> {
            Arc::clone(&self.bank)
        }

        pub(crate) fn add(&mut self, clip: Clip) -> usize {
            self.collect();
            let index = self
                .free
                .pop()
                .expect("capacity covers every possible live lease");
            // SAFETY: The single producer owns all free slots. This one has no outstanding lease.
            unsafe { *self.bank.0[index].clip.get() = Some(clip) };
            index
        }

        pub(crate) fn lease(&self, index: usize) -> Lease {
            self.bank.0[index].pins.fetch_add(1, Ordering::Relaxed);
            Lease {
                bank: Arc::clone(&self.bank),
                index,
            }
        }

        pub(crate) fn retire(&mut self, index: usize) {
            self.retired.push(index);
            self.collect();
        }

        pub(crate) fn collect(&mut self) {
            self.retired.retain(|&index| {
                let slot = &self.bank.0[index];
                if slot.pins.load(Ordering::Acquire) != 0 {
                    return true;
                }
                // SAFETY: A retired slot cannot be leased again. The Acquire load has observed
                // the end of every outstanding lease. Only this producer mutates the slot.
                unsafe { *slot.clip.get() = None };
                self.free.push(index);
                false
            });
        }
    }
}

pub(crate) use bank::{Bank, Lease, Samples};
