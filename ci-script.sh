#!/bin/sh

set -e

cargo build --verbose --no-default-features
cargo test --verbose --no-default-features
cargo build --verbose
cargo test --verbose
