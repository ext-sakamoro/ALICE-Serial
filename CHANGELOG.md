# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- CI and Security & Hygiene also run on `ci/**` branches and on demand; Security & Hygiene runs on every push (no paths filter); a run on `main` is never cancelled by a later push

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
