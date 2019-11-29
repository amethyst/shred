#!/bin/sh

set -e

cargo build --verbose --no-default-features --features "shred-derive";
cargo test --verbose --no-default-features --features "shred-derive";
cargo build --verbose;
cargo test --verbose;
