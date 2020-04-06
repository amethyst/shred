# Changelog

* Batch dispatching ergonomics (getting rid of unsafe on the user side)
  ([#197]).

## 0.10.2 (2020-02-13)

### Changed

* Bumped dependency versions. ([#193])

[#193]: https://github.com/amethyst/shred/pull/193

## 0.10.1 (2020-02-12)

### Changed

* Bump `tynm` to `0.1.3`. ([#187])
* Implement `Resource` for `!Send + !Sync` types when `"parallel"` feature is disabled. ([#186])

[#186]: https://github.com/amethyst/shred/pull/186
[#187]: https://github.com/amethyst/shred/pulls/187

## 0.10.0 (2019-12-31)

### Changed

* Updated `arrayvec` from `0.4` to `0.5`. ([#176])
* Updated `rayon` from `1.1` to `1.3`. ([#180])
* Updated `smallvec` from `0.6` to `1.1`. ([#180])
* `shred-derive`: Updated `syn`, `quote`, `proc-macro2` to `1.0`. ([#176])
* ***Minimum Supported Rust Version updated to `1.38.0`***. ([#176])
* Improved clarity of fetch panic message. ([#182])

[#176]: https://github.com/amethyst/shred/issues/176
[#180]: https://github.com/amethyst/shred/issues/180
[#182]: https://github.com/amethyst/shred/issues/182

## 0.9.4 (2019-11-16)

### Added

* Batch dispatching. ([#147])

[#147]: https://github.com/amethyst/shred/pull/147
