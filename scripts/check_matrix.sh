#!/bin/sh
# Check if all the different combinations of enabled features compile.
# Invoke like `scripts/check_matrix.sh +1.66`, `scripts/check_matrix.sh --target x86_64-pc-windows-msvc`.
# You may have to `rm Cargo.lock` first.

set -e

version='+stable'
case "$1" in
    +*)
    version="$1"
    shift
esac
echo "version=$version"
echo "args=$@"

set -v

cargo "$version" check --no-default-features --features native "$@"
cargo "$version" check --no-default-features --features native,std "$@"

cargo "$version" check --no-default-features --features unix "$@"
cargo "$version" check --no-default-features --features unix,std "$@"

cargo "$version" check --no-default-features --features windows "$@"
cargo "$version" check --no-default-features --features windows,std "$@"

cargo "$version" check --no-default-features --features native,unix "$@"
cargo "$version" check --no-default-features --features native,unix,std "$@"

cargo "$version" check --no-default-features --features native,windows "$@"
cargo "$version" check --no-default-features --features native,windows,std "$@"

cargo "$version" check --no-default-features --features unix,windows "$@"
cargo "$version" check --no-default-features --features unix,windows,std "$@"

cargo "$version" check --no-default-features --features native,unix,windows "$@"
cargo "$version" check --no-default-features --features native,unix,windows,std "$@"
