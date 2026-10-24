//! Oracle: a forged item count never sizes memory past what the bytes can hold
//!
//! - a count above the bytes left is refused before anything is sized for it,
//!   even when it is below the whole input's length (largest single allocation)
//! - counts nested inside each other cannot multiply: the peak of live heap
//!   bytes during a decode stays linear in the input, for a chain of arrays and
//!   maps that each claim nearly all of the bytes after them
//!
//! A counting global allocator records the largest request and the peak of
//! live bytes. The tests run one at a time (a lock), since the counters are
//! process-wide.

use alice_serial::{decode_value, SerialError, Value};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

struct Counting;

static LARGEST: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static SERIAL: Mutex<()> = Mutex::new(());

// SAFETY: forwards every call to the system allocator unchanged and only
// records the sizes
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(live, Ordering::Relaxed);
        // SAFETY: same layout the caller passed, forwarded as is
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: `ptr` came from `System.alloc` with this layout
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Largest single request and peak live bytes above the starting level while
/// `f` runs.
fn measure<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    let base = LIVE.load(Ordering::Relaxed);
    LARGEST.store(0, Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let out = f();
    let largest = LARGEST.load(Ordering::Relaxed);
    let peak = PEAK.load(Ordering::Relaxed) - base;
    (out, largest, peak)
}

#[test]
fn a_count_past_the_bytes_left_allocates_nothing_for_it() {
    // [ bytes(20000), array(count 15000) ] with nothing after the inner count:
    // the count is below the input length but above the bytes left (0)
    const FILL: usize = 20_000;
    const COUNT: u32 = 15_000;
    let _one = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut input = vec![0xDD, 0, 0, 0, 2, 0xC6];
    input.extend_from_slice(&u32::try_from(FILL).unwrap().to_be_bytes());
    input.resize(input.len() + FILL, 0xAB);
    for tag in [0xDDu8, 0xDF] {
        let mut b = input.clone();
        b.extend_from_slice(&[tag]);
        b.extend_from_slice(&COUNT.to_be_bytes());
        assert!(
            usize::try_from(COUNT).unwrap() <= b.len(),
            "the count must be within the whole input"
        );

        let (got, largest, _) = measure(|| decode_value(&b));
        assert_eq!(got, Err(SerialError::BufferTooShort), "{tag:#04x}");
        // the byte string itself (FILL) is the largest thing the decode needs;
        // sizing 15000 items would ask for many times that
        assert!(
            largest >= FILL,
            "measured {largest}: the allocator saw nothing"
        );
        assert!(
            largest < 2 * FILL,
            "{tag:#04x}: largest allocation {largest} bytes, the forged count sized it"
        );
    }
}

/// `levels` containers of `tag`, each holding the next as its first item (a
/// map: as its first key) and claiming as many items as the bytes after its
/// count could hold, then an array of `filler` nulls: every count passes the
/// bytes-left check on its own, and the decode fails only when an outer
/// container's second item is missing.
fn chain(tag: u8, levels: usize, filler: usize) -> Vec<u8> {
    let total = (levels + 1) * 5 + filler;
    let mut out = Vec::with_capacity(total);
    for level in 0..levels {
        let left = total - (level + 1) * 5;
        let claim = if tag == 0xDF { left / 2 } else { left };
        out.extend_from_slice(&[tag]);
        out.extend_from_slice(&u32::try_from(claim).unwrap().to_be_bytes());
    }
    out.extend_from_slice(&[0xDD]);
    out.extend_from_slice(&u32::try_from(filler).unwrap().to_be_bytes());
    out.resize(total, 0xC0);
    out
}

#[test]
fn nested_counts_cannot_multiply_memory() {
    let _one = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let per_item = std::mem::size_of::<Value>();
    for tag in [0xDDu8, 0xDF] {
        let input = chain(tag, 62, 65_536);
        let (got, _, peak) = measure(|| decode_value(&input));
        assert!(got.is_err(), "{tag:#04x}: the chain is short of items");
        // each item takes at least one input byte; a Vec holds at most twice
        // the items pushed into it, a map entry is two values
        let bound = 4 * per_item * input.len() + 64 * 1024;
        assert!(
            peak > input.len(),
            "{tag:#04x}: measured {peak}: the decode allocated nothing"
        );
        assert!(
            peak <= bound,
            "{tag:#04x}: peak live {peak} bytes for {} input bytes (bound {bound}): nested counts multiplied",
            input.len()
        );
    }
}
