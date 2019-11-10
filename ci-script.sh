#!/bin/sh

set -e

if [ "$TRAVIS_RUST_VERSION" == "nightly" ]; then
  cargo build --verbose --all-features;
  cargo test --verbose --all-features;
  cargo bench --verbose --no-run --all-features;
else
  cargo build --verbose --no-default-features --features "shred-derive";
  cargo test --verbose --no-default-features --features "shred-derive";
  cargo build --verbose;
  cargo test --verbose;
fi
