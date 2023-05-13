# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## 0.2.1 - 2023-05-12
### Added
- Support for Rust's unstable allocators API behind the `unstable` feature flag.

### Changed
- Updated to Rust 2021 edition.

## 0.2.0 - 2019-04-27
### Removed
- `into_vec()`: use `into_bytes()` instead
- `From<&BitVec>` for `Vec<bool>`: use `bitvec.iter().collect()` instead
- `From<BitVec>` for `Vec<bool>`: use `bitvec.iter().collect()` instead

## 0.1.5 - 2019-04-27
### Added
- Implemented `FromIterator<bool>` and `FromIterator<&bool>` for `BitVec`.

## 0.1.4 - 2019-04-27
### Added
- `into_bytes()`

### Changed
- Made `Extend` preallocate based on the iterator's `size_hint`.
- `into_vec()` deprecated in favor of `into_bytes()` and will be removed in the
  next minor version bump.

## 0.1.3 - 2019-04-25
### Added
- A consuming iterator.
- Implemented `IntoIterator` for `&Bitvec` and `BitVec`.

## 0.1.2 - 2019-04-25
### Added
- `CHANGELOG.md`
- `BitVec::with_capacity()`
- `BitVec::from_bools()`
- Implemented the following:
  - `From<&[bool]>` for `BitVec`
  - `From<&Vec<bool>>` for `BitVec`
  - `From<Vec<bool>>` for `BitVec`
  - `From<&BitVec>` for `Vec<bool>`
  - `From<BitVec>` for `Vec<bool>`
- Specialized `Iterator` methods `size_hint`, `count`, `nth`, and `last` for
  performance.

