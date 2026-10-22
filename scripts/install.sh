#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
if [ -n "${PREFIX:-}" ]; then
    prefix=$PREFIX
else
    if [ -z "${HOME:-}" ]; then
        printf '%s\n' 'Set PREFIX or HOME before installing Limivra.' >&2
        exit 2
    fi
    prefix=$HOME/.local
fi

case "$prefix" in
    /*) ;;
    *) prefix=$(pwd -P)/$prefix ;;
esac

source_binary=$script_dir/limivra
if [ ! -x "$source_binary" ]; then
    source_binary=$script_dir/../target/release/limivra-cli
fi
if [ ! -x "$source_binary" ]; then
    printf '%s\n' 'limivra binary not found beside install.sh or in target/release.' >&2
    exit 1
fi

bin_dir=$prefix/bin
mkdir -p -- "$bin_dir"
temporary_binary=$(mktemp "$bin_dir/.limivra.XXXXXXXX")
cleanup() {
    rm -f -- "$temporary_binary"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

install -m 0755 -- "$source_binary" "$temporary_binary"
mv -f -- "$temporary_binary" "$bin_dir/limivra"
printf 'Installed Limivra to %s\n' "$bin_dir/limivra"
printf 'Add %s to PATH if it is not already present.\n' "$bin_dir"
