//! Bounded server-side result admission frontier. Frame/journal ACK can retire
//! before a pending lower input resolves, but authority history cannot.

/// Maximum individually admitted results retained above a pending gap.
pub const MAX_ADMITTED_HIGHER: usize = 256;

/// Exact authority/result-history scope. The session owner must compare this
/// to the current authoritative peer incarnation before each ACK callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerKey {
    /// Match-scoped identity, never the zero initial-Hello value.
    pub match_id: [u8; 16],
    /// Authority peer ID within this match.
    pub peer_id: u64,
    /// Nonzero connection epoch allocated uniquely within the match.
    pub connection_epoch: u64,
}

/// Refusal while staging or completing one client-admitted result receipt.
#[derive(Debug, PartialEq, Eq)]
pub enum RetirementError<E> {
    /// Peer, match or connection epoch differs from the retained frontier.
    WrongIdentity,
    /// A result ID is zero or outside the finite sequence window.
    InvalidIdentity,
    /// The admitted-higher candidate exceeds its fixed capacity.
    Capacity,
    /// A previous exact receipt still awaits authority disposition.
    ReceiptOccupied,
    /// No exact receipt is staged for completion.
    NoReceipt,
    /// An authority callback unwound after an uncertain acknowledgement.
    Poisoned,
    /// Authority refused without changing its own result history.
    Authority(E),
}

/// One peer/connection's authority-retired prefix, copied admitted-higher
/// identities and one occupied exact receipt. All work arrays are reserved.
pub struct AuthorityRetirement {
    key: PeerKey,
    retired_through: u64,
    higher: Vec<u64>,
    candidate: Vec<u64>,
    receipt: Vec<u64>,
    receipt_occupied: bool,
    authority_call_in_progress: bool,
}

impl AuthorityRetirement {
    /// Start from a verified same-epoch authority-retired prefix. A new
    /// peer/connection incarnation requires a new instance and baseline.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identity, retirement ordering or capacity.
    pub fn new(key: PeerKey, retired_through: u64) -> Result<Self, RetirementError<()>> {
        if key.connection_epoch == 0 || key.match_id == [0; 16] {
            return Err(RetirementError::WrongIdentity);
        }
        Ok(Self {
            key,
            retired_through,
            higher: Vec::with_capacity(MAX_ADMITTED_HIGHER),
            candidate: Vec::with_capacity(MAX_ADMITTED_HIGHER),
            receipt: Vec::with_capacity(MAX_ADMITTED_HIGHER),
            receipt_occupied: false,
            authority_call_in_progress: false,
        })
    }

    /// Last prefix actually acknowledged in the authoritative result owner.
    pub fn retired_through(&self) -> u64 {
        self.retired_through
    }
    /// Individually client-admitted results still above a gap.
    pub fn admitted_higher_count(&self) -> usize {
        self.higher.len()
    }
    /// Whether this exact result was already admitted by the client. A sent
    /// owner can use this with its pending journal to refuse duplicate work.
    pub fn is_admitted(&self, id: u64) -> bool {
        id != 0 && (id <= self.retired_through || self.higher.binary_search(&id).is_ok())
    }
    /// Whether one exact ACK receipt is still awaiting authority disposition.
    pub fn receipt_occupied(&self) -> bool {
        self.receipt_occupied
    }
    /// Reserved Rust value/Vec capacity bytes, excluding allocator metadata.
    pub fn reserved_capacity_bytes(&self) -> usize {
        size_of::<Self>()
            + (self.higher.capacity() + self.candidate.capacity() + self.receipt.capacity())
                * size_of::<u64>()
    }

    /// Copy one exact result-ID receipt after a validated ledger admission
    /// ACK. This helper does not authenticate the wire ACK itself. A refusal
    /// never overwrites an already occupied receipt or public frontier.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identity, retirement ordering or capacity.
    pub fn stage(&mut self, key: PeerKey, ids: &[u64]) -> Result<(), RetirementError<()>> {
        if self.authority_call_in_progress {
            return Err(RetirementError::Poisoned);
        }
        if key != self.key {
            return Err(RetirementError::WrongIdentity);
        }
        if self.receipt_occupied {
            return Err(RetirementError::ReceiptOccupied);
        }
        if ids.len() > MAX_ADMITTED_HIGHER {
            return Err(RetirementError::Capacity);
        }
        self.candidate.clear();
        self.candidate.extend_from_slice(&self.higher);
        for &id in ids {
            if id == 0 {
                return Err(RetirementError::InvalidIdentity);
            }
            if id <= self.retired_through {
                continue; // Idempotent baseline inclusion or duplicate ACK.
            }
            if id - self.retired_through > MAX_ADMITTED_HIGHER as u64 {
                return Err(RetirementError::InvalidIdentity);
            }
            match self.candidate.binary_search(&id) {
                Ok(_) => {}
                Err(at) => {
                    if self.candidate.len() == MAX_ADMITTED_HIGHER {
                        return Err(RetirementError::Capacity);
                    }
                    self.candidate.insert(at, id);
                }
            }
        }
        self.receipt.clear();
        self.receipt.extend_from_slice(ids);
        self.receipt_occupied = true;
        Ok(())
    }

    /// Complete the occupied receipt. The callback must first verify that
    /// the authority's *current* match, peer and epoch equal `key`, then call
    /// its contiguous-prefix ACK. It must leave authority unchanged on Err.
    /// An unwind poisons this helper and requires peer teardown. On ordinary
    /// callback error the copied receipt stays occupied for retry.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identity, retirement ordering or capacity.
    pub fn finish<E>(
        &mut self,
        key: PeerKey,
        acknowledge_authority: impl FnOnce(PeerKey, u64) -> Result<(), E>,
    ) -> Result<u64, RetirementError<E>> {
        if self.authority_call_in_progress {
            return Err(RetirementError::Poisoned);
        }
        if key != self.key {
            return Err(RetirementError::WrongIdentity);
        }
        if !self.receipt_occupied {
            return Err(RetirementError::NoReceipt);
        }
        let mut next = self.retired_through;
        let mut prefix = 0;
        for &id in &self.candidate {
            if next.checked_add(1) != Some(id) {
                break;
            }
            next = id;
            prefix += 1;
        }
        if prefix != 0 {
            self.authority_call_in_progress = true;
            if let Err(error) = acknowledge_authority(key, next) {
                self.authority_call_in_progress = false;
                return Err(RetirementError::Authority(error));
            }
        }
        self.candidate.drain(..prefix);
        std::mem::swap(&mut self.higher, &mut self.candidate);
        self.candidate.clear();
        self.receipt.clear();
        self.receipt_occupied = false;
        self.retired_through = next;
        self.authority_call_in_progress = false;
        Ok(next)
    }
}
