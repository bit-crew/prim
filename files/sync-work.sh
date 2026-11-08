#!/bin/sh
# sync-work.sh - manual two-way copy between a client copy of ~/work and the
# VPS SMB share served by the optional `files` component (S9-S11).
#
# Two one-way mirrors, always run in this order:
#
#   pull   the VPS wins   -> the client is refreshed from the share, and files
#                             the VPS no longer has are removed from the client
#   push   the client wins -> the client is copied to the share, and files the
#                             user deleted on the client are removed from the VPS
#
# Always pull before pushing: that is what keeps both sides in step without a
# sync engine. Files the VPS gained while the client was offline are NEVER
# deleted by a push (they are not in the manifest yet); the next pull brings
# them down.
#
# Deletions are never silent: a deleted file is moved to .sync-deleted/<ts>/
# on the side that receives the deletion, so it can be recovered with mv.
#
# The manifest (.sync-manifest in the client copy) records the VPS file list
# as of the last operation. It is what makes deletion propagation precise:
# only files the user actually deleted are removed, never anything else.
#
# VCS internals and heavy build dirs are always excluded. .git is never
# touched, so the client copy is NOT a git clone: history stays on the VPS.
#
# Usage:
#   sync-work.sh pull|push [--remote NAME] [--path PATH] [--dest DIR]
#                           [--dry-run] [--force]

set -eu

PROG=${0##*/}
REMOTE=vps
REMOTE_PATH=work
DEST="${WORK_SYNC_DEST:-$HOME/storage/documents/work}"
DIRECTION=""
DRY_RUN=0
FORCE=0
TS=$(date +%Y%m%d-%H%M%S)

MANIFEST_NAME=.sync-manifest
QUARANTINE_NAME=.sync-deleted

log() { printf '\033[1;32m[work-sync]\033[0m %s\n' "$1"; }
err() { printf '\033[1;31m[work-sync]\033[0m %s\n' "$1" >&2; }

usage() {
    cat <<EOF
Usage: $PROG pull|push [options]

Manual two-way copy between a local copy of the work tree and the VPS SMB
share. Run pull before push: pull lets the VPS win, push lets the client
win. A file deleted on one side is removed on the other and kept aside in
.sync-deleted/<timestamp>/ so it can be recovered.

Commands:
  pull    VPS share  -> local copy   (the VPS wins)
  push    local copy -> VPS share    (the client wins)

Options:
  --remote NAME   rclone remote of the VPS share (default: vps)
  --path PATH     path inside the remote (default: work)
  --dest DIR      local copy directory
                  (default: \$WORK_SYNC_DEST or \$HOME/storage/documents/work)
  --dry-run       report what would happen, change nothing
  --force         the direction wins; default is newest-file-wins
  -h, --help      this help
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        pull|push) DIRECTION="$1" ;;
        --remote)
            shift
            [ $# -gt 0 ] || { err "--remote needs a value"; exit 1; }
            REMOTE="$1"
            ;;
        --path)
            shift
            [ $# -gt 0 ] || { err "--path needs a value"; exit 1; }
            REMOTE_PATH="$1"
            ;;
        --dest)
            shift
            [ $# -gt 0 ] || { err "--dest needs a value"; exit 1; }
            DEST="$1"
            ;;
        --dry-run) DRY_RUN=1 ;;
        --force) FORCE=1 ;;
        -h|--help) usage; exit 0 ;;
        *)
            err "unknown argument: $1"
            usage >&2
            exit 1
            ;;
    esac
    shift
done

if [ -z "$DIRECTION" ]; then
    err "missing command: pull or push"
    usage >&2
    exit 1
fi

command -v rclone >/dev/null 2>&1 || {
    err "rclone not found in PATH"
    err "install it from https://rclone.io/ (Termux: pkg install rclone)"
    exit 1
}

case "$REMOTE" in
    :*) : ;;
    *)
        if ! rclone listremotes 2>/dev/null | grep -qx "$REMOTE:"; then
            err "rclone remote '$REMOTE' not found"
            err "create it once with: rclone config"
            exit 1
        fi
        ;;
esac

if [ ! -d "$DEST" ]; then
    if [ "$DIRECTION" = push ]; then
        err "local copy not found: $DEST"
        err "nothing to push"
        exit 1
    fi
    mkdir -p "$DEST" || {
        err "cannot create local copy dir: $DEST"
        err "on Termux run: termux-setup-storage"
        exit 1
    }
    log "created local copy dir $DEST"
fi

# Every rclone call goes through here so the filter set is identical for
# copies, listings and moves. Directory patterns need the trailing slash:
# "--exclude .git" does NOT exclude the contents of a .git directory.
rclone_x() {
    rclone "$@" \
        --exclude '.git/' \
        --exclude 'target/' \
        --exclude 'node_modules/' \
        --exclude '.venv/' \
        --exclude 'dist/' \
        --exclude '.next/' \
        --exclude "$QUARANTINE_NAME/" \
        --exclude "$MANIFEST_NAME" \
        --exclude '*.bak' \
        --exclude '*.pyc'
}

list_local() { rclone_x lsf -R --files-only "$DEST"; }
list_remote() { rclone_x lsf -R --files-only "$REMOTE:$REMOTE_PATH"; }

# A path coming from the manifest must never escape the tree.
safe_path() {
    case "$1" in
        /*|..|../*|*/../*|*/..) return 1 ;;
        *) return 0 ;;
    esac
}

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

MANIFEST="$DEST/$MANIFEST_NAME"
if [ ! -f "$MANIFEST" ]; then
    : > "$MANIFEST"
fi
LC_ALL=C sort -o "$MANIFEST" "$MANIFEST"

LC_ALL=C list_remote | LC_ALL=C sort > "$TMP/remote.list"
LC_ALL=C list_local | LC_ALL=C sort > "$TMP/local.list"

# Files the user deleted on the client: they are still on the VPS and must
# not be resurrected by a pull. They are removed from the VPS by the push.
LC_ALL=C comm -23 "$MANIFEST" "$TMP/local.list" > "$TMP/gone.list"

while IFS= read -r p; do
    [ -n "$p" ] || continue
    if ! safe_path "$p"; then
        err "unsafe path in the manifest, refusing: $p"
        exit 1
    fi
done < "$TMP/gone.list"

GONE_COUNT=$(wc -l < "$TMP/gone.list" | tr -d ' ')

set -- --update
if [ "$FORCE" -eq 1 ]; then
    set --
fi
if [ "$DRY_RUN" -eq 1 ]; then
    set -- "$@" --dry-run
fi
set -- "$@" --suffix .bak -v

if [ "$DIRECTION" = pull ]; then
    log "direction pull: $REMOTE:$REMOTE_PATH -> $DEST"
    # Skip the files the user deleted locally: the pull must not bring them
    # back. The push is what removes them from the VPS.
    if [ "$GONE_COUNT" -gt 0 ]; then
        set -- "$@" --exclude-from "$TMP/gone.list"
        log "keeping $GONE_COUNT file(s) deleted on the client"
    fi
    rclone_x copy "$@" "$REMOTE:$REMOTE_PATH" "$DEST"

    # Files the VPS deleted are removed from the client too.
    LC_ALL=C comm -23 "$MANIFEST" "$TMP/remote.list" > "$TMP/removed.list"
    REMOVED_COUNT=$(wc -l < "$TMP/removed.list" | tr -d ' ')
    if [ "$REMOVED_COUNT" -gt 0 ] && [ "$DRY_RUN" -eq 0 ]; then
        while IFS= read -r p; do
            [ -n "$p" ] || continue
            safe_path "$p" || { err "unsafe path, refusing: $p"; exit 1; }
            [ -e "$DEST/$p" ] || continue
            target="$DEST/$QUARANTINE_NAME/$TS/$p"
            mkdir -p "$(dirname "$target")"
            mv "$DEST/$p" "$target"
            log "quarantined (deleted on the VPS): $p"
        done < "$TMP/removed.list"
    fi
    log "deletions propagated to the client: $REMOVED_COUNT"

    if [ "$DRY_RUN" -eq 0 ]; then
        cp "$TMP/remote.list" "$MANIFEST"
    else
        log "dry run: manifest not updated"
    fi
else
    log "direction push: $DEST -> $REMOTE:$REMOTE_PATH"
    rclone_x copy "$@" "$DEST" "$REMOTE:$REMOTE_PATH"

    # Files the user deleted on the client are removed from the VPS and kept
    # aside, so a wrong deletion can be undone.
    if [ "$GONE_COUNT" -gt 0 ] && [ "$DRY_RUN" -eq 0 ]; then
        # --files-from-raw treats the manifest paths literally. It has to be
        # used alone (it overrides every other filter), which is fine: the
        # paths come from the manifest and are already vetted.
        rclone move --files-from-raw "$TMP/gone.list" \
            "$REMOTE:$REMOTE_PATH" "$REMOTE:$REMOTE_PATH/$QUARANTINE_NAME/$TS"
        log "quarantined (deleted on the client): $GONE_COUNT file(s) in $REMOTE:$REMOTE_PATH/$QUARANTINE_NAME/$TS"
    elif [ "$GONE_COUNT" -gt 0 ]; then
        log "would remove from the VPS: $GONE_COUNT file(s)"
    else
        log "no deletions to propagate"
    fi

    if [ "$DRY_RUN" -eq 0 ]; then
        # After a push the baseline is what the CLIENT holds. Files the VPS
        # gained on its own are deliberately not part of it: they were never
        # on the client, so they are not local deletions and must survive.
        cp "$TMP/local.list" "$MANIFEST"
    else
        log "dry run: manifest not updated"
    fi
fi

log "overwritten files kept as <name>.bak next to them"
log "manifest: $MANIFEST"
