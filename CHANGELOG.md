# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - Unreleased

### Changed

- Licence: `MIT OR Apache-2.0` → `LicenseRef-Proprietary`
- Breaking: each value has exactly one encoding and the decoder accepts only that one: integers in their shortest form, map keys in strictly ascending order of their encodings with no duplicates. Map entries are written in that order whatever their order in `Value::Map`. Floats are carried bit-exact (NaN payloads and `-0.0` kept), so each bit pattern has one encoding
- Breaking: `encode` writes the header `ALSR` + format version 1 (`u16` little-endian) before the value, and `decode` checks it and refuses bytes after the value. `encode_value` / `decode_value` read and write a value without the header
- Breaking: `encode`, `encode_value`, `decode`, `decode_value` and `decode_varint` return `Result<_, SerialError>`; `SerialError` names each refusal (`InvalidTag`, `NonCanonical`, `UnsortedKeys`, `DuplicateKey`, `TooDeep`, `TooLong`, `TrailingBytes`, `BadMagic`, `UnsupportedVersion`, ...) and is `#[non_exhaustive]`; `InvalidFormat` is removed
- `decode_varint` refuses a longer form than the value needs and values over `u64::MAX`; `encode_varint` writes at most 10 bytes in a bounded loop
- `src/format-1.txt` is kept byte-exact on every checkout (`.gitattributes`), since `SERIAL_SEMANTICS_ID` hashes it; CI also runs on Windows
- CI and Security & Hygiene also run on `ci/**` branches and on demand; Security & Hygiene runs on every push (no paths filter); a run on `main` is never cancelled by a later push

### Added

- `bounded`: `len_within(len, elem_size, remaining)` accepts a length or count read from input only when that many `elem_size`-byte elements fit in the bytes that remain, and `len_at_most(len, limit)` bounds a length read from a stream by a named limit; both use checked arithmetic and return `LenError` (the length, the bound and `LenErrorKind::Remaining` / `Limit` / `Overflow` / `ZeroElementSize`), so a decoder sizes an allocation by the input it has, never by a claim in it
- `Value::F32` (tag `ca`, big-endian bits) and packed arrays `F32Array` / `F64Array` / `U32Array` / `U64Array` / `I64Array` (tags `d4`..`d8`, element count then fixed-width little-endian words), so numeric vectors take 4 or 8 bytes per element
- `MAX_DEPTH` (64): the encoder and the decoder refuse arrays and maps nested deeper, so a crafted input cannot exhaust the stack
- A forged length or count is refused before it sizes an allocation, and counts nested in each other cannot multiply memory: containers grow with the items decoded, never from a count (`tests/decode_alloc_bound.rs` measures the largest allocation and the peak of live bytes with a counting allocator)
- `FORMAT_SPEC` (the rules of format 1 as text) and `SERIAL_SEMANTICS_ID` (its SHA-256), to be kept next to stored bytes and compared by the reader
- `tests/canonical.rs`: a golden encoding per type, bit-exact floats (NaN payloads, signalling and negative NaN, `-0.0`, subnormals), and every input of up to 2 bytes and 20,000 sampled longer ones either fail or re-encode to exactly themselves
- Fuzz targets `fuzz_decode` (any bytes: no panic, accepted bytes re-encode to themselves) and `fuzz_roundtrip` (a built value: `encode(decode(encode(v))) == encode(v)`)

### Fixed

- A string, byte string, array or map longer than `u32::MAX` was written with a truncated length; it is now refused with `TooLong`

## [0.1.2] - 2026-10-24

### Changed

- The source repository is published with a single initial commit; the code is the same as 0.1.1
- Comments in the CI workflows, `deny.toml`, `dependabot.yml`, `rust-toolchain.toml` and `scripts/preflight.sh` no longer refer to notes outside this repository
- README: the dependency line uses this repository at tag `v0.1.2`
- `Cargo.lock` is tracked and CI builds with `--locked`

## [0.1.1] - 2026-09-14

- crates.io metadata (repository / keywords / categories)

## [0.1.0] - 2026-09-14

- Initial release
