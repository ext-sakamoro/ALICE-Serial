//! Oracle: a forged item count never sizes an allocation past what the bytes
//! left can hold
//!
//! A count is refused as soon as it exceeds the bytes after it, even when it is
//! still smaller than the whole input; the largest single allocation of the
//! decode is measured with a counting global allocator.

use alice_serial::{decode_value, SerialError};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Largest;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to the system allocator unchanged and only
// records the requested size
unsafe impl GlobalAlloc for Largest {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        // SAFETY: same layout the caller passed, forwarded as is
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System.alloc` with this layout
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Largest = Largest;

#[test]
fn a_count_past_the_bytes_left_allocates_nothing_for_it() {
    // [ bytes(20000), array(count 15000) ] with nothing after the inner count:
    // the count is below the input length but above the bytes left (0)
    const FILL: usize = 20_000;
    const COUNT: u32 = 15_000;
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

        LARGEST.store(0, Ordering::Relaxed);
        assert_eq!(
            decode_value(&b),
            Err(SerialError::BufferTooShort),
            "{tag:#04x}"
        );
        let largest = LARGEST.load(Ordering::Relaxed);
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
