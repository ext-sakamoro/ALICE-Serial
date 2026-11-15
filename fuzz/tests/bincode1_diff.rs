//! The differential check on stable, over a fixed corpus: random byte strings,
//! the encodings of values built from them (so the decoders see valid input),
//! and those encodings with one byte changed or cut short (so they see
//! near-valid input). Fails if any of the three outcomes was compared fewer
//! times than its floor, so a check that silently compared nothing is red.

use std::collections::BTreeMap;

use alice_serial_fuzz::{check, Compared, Kind, Newtype, TupleStruct, UnitStruct, Zoo, LIMIT};
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
    // one value nested past the default depth: alice-serial refuses it at 128 and the
    // check compares it again at a depth that holds it
    let mut kind = Kind::Unit;
    for _ in 0..300 {
        kind = Kind::Nested(Some(Box::new(kind)));
    }
    let deep = Zoo {
        b: true,
        u8_: 1,
        i8_: -1,
        u16_: 2,
        i16_: -2,
        u32_: 3,
        i32_: -3,
        u64_: 4,
        i64_: -4,
        u128_: 5,
        i128_: -5,
        usize_: 6,
        f32_: 1.5,
        f64_: -2.5,
        c: 'x',
        s: "deep".into(),
        o: Some(7),
        unit: (),
        unit_struct: UnitStruct,
        newtype: Newtype(8),
        tuple: (9, -9, 'y'),
        tuple_struct: TupleStruct(10, Some('z')),
        array: [1, 2, 3],
        seq: vec![kind],
        map: BTreeMap::new(),
        nested: None,
    };
    let bytes = alice_serial::bincode1::to_vec(&deep, LIMIT).unwrap();
    let c = check(&bytes);
    assert_eq!((c.too_deep, c.accepted), (1, 1), "{c:?}");
    add(&mut total, c);
    println!("{total:?}");
    assert!(total.encoded > 4000, "values compared: {total:?}");
    assert!(total.accepted > 4000, "accepted inputs compared: {total:?}");
    assert!(total.refused > 4000, "refused inputs compared: {total:?}");
}
