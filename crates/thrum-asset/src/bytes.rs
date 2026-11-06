use std::ops::Deref;

use bytemuck::{Pod, Zeroable};

/// Immutable product bytes whose start is at least 64-byte aligned.
///
/// Native targets memory-map the content-addressed file, so opening a product costs a page-table
/// entry rather than a read and the kernel pages data in as the GPU upload touches it. Targets
/// without files (the browser) or bytes fetched from a dev server use an owned 64-byte-aligned
/// copy. Either way, section casts never need an unaligned fallback.
pub struct ProductBytes {
    inner: Inner,
}

enum Inner {
    #[cfg(not(target_arch = "wasm32"))]
    Mapped(memmap2::Mmap),
    Owned {
        blocks: Vec<Align64>,
        len: usize,
    },
}

#[repr(C, align(64))]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Align64([u8; 64]);

impl ProductBytes {
    /// Copy bytes (for example a dev-server response) into aligned owned storage.
    pub fn copy_from(bytes: &[u8]) -> Self {
        let mut blocks = vec![Align64([0; 64]); bytes.len().div_ceil(64)];
        bytemuck::cast_slice_mut::<Align64, u8>(&mut blocks)[..bytes.len()].copy_from_slice(bytes);
        Self {
            inner: Inner::Owned {
                blocks,
                len: bytes.len(),
            },
        }
    }

    /// Map an immutable content-addressed file read-only.
    ///
    /// # Errors
    /// Returns the I/O error from opening or mapping the file.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn map_immutable(file: &std::fs::File) -> std::io::Result<Self> {
        if file.metadata()?.len() == 0 {
            // Mapping zero bytes fails on some platforms; an empty product is still a product.
            return Ok(Self::copy_from(&[]));
        }
        #[expect(
            unsafe_code,
            reason = "read-only mapping of an immutable content-addressed product file"
        )]
        // SAFETY: `Mmap::map` is unsafe because another process could modify or truncate the
        // file while it is mapped. Store files are written once under a temporary name (synced first under
        // power-loss durability), made read-only and renamed to their BLAKE3 name; the store never rewrites, appends to
        // or truncates a published name, and retirement only unlinks it (which keeps an existing
        // mapping valid on POSIX). Mapped bytes are only read through `&[u8]`.
        let map = unsafe { memmap2::Mmap::map(file)? };
        Ok(Self {
            inner: Inner::Mapped(map),
        })
    }

    /// Whether the bytes are a file mapping rather than an owned copy.
    pub fn is_mapped(&self) -> bool {
        match self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            Inner::Mapped(_) => true,
            Inner::Owned { .. } => false,
        }
    }
}

impl Deref for ProductBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            Inner::Mapped(map) => map,
            Inner::Owned { blocks, len } => &bytemuck::cast_slice::<Align64, u8>(blocks)[..*len],
        }
    }
}

impl std::fmt::Debug for ProductBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProductBytes")
            .field("len", &self.len())
            .field("mapped", &self.is_mapped())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::ProductBytes;

    #[test]
    fn owned_copy_is_aligned_and_exact() {
        let source: Vec<u8> = (0..=200).collect();
        let bytes = ProductBytes::copy_from(&source[1..]);
        assert_eq!(&bytes[..], &source[1..]);
        assert_eq!(bytes.as_ptr().align_offset(64), 0);
        assert!(!bytes.is_mapped());
        assert!(ProductBytes::copy_from(&[]).is_empty());
    }
}
