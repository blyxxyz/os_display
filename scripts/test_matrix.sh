#!/bin/sh
set -e
set -v

cargo +1.68 test --no-default-features --features native,std --lib "$@"
cargo +1.68 test --no-default-features --features unix,std --lib "$@"
cargo +1.68 test --no-default-features --features windows,std --lib "$@"
cargo +1.68 test --no-default-features --features native,unix,std --lib "$@"
cargo +1.68 test --no-default-features --features native,windows,std --lib "$@"
cargo +1.68 test --no-default-features --features unix,windows,std --lib "$@"
cargo +1.68 test --no-default-features --features native,unix,windows,std --lib "$@"
