//! `bounded`: a length is accepted exactly when that many elements fit in the bound, never by
//! panicking, and a refusal carries the length, the bound and the reason.
//!
//! oracle: the arithmetic `len · elem_size ≤ remaining` computed in `u128` (which cannot
//! overflow for these operands), over an exhaustive small grid and at the edges of `u64` / `usize`.

// the grids are small (lengths ≤ 64, sizes ≤ 16), so `as usize` / `as u128` cannot truncate here
#![allow(clippy::cast_possible_truncation)]

use alice_serial::bounded::{len_at_most, len_within, LenError, LenErrorKind};

fn fits(len: u64, elem: usize, remaining: usize) -> bool {
    u128::from(len) * elem as u128 <= remaining as u128
}

#[test]
fn a_length_is_accepted_exactly_when_it_fits_over_a_small_grid() {
    let mut accepted = 0u32;
    let mut refused = 0u32;
    for len in 0..=40u64 {
        for elem in 1..=9usize {
            for remaining in 0..=200usize {
                match len_within(len, elem, remaining) {
                    Ok(n) => {
                        assert!(fits(len, elem, remaining), "{len} × {elem} ≤ {remaining}");
                        assert_eq!(n as u64, len);
                        accepted += 1;
                    }
                    Err(e) => {
                        assert!(!fits(len, elem, remaining), "{len} × {elem} ≤ {remaining}");
                        assert_eq!(
                            e,
                            LenError {
                                len,
                                bound: remaining,
                                kind: LenErrorKind::Remaining
                            }
                        );
                        refused += 1;
                    }
                }
            }
        }
    }
    // both outcomes occur: the grid is not one-sided
    assert!(
        accepted > 10_000 && refused > 10_000,
        "accepted {accepted}, refused {refused}"
    );
}

#[test]
fn exactly_the_remaining_bytes_pass_and_one_more_element_does_not() {
    for elem in 1..=16usize {
        for len in 0..=64u64 {
            let remaining = len as usize * elem;
            assert_eq!(
                len_within(len, elem, remaining),
                Ok(len as usize),
                "{len} × {elem}"
            );
            assert_eq!(
                len_within(len + 1, elem, remaining).unwrap_err().kind,
                LenErrorKind::Remaining,
                "{} × {elem}",
                len + 1
            );
            if remaining > 0 {
                assert_eq!(
                    len_within(len, elem, remaining - 1).unwrap_err().kind,
                    LenErrorKind::Remaining,
                    "{len} × {elem} in {}",
                    remaining - 1
                );
            }
        }
    }
}

#[test]
fn a_byte_count_past_usize_is_an_overflow_not_a_panic() {
    for elem in 2..=16usize {
        assert_eq!(
            len_within(u64::MAX, elem, usize::MAX),
            Err(LenError {
                len: u64::MAX,
                bound: usize::MAX,
                kind: LenErrorKind::Overflow
            }),
            "u64::MAX × {elem}"
        );
    }
    // the smallest count whose bytes overflow, for each element size
    for elem in 2..=16usize {
        let first = (usize::MAX / elem) as u64 + 1;
        assert_eq!(
            len_within(first, elem, usize::MAX).unwrap_err().kind,
            LenErrorKind::Overflow,
            "{first} × {elem}"
        );
        assert_eq!(
            len_within(first - 1, elem, usize::MAX),
            Ok(first as usize - 1),
            "{} × {elem}",
            first - 1
        );
    }
    // one-byte elements cannot overflow on a 64-bit target: the whole range is a remaining check
    #[cfg(target_pointer_width = "64")]
    assert_eq!(len_within(u64::MAX, 1, usize::MAX), Ok(usize::MAX));
}

#[test]
fn a_zero_element_size_is_refused_for_every_length() {
    for len in [0u64, 1, 7, u64::MAX] {
        for remaining in [0usize, 1, usize::MAX] {
            assert_eq!(
                len_within(len, 0, remaining),
                Err(LenError {
                    len,
                    bound: remaining,
                    kind: LenErrorKind::ZeroElementSize
                })
            );
        }
    }
}

#[test]
fn a_stream_length_is_accepted_up_to_its_limit() {
    for limit in [0usize, 1, 64, 1 << 20] {
        assert_eq!(len_at_most(limit as u64, limit), Ok(limit));
        assert_eq!(len_at_most(0, limit), Ok(0));
        assert_eq!(
            len_at_most(limit as u64 + 1, limit),
            Err(LenError {
                len: limit as u64 + 1,
                bound: limit,
                kind: LenErrorKind::Limit
            })
        );
    }
    assert_eq!(
        len_at_most(u64::MAX, 1 << 20).unwrap_err().kind,
        LenErrorKind::Limit
    );
    #[cfg(target_pointer_width = "64")]
    assert_eq!(len_at_most(u64::MAX, usize::MAX), Ok(usize::MAX));
}

#[test]
fn a_refusal_names_the_length_the_bound_and_the_reason() {
    let text = |len, bound, kind| LenError { len, bound, kind }.to_string();
    assert_eq!(
        text(5, 4, LenErrorKind::Remaining),
        "length 5 does not fit in the 4 bytes that remain"
    );
    assert_eq!(
        text(5, 4, LenErrorKind::Limit),
        "length 5 is over the limit 4"
    );
    assert_eq!(
        text(5, 4, LenErrorKind::Overflow),
        "length 5 overflows usize in bytes"
    );
    assert_eq!(
        text(5, 4, LenErrorKind::ZeroElementSize),
        "length 5 of zero-size elements cannot be bounded"
    );
    let e: &dyn core::error::Error = &LenError {
        len: 1,
        bound: 0,
        kind: LenErrorKind::Remaining,
    };
    assert!(e.source().is_none());
}

/// On a 32-bit target a length past `usize::MAX` is an overflow, whatever the element size (the
/// conversion is checked, not a cast). On 64-bit every `u64` fits, so this case does not arise.
#[cfg(target_pointer_width = "32")]
#[test]
fn a_length_past_usize_is_an_overflow_on_32_bit() {
    let len = u64::from(u32::MAX) + 1;
    assert_eq!(
        len_within(len, 1, usize::MAX).unwrap_err().kind,
        LenErrorKind::Overflow
    );
    assert_eq!(
        len_at_most(len, usize::MAX).unwrap_err().kind,
        LenErrorKind::Limit
    );
}
