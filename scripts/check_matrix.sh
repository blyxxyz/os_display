#!/bin/sh
# Check if all the different combinations of enabled features compile.
# I usually run this command:

# rm Cargo.lock; scripts/check_matrix.sh && scripts/check_matrix.sh --target x86_64-pc-windows-msvc && scripts/check_matrix.sh --target wasm32-unknown-unknown

# (Older versions have trouble with modern Cargo.locks.)

set -e
set -v

# 1.68 really should be 1.66 but that one can't handle modern cargo registries

cargo +1.68 check --no-default-features --features native "$@"
cargo +1.68 check --no-default-features --features native,std "$@"

cargo +1.68 check --no-default-features --features unix "$@"
cargo +1.68 check --no-default-features --features unix,std "$@"

cargo +1.68 check --no-default-features --features windows "$@"
cargo +1.68 check --no-default-features --features windows,std "$@"

cargo +1.68 check --no-default-features --features native,unix "$@"
cargo +1.68 check --no-default-features --features native,unix,std "$@"

cargo +1.68 check --no-default-features --features native,windows "$@"
cargo +1.68 check --no-default-features --features native,windows,std "$@"

cargo +1.68 check --no-default-features --features unix,windows "$@"
cargo +1.68 check --no-default-features --features unix,windows,std "$@"

cargo +1.68 check --no-default-features --features native,unix,windows "$@"
cargo +1.68 check --no-default-features --features native,unix,windows,std "$@"
