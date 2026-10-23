#!/bin/sh
set -eu

case "${1:-}" in
  exit)
    exit "${2:-0}"
    ;;
  sleep)
    exec sleep "${2:-30}"
    ;;
  record-pid)
    printf '%s\n' "$$" > "$2"
    exec sleep "${3:-30}"
    ;;
  *)
    exit 64
    ;;
esac
