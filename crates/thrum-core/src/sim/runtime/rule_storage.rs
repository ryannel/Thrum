//! Scoped access to fixed physical column storage for the SDK's rule adapter.

use std::{any::TypeId, marker::PhantomData, ptr::NonNull};

use super::{Handle, ReadPermit, RegisteredWorldError, WorldTxn, WritePermit};

/// Erased pointer to one fixed-capacity column. Access is checked by type and
/// requires the caller to uphold the transaction's scoped loan contract.
#[derive(Clone, Copy)]
pub struct RuleStorage {
    values: NonNull<()>,
    presence: NonNull<u64>,
    len: usize,
    type_id: TypeId,
}

// SAFETY: The token is inert unless accessed through the unsafe loan methods.
// Moving or sharing it does not borrow, dereference, or mutate its storage.
#[expect(unsafe_code, reason = "inert, transaction-scoped column address")]
unsafe impl Send for RuleStorage {}
// SAFETY: As for Send. Workers may access only the disjoint rows they own.
#[expect(unsafe_code, reason = "inert, transaction-scoped column address")]
unsafe impl Sync for RuleStorage {}

/// Typed fixed-capacity column address, bound once before a rule iterates rows.
pub struct RuleColumn<T> {
    storage: RuleStorage,
    marker: PhantomData<T>,
}

impl RuleStorage {
    /// Bind this address to its registered type.
    ///
    /// # Panics
    ///
    /// The requested type is not the column's type.
    pub fn typed<T: 'static>(self) -> RuleColumn<T> {
        assert_eq!(self.type_id, TypeId::of::<T>());
        RuleColumn {
            storage: self,
            marker: PhantomData,
        }
    }
}

impl<T> RuleColumn<T> {
    /// Read a physical row.
    ///
    /// # Safety
    ///
    /// The column loan must still be in its originating callback, before any
    /// further world access. The row must have no live mutable reference for
    /// the returned borrow's lifetime.
    ///
    /// # Panics
    ///
    /// The physical row is outside the registered column.
    #[expect(
        unsafe_code,
        reason = "read a row admitted by a scoped rule projection"
    )]
    pub unsafe fn read<'a>(&self, row: usize) -> Option<&'a T> {
        // SAFETY: Structural edits cannot change column presence during a live loan.
        let cell = unsafe { self.present_row_address(row) }?;
        // SAFETY: The caller guarantees the column is still live and that
        // this physical row has no concurrent exclusive loan.
        unsafe { &*cell }.as_ref()
    }

    /// Lend a physical row mutably and capture its first preimage for this rule.
    ///
    /// # Safety
    ///
    /// The column loan must still be in its originating callback, before any
    /// further world access. The callback must hold a write permit and must
    /// own the row exclusively for the lifetime of the returned reference.
    /// Submit the preimage to the transaction after all loans have ended,
    /// including on refusal or unwind.
    ///
    /// # Panics
    ///
    /// The physical row is outside the registered column.
    #[expect(
        unsafe_code,
        reason = "lend a row admitted exclusively by a scoped rule projection"
    )]
    pub unsafe fn write<'a, P>(
        &self,
        row: usize,
        prior: &'a mut Option<P>,
        capture: impl FnOnce(&T) -> P,
    ) -> Option<&'a mut T> {
        // SAFETY: Structural edits cannot change column presence during a live loan.
        let cell = unsafe { self.present_row_address(row) }?;
        // SAFETY: The caller guarantees the column is still live and that it
        // holds the only loan of this physical row.
        let current = unsafe { &mut *cell }.as_mut()?;
        prior.get_or_insert_with(|| capture(current));
        Some(current)
    }

    #[expect(unsafe_code, reason = "read the presence bitmap of a live column")]
    unsafe fn present_row_address(&self, row: usize) -> Option<*mut Option<T>> {
        assert!(row < self.storage.len);
        let word = self.storage.presence.as_ptr().wrapping_add(row / 64);
        // SAFETY: The row bound gives a valid presence word. The caller holds
        // a live loan, during which all presence words are immutable.
        let word = unsafe { *word };
        let present = word & (1 << (row % 64)) != 0;
        // Several optional projections may share a missing component. Do
        // not create even a temporary reference to its vacant cell.
        present.then(|| {
            self.storage
                .values
                .cast::<Option<T>>()
                .as_ptr()
                .wrapping_add(row)
        })
    }
}

impl WorldTxn<'_> {
    /// Prepare a raw column address under a checked permit. Rule callbacks use
    /// read permits to bind projections and independently require a write
    /// permit for each exclusive projection.
    ///
    /// # Errors
    ///
    /// The permit belongs to another runtime or callback.
    pub fn rule_storage<T: Send + Sync + 'static>(
        &mut self,
        permit: &ReadPermit<T>,
    ) -> Result<RuleStorage, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if permit.system != self.system {
            return Err(RegisteredWorldError::WrongSystem);
        }
        let (values, presence, len) = self.world.rule_column_addresses::<T>(permit.component);
        Ok(RuleStorage {
            values,
            presence,
            len,
            type_id: TypeId::of::<T>(),
        })
    }

    /// Journal the preimages from one column after a scoped callback.
    ///
    /// # Errors
    ///
    /// The permit belongs to another runtime or callback.
    ///
    /// # Safety
    ///
    /// Each row must be a current, present handle of this callback, with its
    /// true preimage from before the loan. No column loans may still be in use.
    #[expect(
        unsafe_code,
        reason = "preimages must belong to this callback's scoped loans"
    )]
    pub unsafe fn journal_rule_column<T: PartialEq + Send + Sync + 'static>(
        &mut self,
        permit: &WritePermit<T>,
        preimages: impl IntoIterator<Item = (Handle, T)>,
        finite: impl Fn(&T) -> bool,
    ) -> Result<(), RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if permit.system != self.system {
            return Err(RegisteredWorldError::WrongSystem);
        }
        self.world
            .journal_rule_column(permit.component, preimages, self.undo, self.dirty, finite)
    }
}
