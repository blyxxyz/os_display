#!/bin/sh
# Check if all the different combinations of enabled features compile.
# I usually run this command:

# rm Cargo.lock; scripts/check_matrix.sh && scripts/check_matrix.sh --target x86_64-pc-windows-msvc && scripts/check_matrix.sh --target wasm32-unknown-unknown

# (Older versions have trouble with modern Cargo.locks.)

set -e
set -v

cargo +1.31 check --no-default-features --features native "$@"
cargo +1.36 check --no-default-features --features native,alloc "$@"
cargo +1.31 check --no-default-features --features native,std "$@"

cargo +1.31 check --no-default-features --features unix "$@"
cargo +1.36 check --no-default-features --features unix,alloc "$@"
cargo +1.31 check --no-default-features --features unix,std "$@"

cargo +1.31 check --no-default-features --features windows "$@"
cargo +1.36 check --no-default-features --features windows,alloc "$@"
cargo +1.31 check --no-default-features --features windows,std "$@"

cargo +1.31 check --no-default-features --features native,unix "$@"
cargo +1.36 check --no-default-features --features native,unix,alloc "$@"
cargo +1.31 check --no-default-features --features native,unix,std "$@"

cargo +1.31 check --no-default-features --features native,windows "$@"
cargo +1.36 check --no-default-features --features native,windows,alloc "$@"
cargo +1.31 check --no-default-features --features native,windows,std "$@"

cargo +1.31 check --no-default-features --features unix,windows "$@"
cargo +1.36 check --no-default-features --features unix,windows,alloc "$@"
cargo +1.31 check --no-default-features --features unix,windows,std "$@"

cargo +1.31 check --no-default-features --features native,unix,windows "$@"
cargo +1.36 check --no-default-features --features native,unix,windows,alloc "$@"
cargo +1.31 check --no-default-features --features native,unix,windows,std "$@"
