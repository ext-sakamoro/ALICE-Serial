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

## bincode 1 layout (feature `serde`)

`bincode1` reads and writes the bytes bincode 1.x writes by default (and bincode 2.x with
`config::legacy()`), for any serde type, so stores that already hold such bytes keep them
unchanged without depending on bincode:

- fixed-width little-endian integers, `u64` lengths, a `u32` variant index, a `u8` tag for `Option`
- every call takes a limit; a decode checks each length against the bytes that remain before it sizes anything
- a decode refuses values nested deeper than `DEFAULT_MAX_DEPTH` (128) levels, each enum, `Some`, sequence, map, tuple, struct and newtype struct being one, so input cannot exhaust the stack; `from_slice_with_depth` names another depth
- an encode refuses a sequence of zero-size elements, which could not be read back
- `from_slice` refuses trailing bytes; `from_slice_prefix` returns the value and the bytes it used
- builds without `std` (`alloc` only)

`BINCODE1_LAYOUT` states the rules and `BINCODE1_LAYOUT_ID` is its SHA-256.

```rust
use alice_serial::bincode1::{from_slice, to_vec};

let bytes = to_vec(&(7u16, "ab"), 1 << 20)?;
assert_eq!(bytes, [7, 0, 2, 0, 0, 0, 0, 0, 0, 0, b'a', b'b']);
let back: (u16, String) = from_slice(&bytes, 1 << 20)?;
# Ok::<(), alice_serial::bincode1::Bincode1Error>(())
```

## License

`LicenseRef-Proprietary` from this version: the source is published for reading only, and
using, copying, modifying or distributing it needs a written agreement with
the copyright holder. The text is [LICENSE-Proprietary](LICENSE-Proprietary). Versions
published before keep the licence they were published under.

Licence enquiries: <contact@extoria.co.jp>
