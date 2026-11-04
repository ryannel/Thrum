//! Reusable immutable endpoint leases. The game supplies page data; the engine
//! owns bounded page reuse and refuses publication while every page is leased.

use std::{mem::size_of, sync::Arc};

const PAGE_COUNT: usize = 4;

fn reusable<T>(page: &Arc<T>) -> bool {
    Arc::strong_count(page) == 1 && Arc::weak_count(page) == 0
}

/// Every preallocated page is held by a consumer or retained as a recent endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicationBusy;

/// Bounded pool of reusable committed pages. A capture mutates only an unleased
/// page, then publishes it atomically as the latest endpoint.
pub struct SnapshotRing<T> {
    pages: Vec<Arc<T>>,
    cursor: usize,
    latest: Option<Arc<T>>,
    previous: Option<Arc<T>>,
}

impl<T> SnapshotRing<T> {
    /// Prepare four page payloads before ticking; `make` is called once per slot.
    /// Two pages retain adjacent endpoints and two remain available for capture
    /// or external leases.
    pub fn new(mut make: impl FnMut() -> T) -> Self {
        Self {
            pages: (0..PAGE_COUNT).map(|_| Arc::new(make())).collect(),
            cursor: 0,
            latest: None,
            previous: None,
        }
    }

    /// Whether an unleased page exists for the next complete publication.
    pub fn can_capture(&self) -> bool {
        self.pages.iter().any(reusable)
    }

    /// Fill and publish one complete page. Validate the candidate before this
    /// call; a busy refusal does not invoke `fill` or change the current page.
    ///
    /// # Errors
    ///
    /// Every page is leased, so the due tick must be retried after a lease drops.
    pub fn capture(&mut self, fill: impl FnOnce(&mut T, bool)) -> Result<(), PublicationBusy> {
        let index = (0..self.pages.len())
            .map(|offset| (self.cursor + offset) % self.pages.len())
            .find(|&index| reusable(&self.pages[index]))
            .ok_or(PublicationBusy)?;
        let first = self.latest.is_none();
        let page = Arc::get_mut(&mut self.pages[index]).ok_or(PublicationBusy)?;
        fill(page, first);
        self.cursor = (index + 1) % self.pages.len();
        self.previous = self.latest.take();
        self.latest = Some(Arc::clone(&self.pages[index]));
        Ok(())
    }

    /// Lease the latest complete endpoint, if one has been published.
    pub fn latest(&self) -> Option<Arc<T>> {
        self.latest.clone()
    }

    /// Lease the endpoint immediately before latest, if available.
    pub fn previous(&self) -> Option<Arc<T>> {
        self.previous.clone()
    }

    /// Reserved payload and ring storage for diagnostics. `payload_bytes`
    /// reports one page including its nested allocations.
    pub fn reserved_bytes(&self, payload_bytes: impl Fn(&T) -> usize) -> usize {
        size_of::<Self>()
            + self.pages.capacity() * size_of::<Arc<T>>()
            + self
                .pages
                .iter()
                .map(|page| payload_bytes(page) + 2 * size_of::<usize>())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{PublicationBusy, SnapshotRing};

    #[test]
    fn held_pages_refuse_without_calling_fill_and_resume_after_release() {
        let mut ring = SnapshotRing::new(|| 0u64);
        ring.capture(|page, first| {
            assert!(first);
            *page = 1;
        })
        .expect("first page");
        let first = ring.latest().expect("first lease");
        ring.capture(|page, first| {
            assert!(!first);
            *page = 2;
        })
        .expect("second page");
        let second = ring.latest().expect("second lease");
        ring.capture(|page, _| *page = 3).expect("third page");
        ring.capture(|page, _| *page = 4).expect("fourth page");
        assert_eq!(*first, 1);
        assert_eq!(*second, 2);
        assert_eq!(
            ring.capture(|_, _| panic!("must not fill")),
            Err(PublicationBusy)
        );
        drop(first);
        ring.capture(|page, _| *page = 5)
            .expect("released page reusable");
        assert_eq!(*second, 2);
        assert_eq!(*ring.latest().expect("latest"), 5);
    }

    #[test]
    fn weak_lease_does_not_block_later_reusable_page_or_claim_false_readiness() {
        let mut ring = SnapshotRing::new(|| 0u64);
        ring.capture(|page, _| *page = 1).expect("first page");
        let weak = Arc::downgrade(&ring.latest().expect("first lease"));
        ring.capture(|page, _| *page = 2).expect("second page");
        let second = ring.latest().expect("second lease");
        ring.capture(|page, _| *page = 3).expect("third page");
        ring.capture(|page, _| *page = 4).expect("fourth page");
        assert!(!ring.can_capture(), "weak first page must not be selected");
        assert_eq!(
            ring.capture(|_, _| panic!("must not fill")),
            Err(PublicationBusy)
        );
        drop(weak);
        assert!(ring.can_capture());
        ring.capture(|page, _| *page = 5)
            .expect("first page is now reusable");
        assert_eq!(*second, 2);
    }
}
