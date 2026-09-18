#!/usr/bin/env bash
# Verify the repository README stays in sync:
#   1. every relative link in README.md resolves to a real file/directory
#   2. every crate in code/Cargo.toml [workspace] members is linked from README
#
# Run from anywhere; the README and repo root are located from this script.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../../../.." && pwd)"
readme="$repo_root/README.md"

if [ ! -f "$readme" ]; then
    echo "ERROR: no README.md at $readme" >&2
    exit 1
fi

fail=0

# 1. Relative link/image targets must exist.
while IFS= read -r link; do
    case "$link" in
        http://* | https://* | mailto:* | \#*) continue ;;
    esac
    target="${link%%#*}"
    [ -z "$target" ] && continue
    if [ ! -e "$repo_root/$target" ]; then
        echo "MISSING LINK: '$target' (linked from README) does not exist"
        fail=1
    fi
done < <(grep -oE '\]\([^)]+\)' "$readme" | sed -E 's/^\]\(//; s/\)$//' | sort -u)

# 2. Every workspace member crate must appear as a `code/<member>` link.
while IFS= read -r member; do
    case "$member" in
        ex*) ;;
        *) continue ;;
    esac
    if ! grep -q "code/$member" "$readme"; then
        echo "MISSING ROW: workspace member '$member' is not linked from README"
        fail=1
    fi
done < <(grep -oE '"[^"]+"' "$repo_root/code/Cargo.toml" | tr -d '"')

if [ "$fail" -ne 0 ]; then
    exit 1
fi

echo "README check passed: all relative links resolve and every example is listed."
