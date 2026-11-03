//! Test-only native SQLite I/O observer. No production unsafe boundary.
//!
//! Each observer registers a unique, non-default VFS. Native file layout and
//! every operation except write/sync/truncate/close stay unchanged. The copied
//! method table lives in the registry until its file closes. Native callbacks
//! run outside the registry lock; poisoned locks are recovered without panic.
//! Drop unregisters only after every file closes. On premature test unwinding
//! it deliberately leaks registration storage rather than leave C dangling.
#![allow(unsafe_code)]

use rusqlite::ffi;
use std::collections::BTreeMap;
use std::ffi::{CString, c_char, c_int, c_void};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(super) enum WalFault {
    BeforeWrite = 1,
    BeforeCommitSync = 2,
    AfterCommitSync = 3,
}

#[derive(Default)]
pub(super) struct Counts {
    pub main_writes: AtomicU64,
    pub main_bytes: AtomicU64,
    pub main_truncates: AtomicU64,
    pub fail_main_write: AtomicBool,
    pub wal_writes: AtomicU64,
    pub wal_commit_frames: AtomicU64,
    pub wal_commit_syncs: AtomicU64,
    pub wal_fault_hits: AtomicU64,
    wal_fault: AtomicU8,
    wal_commit_pending: AtomicBool,
    open: AtomicUsize,
}
struct VfsEntry {
    native: usize,
    counts: Arc<Counts>,
}
struct FileEntry {
    original: usize,
    _methods: Box<ffi::sqlite3_io_methods>,
    counts: Arc<Counts>,
    main: bool,
    wal: bool,
}
#[derive(Default)]
struct Registry {
    vfs: BTreeMap<usize, VfsEntry>,
    files: BTreeMap<usize, FileEntry>,
}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(0);
fn registry() -> std::sync::MutexGuard<'static, Registry> {
    REGISTRY
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub(super) struct Observer {
    vfs: Option<Box<ffi::sqlite3_vfs>>,
    name: Option<CString>,
    pub counts: Arc<Counts>,
}
impl Observer {
    pub fn new(native: &str) -> Self {
        let native = CString::new(native).unwrap();
        let name = CString::new(format!(
            "thrum-checkpoint-observer-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
        .unwrap();
        // SQLite's built-in VFS is process-lifetime storage. The caller keeps
        // this observer alive beyond all connections opened through its name.
        unsafe {
            let original = ffi::sqlite3_vfs_find(native.as_ptr());
            assert!(!original.is_null());
            let mut vfs = Box::new(*original);
            vfs.zName = name.as_ptr();
            vfs.pNext = std::ptr::null_mut();
            vfs.xOpen = Some(open);
            let counts = Arc::new(Counts::default());
            registry().vfs.insert(
                (&mut *vfs as *mut ffi::sqlite3_vfs) as usize,
                VfsEntry {
                    native: original as usize,
                    counts: counts.clone(),
                },
            );
            assert_eq!(ffi::sqlite3_vfs_register(&mut *vfs, 0), ffi::SQLITE_OK);
            Self {
                vfs: Some(vfs),
                name: Some(name),
                counts,
            }
        }
    }
    pub fn name(&self) -> &str {
        self.name.as_ref().unwrap().to_str().unwrap()
    }
    pub fn assert_closed(&self) {
        assert_eq!(self.counts.open.load(Ordering::Relaxed), 0);
    }
    /// The caller has already persisted the pending intent and complete blobs;
    /// its next WAL transaction is the final root/outcome COMMIT.
    pub fn arm_wal_fault(&self, fault: WalFault) {
        self.counts
            .wal_commit_pending
            .store(false, Ordering::Relaxed);
        assert_eq!(
            self.counts.wal_fault.swap(fault as u8, Ordering::Relaxed),
            0
        );
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        let Some(mut vfs) = self.vfs.take() else {
            return;
        };
        // Never panic in Drop or free storage still callable from SQLite.
        if self.counts.open.load(Ordering::Relaxed) != 0
            || unsafe { ffi::sqlite3_vfs_unregister(&mut *vfs) } != ffi::SQLITE_OK
        {
            Box::leak(vfs);
            if let Some(name) = self.name.take() {
                let _ = name.into_raw();
            }
            return;
        }
        registry()
            .vfs
            .remove(&((&mut *vfs as *mut ffi::sqlite3_vfs) as usize));
    }
}

unsafe extern "C" fn open(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    out: *mut c_int,
) -> c_int {
    // All pointers/lifetimes are supplied by SQLite. No Rust references to the
    // native file or registry lock survive an underlying callback.
    unsafe {
        let Some((native, counts)) = registry()
            .vfs
            .get(&(vfs as usize))
            .map(|e| (e.native, e.counts.clone()))
        else {
            return ffi::SQLITE_IOERR;
        };
        let native = native as *mut ffi::sqlite3_vfs;
        let Some(call) = (*native).xOpen else {
            return ffi::SQLITE_IOERR;
        };
        let rc = call(native, name, file, flags, out);
        if rc != ffi::SQLITE_OK || (*file).pMethods.is_null() {
            return rc;
        }
        let original = (*file).pMethods;
        let mut methods = Box::new(*original);
        methods.xWrite = Some(write);
        methods.xSync = Some(sync);
        methods.xTruncate = Some(truncate);
        methods.xClose = Some(close);
        let address = &*methods as *const ffi::sqlite3_io_methods;
        counts.open.fetch_add(1, Ordering::Relaxed);
        registry().files.insert(
            file as usize,
            FileEntry {
                original: original as usize,
                _methods: methods,
                counts,
                main: flags & ffi::SQLITE_OPEN_MAIN_DB != 0,
                wal: flags & ffi::SQLITE_OPEN_WAL != 0,
            },
        );
        (*file).pMethods = address;
        rc
    }
}
fn entry(file: *mut ffi::sqlite3_file) -> Option<(usize, Arc<Counts>, bool, bool)> {
    registry()
        .files
        .get(&(file as usize))
        .map(|e| (e.original, e.counts.clone(), e.main, e.wal))
}
unsafe extern "C" fn write(
    file: *mut ffi::sqlite3_file,
    bytes: *const c_void,
    n: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    unsafe {
        let Some((original, counts, main, wal)) = entry(file) else {
            return ffi::SQLITE_IOERR_WRITE;
        };
        if main {
            counts.main_writes.fetch_add(1, Ordering::Relaxed);
            if n > 0 {
                counts.main_bytes.fetch_add(n as u64, Ordering::Relaxed);
            }
            if counts.fail_main_write.load(Ordering::Relaxed) {
                return ffi::SQLITE_IOERR_WRITE;
            }
        }
        if wal {
            counts.wal_writes.fetch_add(1, Ordering::Relaxed);
            if counts
                .wal_fault
                .compare_exchange(
                    WalFault::BeforeWrite as u8,
                    0,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                counts.wal_fault_hits.fetch_add(1, Ordering::Relaxed);
                return ffi::SQLITE_IOERR_WRITE;
            }
        }
        let Some(call) = (*(original as *const ffi::sqlite3_io_methods)).xWrite else {
            return ffi::SQLITE_IOERR_WRITE;
        };
        let rc = call(file, bytes, n, offset);
        if wal && rc == ffi::SQLITE_OK && n >= 8 && offset >= 0 {
            // This store verifies 4096-byte pages. A WAL frame starts after
            // its 32-byte header, with page number and nonzero database size
            // identifying a commit frame. Inspect only bytes SQLite supplied.
            let data = std::slice::from_raw_parts(bytes.cast::<u8>(), n as usize);
            let start = offset as u64;
            let first = if start <= 32 {
                32
            } else {
                32 + (start - 32).div_ceil(4120) * 4120
            };
            let mut header = first;
            while header + 8 <= start + n as u64 {
                let at = (header - start) as usize;
                if data[at + 4..at + 8] != [0; 4] {
                    counts.wal_commit_frames.fetch_add(1, Ordering::Relaxed);
                    counts.wal_commit_pending.store(true, Ordering::Relaxed);
                }
                header += 4120;
            }
        }
        rc
    }
}
unsafe extern "C" fn sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    unsafe {
        let Some((original, counts, _, wal)) = entry(file) else {
            return ffi::SQLITE_IOERR_FSYNC;
        };
        let commit = wal && counts.wal_commit_pending.load(Ordering::Relaxed);
        let fault = counts.wal_fault.load(Ordering::Relaxed);
        if commit && fault == WalFault::BeforeCommitSync as u8 {
            counts.wal_fault.store(0, Ordering::Relaxed);
            counts.wal_fault_hits.fetch_add(1, Ordering::Relaxed);
            return ffi::SQLITE_IOERR_FSYNC;
        }
        let Some(call) = (*(original as *const ffi::sqlite3_io_methods)).xSync else {
            return ffi::SQLITE_IOERR_FSYNC;
        };
        let rc = call(file, flags);
        if commit && rc == ffi::SQLITE_OK {
            counts.wal_commit_syncs.fetch_add(1, Ordering::Relaxed);
            counts.wal_commit_pending.store(false, Ordering::Relaxed);
            if fault == WalFault::AfterCommitSync as u8 {
                counts.wal_fault.store(0, Ordering::Relaxed);
                counts.wal_fault_hits.fetch_add(1, Ordering::Relaxed);
                return ffi::SQLITE_IOERR_FSYNC;
            }
        }
        rc
    }
}
unsafe extern "C" fn truncate(file: *mut ffi::sqlite3_file, n: ffi::sqlite3_int64) -> c_int {
    unsafe {
        let Some((original, counts, main, _)) = entry(file) else {
            return ffi::SQLITE_IOERR_TRUNCATE;
        };
        if main {
            counts.main_truncates.fetch_add(1, Ordering::Relaxed);
        }
        let Some(call) = (*(original as *const ffi::sqlite3_io_methods)).xTruncate else {
            return ffi::SQLITE_IOERR_TRUNCATE;
        };
        call(file, n)
    }
}
unsafe extern "C" fn close(file: *mut ffi::sqlite3_file) -> c_int {
    unsafe {
        let Some(entry) = registry().files.remove(&(file as usize)) else {
            return ffi::SQLITE_IOERR_CLOSE;
        };
        let Some(call) = (*(entry.original as *const ffi::sqlite3_io_methods)).xClose else {
            return ffi::SQLITE_IOERR_CLOSE;
        };
        let rc = call(file);
        entry.counts.open.fetch_sub(1, Ordering::Relaxed);
        // entry and its method table remain alive until native close returns.
        rc
    }
}
