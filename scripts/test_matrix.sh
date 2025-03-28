#!/bin/sh
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

cargo "$version" test --no-default-features --features native,std --lib "$@"
cargo "$version" test --no-default-features --features unix,std --lib "$@"
cargo "$version" test --no-default-features --features windows,std --lib "$@"
cargo "$version" test --no-default-features --features native,unix,std --lib "$@"
cargo "$version" test --no-default-features --features native,windows,std --lib "$@"
cargo "$version" test --no-default-features --features unix,windows,std --lib "$@"
cargo "$version" test --no-default-features --features native,unix,windows,std --lib "$@"
