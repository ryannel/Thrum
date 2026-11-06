//! Constant-time structural validation of a Thrum container's header and section table.

use bytemuck::Pod;

use crate::AssetError;
use crate::layout::{CONTAINER_FORMAT, ContainerHeader, SECTION_ALIGN, Section};

const MAX_SECTIONS: usize = 32;

/// Validated section table of one container.
pub(crate) struct Container<'a> {
    bytes: &'a [u8],
    sections: &'a [Section],
}

impl<'a> Container<'a> {
    /// Check header, version, length, section alignment, bounds and ordering.
    pub(crate) fn parse(bytes: &'a [u8], magic: [u8; 8]) -> Result<Self, AssetError> {
        let header_len = size_of::<ContainerHeader>();
        let header: &ContainerHeader = bytes
            .get(..header_len)
            .and_then(|head| bytemuck::try_from_bytes(head).ok())
            .ok_or(AssetError::Malformed("container header"))?;
        if header.magic != magic {
            return Err(AssetError::Malformed("container magic"));
        }
        if header.format != CONTAINER_FORMAT {
            return Err(AssetError::UnsupportedVersion(header.format));
        }
        if usize::try_from(header.total_len).ok() != Some(bytes.len()) {
            return Err(AssetError::Malformed("container length"));
        }
        let count = usize::try_from(header.section_count)
            .map_err(|_| AssetError::Malformed("section count"))?;
        if count > MAX_SECTIONS {
            return Err(AssetError::Malformed("section count"));
        }
        let table_end = header_len + count * size_of::<Section>();
        let sections: &[Section] = bytes
            .get(header_len..table_end)
            .and_then(|table| bytemuck::try_cast_slice(table).ok())
            .ok_or(AssetError::Malformed("section table"))?;
        let mut cursor = table_end as u64;
        for section in sections {
            let length = u64::from(section.element_size)
                .checked_mul(section.count)
                .ok_or(AssetError::Malformed("section length"))?;
            let end = section
                .offset
                .checked_add(length)
                .ok_or(AssetError::Malformed("section end"))?;
            if section.offset % SECTION_ALIGN as u64 != 0
                || section.offset < cursor
                || end > header.total_len
            {
                return Err(AssetError::Malformed("section placement"));
            }
            cursor = end;
        }
        Ok(Self { bytes, sections })
    }

    /// Typed slice of the unique section `tag`, whose element size must equal `size_of::<T>()`.
    pub(crate) fn slice<T: Pod>(&self, tag: [u8; 4]) -> Result<&'a [T], AssetError> {
        let mut found = self.sections.iter().filter(|section| section.tag == tag);
        let section = found
            .next()
            .ok_or(AssetError::Malformed("missing section"))?;
        if found.next().is_some() {
            return Err(AssetError::Malformed("duplicate section"));
        }
        if section.element_size as usize != size_of::<T>() {
            return Err(AssetError::Malformed("section element size"));
        }
        // Placement was checked against the total length in `parse`.
        let start =
            usize::try_from(section.offset).map_err(|_| AssetError::Malformed("section offset"))?;
        let count =
            usize::try_from(section.count).map_err(|_| AssetError::Malformed("section count"))?;
        let bytes = &self.bytes[start..start + count * size_of::<T>()];
        bytemuck::try_cast_slice(bytes).map_err(|_| AssetError::Malformed("section alignment"))
    }

    /// The single element of section `tag`.
    pub(crate) fn one<T: Pod>(&self, tag: [u8; 4]) -> Result<&'a T, AssetError> {
        match self.slice::<T>(tag)? {
            [value] => Ok(value),
            _ => Err(AssetError::Malformed("section must hold one element")),
        }
    }
}
