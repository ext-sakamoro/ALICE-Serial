# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - Unreleased

### Changed

- Breaking: each value has exactly one encoding and the decoder accepts only that one: integers in their shortest form, map keys in strictly ascending order of their encodings with no duplicates. Map entries are written in that order whatever their order in `Value::Map`. Floats are carried bit-exact (NaN payloads and `-0.0` kept), so each bit pattern has one encoding
- Breaking: `encode` writes the header `ALSR` + format version 1 (`u16` little-endian) before the value, and `decode` checks it and refuses bytes after the value. `encode_value` / `decode_value` read and write a value without the header
- Breaking: `encode`, `encode_value`, `decode`, `decode_value` and `decode_varint` return `Result<_, SerialError>`; `SerialError` names each refusal (`InvalidTag`, `NonCanonical`, `UnsortedKeys`, `DuplicateKey`, `TooDeep`, `TooLong`, `TrailingBytes`, `BadMagic`, `UnsupportedVersion`, ...) and is `#[non_exhaustive]`; `InvalidFormat` is removed
- `decode_varint` refuses a longer form than the value needs and values over `u64::MAX`

### Added

- `Value::F32` (tag `ca`, big-endian bits) and packed arrays `F32Array` / `F64Array` / `U32Array` / `U64Array` / `I64Array` (tags `d4`..`d8`, element count then fixed-width little-endian words), so numeric vectors take 4 or 8 bytes per element
- `MAX_DEPTH` (64): the encoder and the decoder refuse arrays and maps nested deeper, so a crafted input cannot exhaust the stack
- A forged length or count is refused before it sizes an allocation (`tests/decode_alloc_bound.rs` measures the largest allocation with a counting allocator)
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
