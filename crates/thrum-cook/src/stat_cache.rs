//! Stat cache: re-hash a source file only when its filesystem stamp changes.
//!
//! The stamp is length, modification time, change time and inode. An editor's atomic save
//! (write a new file, rename over the old) changes the inode even when sizes and times collide.
//! As in Git's index, an entry is *racily clean* when its file was modified within
//! [`RACY_WINDOW_NS`] of the moment it was hashed: a later write in the same timestamp tick could
//! leave the stamp unchanged, so such entries are re-hashed on the next scan until they age out.

use std::collections::{BTreeMap, HashMap};
use std::fs::Metadata;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use thrum_asset::ContentHash;

/// Window inside which an unchanged stamp is not trusted.
pub const RACY_WINDOW_NS: i128 = 1_000_000_000;

const FILE_MAGIC: &[u8; 8] = b"THRMSTAT";
const FILE_FORMAT: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified_ns: i128,
    changed_ns: i128,
    inode: u64,
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    stamp: Stamp,
    hash: ContentHash,
    hashed_ns: i128,
}

/// Counts from one scan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanCounts {
    /// Files whose stamp matched and whose hash was reused.
    pub reused: usize,
    /// Files read and hashed.
    pub hashed: usize,
    /// Files that disappeared.
    pub removed: usize,
    /// Files whose stamp changed while being hashed; they are retried by the next scan.
    pub unstable: usize,
}

/// Source-relative path to content hash, kept current by stat comparison.
#[derive(Debug, Default)]
pub struct StatCache {
    entries: HashMap<PathBuf, Entry>,
}

impl StatCache {
    /// Current hash of every tracked file, in path order.
    pub fn files(&self) -> BTreeMap<PathBuf, ContentHash> {
        self.entries
            .iter()
            .map(|(path, entry)| (path.clone(), entry.hash))
            .collect()
    }

    /// Hash of one tracked file.
    pub fn hash(&self, relative: &Path) -> Option<ContentHash> {
        self.entries.get(relative).map(|entry| entry.hash)
    }

    /// Walk `root` and bring every file's hash up to date. Stat and hash work runs on all cores.
    ///
    /// # Errors
    /// I/O failure walking the tree.
    pub fn scan_all(&mut self, root: &Path) -> std::io::Result<ScanCounts> {
        let mut found = Vec::new();
        walk(root, Path::new(""), &mut found)?;
        let removed: Vec<PathBuf> = self
            .entries
            .keys()
            .filter(|path| found.binary_search(path).is_err())
            .cloned()
            .collect();
        let mut counts = self.refresh(root, &found);
        for path in removed {
            self.entries.remove(&path);
            counts.removed += 1;
        }
        Ok(counts)
    }

    /// Refresh only the given source-relative paths (from watcher events). Missing paths are
    /// dropped; directories are walked.
    ///
    /// # Errors
    /// I/O failure walking a named directory.
    pub fn scan_paths(&mut self, root: &Path, paths: &[PathBuf]) -> std::io::Result<ScanCounts> {
        let mut found = Vec::new();
        let mut removed = 0;
        for relative in paths {
            // Match the full walk: hidden files and directories (editor temporaries, AppleDouble
            // `._` files) are never sources.
            if relative
                .components()
                .any(|part| part.as_os_str().to_string_lossy().starts_with('.'))
            {
                continue;
            }
            let absolute = root.join(relative);
            if absolute.is_dir() {
                walk(&absolute, relative, &mut found)?;
            } else if absolute.is_file() {
                found.push(relative.clone());
            } else {
                let prefix: Vec<PathBuf> = self
                    .entries
                    .keys()
                    .filter(|path| path.starts_with(relative))
                    .cloned()
                    .collect();
                for path in prefix {
                    self.entries.remove(&path);
                    removed += 1;
                }
            }
        }
        found.sort();
        found.dedup();
        let mut counts = self.refresh(root, &found);
        counts.removed += removed;
        Ok(counts)
    }

    fn refresh(&mut self, root: &Path, paths: &[PathBuf]) -> ScanCounts {
        let now = now_ns();
        let previous = &self.entries;
        let results: Vec<(PathBuf, Option<Entry>, bool)> = paths
            .par_iter()
            .map(|relative| {
                let absolute = root.join(relative);
                let Ok(stamp) = std::fs::metadata(&absolute).map(|meta| stamp(&meta)) else {
                    return (relative.clone(), None, false);
                };
                if let Some(entry) = previous.get(relative)
                    && entry.stamp == stamp
                    && entry.stamp.modified_ns < entry.hashed_ns - RACY_WINDOW_NS
                {
                    return (relative.clone(), Some(*entry), true);
                }
                match hash_stable(&absolute, stamp) {
                    Some(hash) => (
                        relative.clone(),
                        Some(Entry {
                            stamp,
                            hash,
                            hashed_ns: now,
                        }),
                        false,
                    ),
                    None => (relative.clone(), None, false),
                }
            })
            .collect();
        let mut counts = ScanCounts::default();
        for (path, entry, reused) in results {
            match entry {
                Some(entry) if reused => {
                    counts.reused += 1;
                    self.entries.insert(path, entry);
                }
                Some(entry) => {
                    counts.hashed += 1;
                    self.entries.insert(path, entry);
                }
                None => {
                    // Keep the last stable hash, if any, until the writer finishes.
                    counts.unstable += 1;
                }
            }
        }
        counts
    }

    /// Load a persisted cache; an absent or unreadable file yields an empty cache.
    pub fn load(path: &Path) -> Self {
        let mut bytes = Vec::new();
        let read = std::fs::File::open(path).and_then(|mut file| file.read_to_end(&mut bytes));
        read.ok().and_then(|_| decode(&bytes)).unwrap_or_default()
    }

    /// Persist the cache atomically (temporary file then rename).
    ///
    /// # Errors
    /// I/O failure.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut out = Vec::with_capacity(16 + self.entries.len() * 96);
        out.extend_from_slice(FILE_MAGIC);
        out.extend_from_slice(&FILE_FORMAT.to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(self.entries.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        for (relative, entry) in &self.entries {
            let text = relative.to_string_lossy();
            out.extend_from_slice(&u32::try_from(text.len()).unwrap_or(u32::MAX).to_le_bytes());
            out.extend_from_slice(text.as_bytes());
            out.extend_from_slice(&entry.stamp.len.to_le_bytes());
            out.extend_from_slice(&entry.stamp.modified_ns.to_le_bytes());
            out.extend_from_slice(&entry.stamp.changed_ns.to_le_bytes());
            out.extend_from_slice(&entry.stamp.inode.to_le_bytes());
            out.extend_from_slice(&entry.hashed_ns.to_le_bytes());
            out.extend_from_slice(entry.hash.as_bytes());
        }
        let temporary = path.with_extension("tmp");
        std::fs::File::create(&temporary)?.write_all(&out)?;
        std::fs::rename(temporary, path)
    }
}

fn decode(bytes: &[u8]) -> Option<StatCache> {
    let mut reader = Cursor { bytes, at: 0 };
    if reader.take(8)? != FILE_MAGIC || reader.u32()? != FILE_FORMAT {
        return None;
    }
    let count = reader.u32()?;
    let mut entries = HashMap::with_capacity(count.min(1 << 20) as usize);
    for _ in 0..count {
        let length = reader.u32()? as usize;
        let path = PathBuf::from(std::str::from_utf8(reader.take(length)?).ok()?);
        let stamp = Stamp {
            len: reader.u64()?,
            modified_ns: reader.i128()?,
            changed_ns: reader.i128()?,
            inode: reader.u64()?,
        };
        let hashed_ns = reader.i128()?;
        let hash = ContentHash::from_bytes(reader.take(32)?.try_into().ok()?);
        entries.insert(
            path,
            Entry {
                stamp,
                hash,
                hashed_ns,
            },
        );
    }
    (reader.at == bytes.len()).then_some(StatCache { entries })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let slice = self.bytes.get(self.at..self.at.checked_add(length)?)?;
        self.at += length;
        Some(slice)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn i128(&mut self) -> Option<i128> {
        Some(i128::from_le_bytes(self.take(16)?.try_into().ok()?))
    }
}

/// Hash a file and confirm its stamp did not change while it was read.
fn hash_stable(path: &Path, before: Stamp) -> Option<ContentHash> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    if bytes.len() >= 1 << 20 {
        hasher.update_rayon(&bytes);
    } else {
        hasher.update(&bytes);
    }
    let after = std::fs::metadata(path).ok().map(|meta| stamp(&meta))?;
    (after == before && bytes.len() as u64 == before.len)
        .then(|| ContentHash::from_bytes(*hasher.finalize().as_bytes()))
}

fn walk(directory: &Path, relative: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let start = out.len();
    for item in std::fs::read_dir(directory)? {
        let item = item?;
        let name = item.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let kind = item.file_type()?;
        let child = relative.join(&name);
        if kind.is_dir() {
            walk(&item.path(), &child, out)?;
        } else if kind.is_file() {
            out.push(child);
        }
    }
    out[start..].sort();
    if relative.as_os_str().is_empty() {
        out.sort();
    }
    Ok(())
}

fn now_ns() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos().try_into().unwrap_or(i128::MAX))
}

#[cfg(unix)]
fn stamp(meta: &Metadata) -> Stamp {
    use std::os::unix::fs::MetadataExt;
    Stamp {
        len: meta.len(),
        modified_ns: i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec()),
        changed_ns: i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec()),
        inode: meta.ino(),
    }
}

#[cfg(not(unix))]
fn stamp(meta: &Metadata) -> Stamp {
    let modified = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |time| time.as_nanos().try_into().unwrap_or(i128::MAX));
    Stamp {
        len: meta.len(),
        modified_ns: modified,
        changed_ns: modified,
        inode: 0,
    }
}
