use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

pub struct CountingAlloc;

static ALLOC_PROFILING_ENABLED: AtomicBool = AtomicBool::new(false);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static TOTAL_DEALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

#[derive(Clone, Copy, Debug, Default)]
pub struct AllocProfileSnapshot {
    pub live_bytes: usize,
    pub peak_live_bytes: usize,
    pub total_alloc_bytes: u64,
    pub total_dealloc_bytes: u64,
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub realloc_calls: u64,
}

impl AllocProfileSnapshot {
    pub fn net_bytes(self) -> i64 {
        self.total_alloc_bytes as i64 - self.total_dealloc_bytes as i64
    }
}

#[inline]
fn update_peak(new_live: usize) {
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while new_live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, new_live, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => break,
            Err(next_peak) => peak = next_peak,
        }
    }
}

#[inline]
fn on_alloc(size: usize) {
    TOTAL_ALLOC_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    let new_live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    update_peak(new_live);
}

#[inline]
fn on_dealloc(size: usize) {
    TOTAL_DEALLOC_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
}

#[inline]
fn on_realloc(old_size: usize, new_size: usize) {
    REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    if new_size >= old_size {
        let delta = new_size - old_size;
        TOTAL_ALLOC_BYTES.fetch_add(delta as u64, Ordering::Relaxed);
        let new_live = LIVE_BYTES.fetch_add(delta, Ordering::Relaxed) + delta;
        update_peak(new_live);
    } else {
        let delta = old_size - new_size;
        TOTAL_DEALLOC_BYTES.fetch_add(delta as u64, Ordering::Relaxed);
        LIVE_BYTES.fetch_sub(delta, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if ALLOC_PROFILING_ENABLED.load(Ordering::Relaxed) && !ptr.is_null() {
            on_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ALLOC_PROFILING_ENABLED.load(Ordering::Relaxed) {
            on_dealloc(layout.size());
        }
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if ALLOC_PROFILING_ENABLED.load(Ordering::Relaxed) && !out.is_null() {
            on_realloc(layout.size(), new_size);
        }
        out
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if ALLOC_PROFILING_ENABLED.load(Ordering::Relaxed) && !ptr.is_null() {
            on_alloc(layout.size());
        }
        ptr
    }
}

pub fn init_from_env() {
    let enabled = std::env::var("ACCRETION_ALLOC_PROFILE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    ALLOC_PROFILING_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    ALLOC_PROFILING_ENABLED.load(Ordering::Relaxed)
}

pub fn reset_counters() {
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_BYTES.store(0, Ordering::Relaxed);
    TOTAL_ALLOC_BYTES.store(0, Ordering::Relaxed);
    TOTAL_DEALLOC_BYTES.store(0, Ordering::Relaxed);
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
}

pub fn snapshot() -> AllocProfileSnapshot {
    AllocProfileSnapshot {
        live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
        peak_live_bytes: PEAK_BYTES.load(Ordering::Relaxed),
        total_alloc_bytes: TOTAL_ALLOC_BYTES.load(Ordering::Relaxed),
        total_dealloc_bytes: TOTAL_DEALLOC_BYTES.load(Ordering::Relaxed),
        alloc_calls: ALLOC_CALLS.load(Ordering::Relaxed),
        dealloc_calls: DEALLOC_CALLS.load(Ordering::Relaxed),
        realloc_calls: REALLOC_CALLS.load(Ordering::Relaxed),
    }
}
