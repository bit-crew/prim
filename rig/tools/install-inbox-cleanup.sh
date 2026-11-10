#!/bin/sh
# install-inbox-cleanup.sh - install the ephemeral image inbox cleanup here.
#
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/rig/tools/install-inbox-cleanup.sh | sh
#
# Installs the prune script, the two systemd user units and enables the
# timer: twice a day, deleting only what is older than 12 hours.
#
# This belongs on the machine where the agent runs (the VPS): the inbox is
# ~/.cache/prim/inbox there.
#
# Idempotent: running it again is safe.
#
# RAW_BASE overrides where the files are downloaded from (default: main).

set -eu

PROG=${0##*/}
RAW_BASE="${RAW_BASE:-https://raw.githubusercontent.com/bit-crew/prim/main}"
BIN_DIR="$HOME/.local/bin"
UNIT_DIR="$HOME/.config/systemd/user"
INBOX="$HOME/.cache/prim/inbox"
MAX_AGE_MIN="${PRIM_INBOX_MAX_AGE_MIN:-720}"

log() { printf '[inbox-cleanup-install] %s\n' "$1"; }
err() { printf '[inbox-cleanup-install] %s\n' "$1" >&2; }

usage() {
    cat <<EOF
Usage: $PROG

Installs the inbox cleanup (script + systemd user units) and enables the
timer that runs it twice a day. Safe to run more than once.

Environment:
  RAW_BASE   where the files are downloaded from
              (default: $RAW_BASE)
EOF
}

case "${1:-}" in
    -h|--help) usage; exit 0 ;;
    "") : ;;
    *) err "unknown argument: $1"; usage >&2; exit 1 ;;
esac

fetch() {
    # fetch <local-name> <relative-path>: from a sibling file when run
    # locally (the script sits in rig/tools/, the units in rig/systemd/),
    # from RAW_BASE when piped.
    name="$1"
    rel="$2"
    dest="$3"
    here=$(dirname "$0")
    for candidate in "$here/$name" "$here/../systemd/$name"; do
        if [ -f "$candidate" ]; then
            install -m 755 "$candidate" "$dest"
            return 0
        fi
    done
    url="$RAW_BASE/$rel"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest.tmp"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest.tmp" "$url"
    else
        err "neither curl nor wget found; cannot download $name"
        exit 1
    fi
    chmod 755 "$dest.tmp"
    mv "$dest.tmp" "$dest"
}

mkdir -p "$BIN_DIR" "$UNIT_DIR" "$INBOX"

fetch prim-inbox-cleanup rig/tools/prim-inbox-cleanup "$BIN_DIR/prim-inbox-cleanup"
log "installed $BIN_DIR/prim-inbox-cleanup"

for unit in prim-inbox-cleanup.service prim-inbox-cleanup.timer; do
    fetch "$unit" "rig/systemd/$unit" "$UNIT_DIR/$unit"
    chmod 644 "$UNIT_DIR/$unit"
    log "installed $UNIT_DIR/$unit"
done

if ! command -v systemctl >/dev/null 2>&1; then
    err "systemctl not found: the units are in place, enable them by hand"
    exit 1
fi

systemctl --user daemon-reload
systemctl --user enable --now prim-inbox-cleanup.timer
log "timer enabled: runs twice a day, deletes only what is older than $MAX_AGE_MIN minutes"

cat <<EOF

Listo. Para ver que borraria sin borrar nada:

  prim-inbox-cleanup --dry-run

La proxima pasada del timer:
EOF
systemctl --user list-timers --all 2>/dev/null | grep inbox || true
