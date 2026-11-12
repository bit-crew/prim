#!/bin/sh
# install-img-grab.sh - install the clipboard image grab on this client.
#
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/rig/client/install-img-grab.sh | sh
#
# What it does:
#   1. installs img-grab into ~/.local/bin
#   2. wires the key binding for the terminal it detects (Ghostty and kitty
#      are edited in place; for WezTerm and Termux the snippet is printed
#      instead, because appending to those configs would clobber settings
#      that are not ours)
#   3. prints how to try it
#
# Idempotent: running it again is safe and does not duplicate the binding.
#
# RAW_BASE overrides where img-grab is downloaded from (default: main).
#   RAW_BASE=https://raw.githubusercontent.com/bit-crew/prim/feat/clipboard-inbox \
#     sh install-img-grab.sh

set -eu

PROG=${0##*/}
RAW_BASE="${RAW_BASE:-https://raw.githubusercontent.com/bit-crew/prim/main}"
BIN_DIR="$HOME/.local/bin"
TARGET="$BIN_DIR/img-grab"
HOME_DIR=$(cd "$HOME" && pwd)

log() { printf '[img-grab-install] %s\n' "$1"; }
err() { printf '[img-grab-install] %s\n' "$1" >&2; }

usage() {
    cat <<EOF
Usage: $PROG

Installs img-grab into ~/.local/bin and wires the key binding for the
terminal it detects. Safe to run more than once.

Environment:
  RAW_BASE   where img-grab is downloaded from
              (default: $RAW_BASE)
EOF
}

case "${1:-}" in
    -h|--help) usage; exit 0 ;;
    "") : ;;
    *) err "unknown argument: $1"; usage >&2; exit 1 ;;
esac

mkdir -p "$BIN_DIR"

# Where does img-grab come from? A local install copies the sibling file; a
# piped install downloads it.
if [ -f "$0" ] && [ -f "$(dirname "$0")/img-grab" ]; then
    install -m 755 "$(dirname "$0")/img-grab" "$TARGET"
    log "installed $TARGET (from $(dirname "$0")/img-grab)"
else
    URL="$RAW_BASE/rig/client/img-grab"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$URL" -o "$TARGET.tmp"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$TARGET.tmp" "$URL"
    else
        err "neither curl nor wget found; cannot download img-grab"
        exit 1
    fi
    chmod 755 "$TARGET.tmp"
    mv "$TARGET.tmp" "$TARGET"
    log "installed $TARGET (from $URL)"
fi

[ -x "$TARGET" ] || { err "img-grab did not end up executable"; exit 1; }

# ---- the binding -----------------------------------------------------------

add_line() {
    # add_line <file> <line>: append once, keeping a marker comment.
    file="$1"
    line="$2"
    marker="# prim-clipboard: clipboard image -> VPS inbox"
    mkdir -p "$(dirname "$file")"
    if [ -f "$file" ] && grep -qF "$marker" "$file"; then
        log "binding already present in $file"
        return 0
    fi
    exists=0
    if [ -f "$file" ]; then
        exists=1
        cp "$file" "$file.bak"
        log "backup: $file.bak"
    fi
    if [ "$exists" -eq 1 ]; then
        printf '\n' >> "$file"
    fi
    printf '%s\n' "$marker" >> "$file"
    printf '%s\n' "$line" >> "$file"
    log "binding written in $file"
}

case "${TERM_PROGRAM:-}" in
    ghostty)
        add_line "$HOME_DIR/.config/ghostty/config" \
            "keybind = CTRL+SHIFT+G=exec:$TARGET"
        ;;
    kitty)
        add_line "$HOME_DIR/.config/kitty/kitty.conf" \
            "map ctrl+shift+g launch --type=background $TARGET"
        ;;
    WezTerm|iTerm.app)
        log "WezTerm/iTerm2 config not edited (it is a Lua file): add this by hand"
        printf '  config.keys = {\n    { key = "G", mods = "CTRL|SHIFT", action = WezTermSpawnCommand.new("%s") },\n  }\n' "$TARGET"
        ;;
    *)
        if [ -n "${PREFIX:-}" ] && [ -d "$HOME_DIR/.termux" ]; then
            log "Termux: add this line to ~/.termux/termux.properties"
            printf '  extra-keys = CTRL+ALT+G,%s\\n\n' "$TARGET"
        else
            log "terminal not detected; here are the snippets"
            printf '  Ghostty (~/.config/ghostty/config):\n    keybind = CTRL+SHIFT+G=exec:%s\n' "$TARGET"
            printf '  kitty (~/.config/kitty/kitty.conf):\n    map ctrl+shift+g launch --type=background %s\n' "$TARGET"
            printf '  WezTerm (wezterm.lua):\n    { key = "G", mods = "CTRL|SHIFT", action = WezTermSpawnCommand.new("%s") }\n' "$TARGET"
            printf '  Termux (~/.termux/termux.properties):\n    extra-keys = CTRL+ALT+G,%s\\n\n' "$TARGET"
        fi
        ;;
esac

# ---- how to try it ---------------------------------------------------------

cat <<EOF

Listo. Probalo asi:

  1. copiá una imagen al portapapeles
  2. $TARGET --dry-run      # muestra el plan sin subir nada
  3. $TARGET                # la sube y deja la ruta en el portapapeles
  4. pegá la ruta (Cmd+V) en la sesion del agente

La imagen queda en el inbox del VPS (~/.cache/prim/inbox) y la limpieza
borra lo que tenga mas de 12 horas, dos veces por dia.

Si el remoto no se llama "vps", pasale el host: $TARGET --host <tu-alias-ssh>
EOF
