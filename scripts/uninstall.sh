#!/bin/sh
set -eu

if [ -n "${PREFIX:-}" ]; then
    prefix=$PREFIX
else
    if [ -z "${HOME:-}" ]; then
        printf '%s\n' 'Set PREFIX or HOME before uninstalling Limivra.' >&2
        exit 2
    fi
    prefix=$HOME/.local
fi

case "$prefix" in
    /*) ;;
    *) prefix=$(pwd -P)/$prefix ;;
esac

bin_dir=$prefix/bin
rm -f -- "$bin_dir/limivra"
rmdir -- "$bin_dir" 2>/dev/null || true
printf 'Removed Limivra from %s/bin/limivra\n' "$prefix"
