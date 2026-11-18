/// Failure to decode canonical authority bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateDecodeError {
    /// The input ended before the complete value.
    Truncated,
    /// A value was outside its canonical range or failed validation.
    Invalid,
    /// The encoded input or temporary storage exceeds the caller's limit.
    Limit,
}

impl std::fmt::Display for StateDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Truncated => "truncated authority bytes",
            Self::Invalid => "invalid authority value",
            Self::Limit => "authority byte limit exceeded",
        })
    }
}

impl std::error::Error for StateDecodeError {}

/// Cursor over bounded authority bytes. Codecs that allocate temporary or
/// retained heap storage must charge its complete size before allocation.
pub struct StateReader<'a> {
    bytes: &'a [u8],
    budget: usize,
}

impl<'a> StateReader<'a> {
    pub(crate) fn new(bytes: &'a [u8], budget: usize) -> Result<Self, StateDecodeError> {
        if bytes.len() > budget {
            return Err(StateDecodeError::Limit);
        }
        Ok(Self { bytes, budget })
    }

    /// Read a fixed-width field and advance the cursor.
    ///
    /// # Errors
    /// The input has fewer than N bytes remaining.
    pub fn read<const N: usize>(&mut self) -> Result<[u8; N], StateDecodeError> {
        let (value, rest) = self
            .bytes
            .split_first_chunk()
            .ok_or(StateDecodeError::Truncated)?;
        self.bytes = rest;
        Ok(*value)
    }

    /// Charge a prospective allocation including its elements but excluding allocator overhead.
    ///
    /// # Errors
    /// The allocation would exceed the total heap budget for the decode.
    pub fn charge_allocation(&mut self, bytes: usize) -> Result<(), StateDecodeError> {
        self.budget = self
            .budget
            .checked_sub(bytes)
            .ok_or(StateDecodeError::Limit)?;
        Ok(())
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len()
    }
}
