//! Byte-bounded LSB-first fields and canonical unsigned LEB128 values.

use super::DeltaError;

pub(super) struct Writer<'a> {
    bytes: &'a mut [u8],
    bit: usize,
}

impl<'a> Writer<'a> {
    pub(super) fn new(bytes: &'a mut [u8], bit: usize) -> Self {
        bytes.fill(0);
        Self { bytes, bit }
    }

    pub(super) fn position(&self) -> usize {
        self.bit / 8
    }

    pub(super) fn put_bits(&mut self, mut value: u64, width: u8) -> Result<(), DeltaError> {
        if width == 0 || width > 64 || (width < 64 && value >> width != 0) {
            return Err(DeltaError::FieldRange);
        }
        let end = self
            .bit
            .checked_add(usize::from(width))
            .filter(|end| *end <= self.bytes.len().saturating_mul(8))
            .ok_or(DeltaError::DestinationTooSmall)?;
        let mut byte = self.bit / 8;
        let mut shift = self.bit % 8;
        let mut remaining = usize::from(width);
        while remaining != 0 {
            let count = remaining.min(8 - shift);
            let mask = (1u16 << count) - 1;
            let low = u8::try_from(value & u64::from(mask)).expect("at most eight bits");
            self.bytes[byte] |= low << shift;
            value >>= count;
            remaining -= count;
            byte += 1;
            shift = 0;
        }
        self.bit = end;
        Ok(())
    }

    pub(super) fn align(&mut self) -> Result<(), DeltaError> {
        self.bit = self
            .bit
            .checked_add(7)
            .ok_or(DeltaError::DestinationTooSmall)?
            & !7;
        if self.bit > self.bytes.len().saturating_mul(8) {
            return Err(DeltaError::DestinationTooSmall);
        }
        Ok(())
    }

    pub(super) fn put_var(&mut self, mut value: u64) -> Result<(), DeltaError> {
        if !self.bit.is_multiple_of(8) {
            return Err(DeltaError::Padding);
        }
        loop {
            let mut byte = u8::try_from(value & 0x7f).expect("seven bits");
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.put_bits(u64::from(byte), 8)?;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }
}

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8], bit: usize) -> Self {
        Self { bytes, bit }
    }

    pub(super) fn position(&self) -> usize {
        self.bit / 8
    }

    pub(super) fn get_bits(&mut self, width: u8) -> Result<u64, DeltaError> {
        if width == 0 || width > 64 {
            return Err(DeltaError::Schema);
        }
        let end = self
            .bit
            .checked_add(usize::from(width))
            .filter(|end| *end <= self.bytes.len().saturating_mul(8))
            .ok_or(DeltaError::Truncated)?;
        let mut byte = self.bit / 8;
        let mut shift = self.bit % 8;
        let mut remaining = usize::from(width);
        let mut output = 0u64;
        let mut written = 0;
        while remaining != 0 {
            let count = remaining.min(8 - shift);
            let mask = (1u16 << count) - 1;
            let low =
                u64::from((self.bytes[byte] >> shift) & u8::try_from(mask).expect("eight bits"));
            output |= low << written;
            remaining -= count;
            written += count;
            byte += 1;
            shift = 0;
        }
        self.bit = end;
        Ok(output)
    }

    pub(super) fn align(&mut self) -> Result<(), DeltaError> {
        let aligned = self.bit.checked_add(7).ok_or(DeltaError::Truncated)? & !7;
        if aligned > self.bytes.len().saturating_mul(8) {
            return Err(DeltaError::Truncated);
        }
        while self.bit < aligned {
            if self.get_bits(1)? != 0 {
                return Err(DeltaError::Padding);
            }
        }
        Ok(())
    }

    pub(super) fn get_var(&mut self) -> Result<u64, DeltaError> {
        if !self.bit.is_multiple_of(8) {
            return Err(DeltaError::Padding);
        }
        let mut value = 0u64;
        for index in 0..10 {
            let byte = u8::try_from(self.get_bits(8)?).expect("eight bits");
            let bits = u64::from(byte & 0x7f);
            if index == 9 && bits > 1 {
                return Err(DeltaError::Varint);
            }
            value |= bits << (index * 7);
            if byte & 0x80 == 0 {
                if index != 0 && bits == 0 {
                    return Err(DeltaError::Varint);
                }
                return Ok(value);
            }
        }
        Err(DeltaError::Varint)
    }
}
