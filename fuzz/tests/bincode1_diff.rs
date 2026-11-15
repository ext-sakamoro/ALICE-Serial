//! The differential check on stable, over a fixed corpus: random byte strings,
//! the encodings of values built from them (so the decoders see valid input),
//! and those encodings with one byte changed or cut short (so they see
//! near-valid input). Fails if any of the three outcomes was compared fewer
//! times than its floor, so a check that silently compared nothing is red.

use alice_serial_fuzz::{check, Compared, Zoo, LIMIT};
use arbitrary::{Arbitrary, Unstructured};

/// xorshift64*: a fixed sequence, the same on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

fn add(total: &mut Compared, c: Compared) {
    total.encoded += c.encoded;
    total.accepted += c.accepted;
    total.refused += c.refused;
    total.too_deep += c.too_deep;
}

#[test]
fn alice_serial_agrees_with_bincode_1_and_2_on_a_fixed_corpus() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut total = Compared::default();
    for round in 0..4000 {
        let len = (rng.next() % 600) as usize;
        let data = rng.bytes(len);
        add(&mut total, check(&data));

        let Ok(v) = Zoo::arbitrary(&mut Unstructured::new(&data)) else {
            continue;
        };
        let valid = alice_serial::bincode1::to_vec(&v, LIMIT).unwrap();
        add(&mut total, check(&valid));

        let mut changed = valid.clone();
        if !changed.is_empty() {
            let at = (rng.next() as usize) % changed.len();
            changed[at] ^= 1 << (round % 8);
            add(&mut total, check(&changed));
            let cut = (rng.next() as usize) % changed.len();
            add(&mut total, check(&valid[..cut]));
        }
        // the valid bytes with more after them: accepted, with the prefix used
        let mut longer = valid.clone();
        longer.extend_from_slice(&[0xab, 0xcd]);
        add(&mut total, check(&longer));
    }
    println!("{total:?}");
    assert!(total.encoded > 4000, "values compared: {total:?}");
    assert!(total.accepted > 4000, "accepted inputs compared: {total:?}");
    assert!(total.refused > 4000, "refused inputs compared: {total:?}");
}
