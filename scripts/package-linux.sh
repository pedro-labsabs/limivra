#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
project_root=$(CDPATH= cd -- "$script_dir/.." && pwd -P)

case "$(uname -s):$(uname -m)" in
    Linux:x86_64) ;;
    *)
        printf '%s\n' 'package-linux.sh requires a Linux x86_64 host.' >&2
        exit 2
        ;;
esac

source_date_epoch=${SOURCE_DATE_EPOCH:-0}
case "$source_date_epoch" in
    ''|*[!0-9]*)
        printf '%s\n' 'SOURCE_DATE_EPOCH must be a non-negative integer.' >&2
        exit 2
        ;;
esac

if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    case "$CARGO_TARGET_DIR" in
        /*) target_dir=$CARGO_TARGET_DIR ;;
        *) target_dir=$project_root/$CARGO_TARGET_DIR ;;
    esac
else
    target_dir=$project_root/target
fi

if [ -n "${LIMIVRA_DIST_DIR:-}" ]; then
    case "$LIMIVRA_DIST_DIR" in
        /*) dist_dir=$LIMIVRA_DIST_DIR ;;
        *) dist_dir=$project_root/$LIMIVRA_DIST_DIR ;;
    esac
else
    dist_dir=$project_root/dist
fi

archive_name=limivra-linux-x86_64.tar.gz
temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/limivra-package.XXXXXXXX")
temporary_archive=
temporary_checksums=
temporary_tar=
cleanup() {
    rm -rf -- "$temporary_root"
    if [ -n "$temporary_archive" ]; then
        rm -f -- "$temporary_archive"
    fi
    if [ -n "$temporary_checksums" ]; then
        rm -f -- "$temporary_checksums"
    fi
    if [ -n "$temporary_tar" ]; then
        rm -f -- "$temporary_tar"
    fi
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

cd "$project_root"
"${CARGO:-cargo}" build -p limivra-cli --release --locked --offline

binary=$target_dir/release/limivra-cli
if [ ! -x "$binary" ]; then
    printf 'Expected release binary was not produced: %s\n' "$binary" >&2
    exit 1
fi

staging_dir=$temporary_root/package
mkdir -p "$staging_dir" "$dist_dir"
install -m 0755 -- "$binary" "$staging_dir/limivra"
install -m 0755 -- "$script_dir/install.sh" "$staging_dir/install.sh"
install -m 0755 -- "$script_dir/uninstall.sh" "$staging_dir/uninstall.sh"
install -m 0644 -- "$project_root/LICENSE" "$staging_dir/LICENSE"
install -m 0644 -- "$project_root/NOTICES.md" "$staging_dir/NOTICES.md"
install -m 0644 -- "$project_root/README.md" "$staging_dir/README.md"

temporary_archive=$dist_dir/.$archive_name.$$.tmp
temporary_checksums=$dist_dir/.SHA256SUMS.$$.tmp
temporary_tar=$dist_dir/.$archive_name.$$.tar.tmp
(
    cd "$staging_dir"
    tar --sort=name --mtime="@$source_date_epoch" --owner=0 --group=0 --numeric-owner \
        -cf "$temporary_tar" LICENSE NOTICES.md README.md install.sh limivra uninstall.sh
)
gzip -n -c -- "$temporary_tar" > "$temporary_archive"
rm -f -- "$temporary_tar"
temporary_tar=
checksum_output=$(sha256sum -- "$temporary_archive")
checksum=${checksum_output%% *}
printf '%s  %s\n' "$checksum" "$archive_name" > "$temporary_checksums"

mv -- "$temporary_archive" "$dist_dir/$archive_name"
temporary_archive=
mv -- "$temporary_checksums" "$dist_dir/SHA256SUMS"
temporary_checksums=
printf 'Created %s\n' "$dist_dir/$archive_name"
printf 'Checksum recorded in %s/SHA256SUMS\n' "$dist_dir"
