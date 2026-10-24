# ALICE-Serial

Serialization formats (MessagePack, CBOR-like, binary encoding)

## Installation

```toml
[dependencies]
alice-serial = { git = "https://github.com/ext-sakamoro/ALICE-Serial", tag = "v0.2.0" }
```

## Format

Each value has exactly one encoding, and `decode` accepts only that one, so encoded bytes can be hashed, signed and compared:

- integers in their shortest form; floats bit-exact (NaN payloads and `-0.0` kept)
- `f32` / `f64` / `u32` / `u64` / `i64` vectors packed little-endian, 4 or 8 bytes per element
- map keys in strictly ascending order of their encodings, no duplicates
- nesting at most `MAX_DEPTH` (64) arrays and maps
- `encode` / `decode` carry the header `ALSR` + format version; `decode` refuses trailing bytes

`FORMAT_SPEC` states the rules and `SERIAL_SEMANTICS_ID` is its SHA-256.

```rust
use alice_serial::{decode, encode, Value};

let v = Value::Map(vec![(Value::Str("b".into()), Value::U64(2)), (Value::Str("a".into()), Value::U64(1))]);
let bytes = encode(&v)?;
let back = decode(&bytes)?; // keys come back in canonical order
assert_eq!(encode(&back)?, bytes);
# Ok::<(), alice_serial::SerialError>(())
```

## License

`LicenseRef-Proprietary`. The text is [LICENSE-Proprietary](LICENSE-Proprietary).

Enquiries: https://extoria.co.jp/
