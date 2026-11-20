#!/bin/sh
# install-herramientas.sh - install the data and markdown viewers on this client.
#
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/rig/client/install-herramientas.sh | sh
#
# What it does:
#   1. installs the Termux packages glow and miller (binary: mlr)
#   2. installs the static qsv binary into ~/.local/bin
#   3. prints how to try them
#
# Why this is a hand applied script and not a home-manager profile: the
# Termux home is /data/data/com.termux/files/home, not /home/<user>, so the
# dc1 profile cannot write into the device yet (see rig/nix/modules/termux.nix).
# Until that is settled, the client installs what it needs by hand, the same
# way rig/client/install-img-grab.sh does.
#
# Idempotent: running it again is safe. What is already there is left alone
# unless FORCE=1.
#
# QSV_VERSION pins the upstream release; bump it when a new one is out. The
# musl build is statically linked, so it runs on Android without glibc.
#
#   QSV_VERSION=30.0.0 sh install-herramientas.sh

set -eu

PROG=${0##*/}
BIN_DIR="$HOME/.local/bin"
QSV_VERSION="${QSV_VERSION:-29.0.0}"
QSV_TARGET="aarch64-unknown-linux-musl"
# sha256 of the upstream release zip. Bump both together, never one alone.
QSV_SHA256="${QSV_SHA256:-32331cea4d2c013ac5ce066e4fe6f791c312f3472f943f419c713d426bf04313}"
FORCE="${FORCE:-0}"

log() { printf '[herramientas-install] %s\n' "$1"; }
err() { printf '[herramientas-install] %s\n' "$1" >&2; }

usage() {
    cat <<EOF
Usage: $PROG

Installs glow, miller (mlr) and qsv on this Termux client. Safe to run
more than once.

Environment:
  QSV_VERSION   upstream qsv release to install (default: $QSV_VERSION)
  QSV_SHA256    sha256 of that release zip (must match the version)
  FORCE         set to 1 to reinstall qsv even if it is already there
EOF
}

case "${1:-}" in
    -h|--help) usage; exit 0 ;;
    "") : ;;
    *) err "unknown argument: $1"; usage >&2; exit 1 ;;
esac

have() { command -v "$1" >/dev/null 2>&1; }

# ---- where are we? ---------------------------------------------------------

if [ -z "${PREFIX:-}" ] || [ ! -d "${PREFIX}" ]; then
    err "this installer is for the Termux client (the Daylight DC-1)."
    err "PREFIX is empty here, so pkg is not available."
    exit 1
fi

if [ "$(uname -m)" != "aarch64" ]; then
    err "expected an aarch64 device; this one reports $(uname -m)"
    exit 1
fi

if ! have curl; then
    err "curl is missing; run: pkg install curl"
    exit 1
fi

# ---- Termux packages -------------------------------------------------------

install_pkg() {
    package="$1"
    binary="$2"
    if have "$binary"; then
        log "$binary already installed"
        return 0
    fi
    log "installing $package (pkg)"
    pkg install -y "$package"
}

install_pkg glow glow
install_pkg miller mlr

if ! have unzip; then
    log "installing unzip (pkg)"
    pkg install -y unzip
fi

# ---- qsv (static musl build) -----------------------------------------------

mkdir -p "$BIN_DIR"

QSV_URL="https://github.com/dathere/qsv/releases/download/$QSV_VERSION/qsv-$QSV_VERSION-$QSV_TARGET.zip"

if [ -x "$BIN_DIR/qsv" ] && [ "$FORCE" != "1" ]; then
    log "qsv already installed at $BIN_DIR/qsv"
else
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT INT TERM
    log "downloading qsv $QSV_VERSION ($QSV_TARGET)"
    curl -fsSL --max-time 300 "$QSV_URL" -o "$tmp/qsv.zip"
    actual=$(sha256sum "$tmp/qsv.zip" | cut -d' ' -f1)
    if [ "$actual" != "$QSV_SHA256" ]; then
        err "checksum mismatch for $QSV_URL"
        err "expected $QSV_SHA256"
        err "got      $actual"
        exit 1
    fi
    log "checksum ok"
    unzip -o -j "$tmp/qsv.zip" qsv -d "$tmp" >/dev/null
    install -m 755 "$tmp/qsv" "$BIN_DIR/qsv"
    log "installed $BIN_DIR/qsv"
fi

# ---- did it work? ---------------------------------------------------------

verify() {
    label="$1"
    shift
    if "$@" --version >/dev/null 2>&1; then
        log "ok: $label"
    else
        err "not working: $label"
    fi
}

verify glow glow --version
verify miller mlr --version
verify qsv "$BIN_DIR/qsv" --version

# ---- how to try it ---------------------------------------------------------

cat <<EOF

Listo. Probalo asi:

  glow README.md                 # markdown renderizado en la terminal
  glow -p -w 120 docs/guia.md    # modo pager, ancho 120 (las tablas se cortan)
  mlr --csv head -n 5 datos.csv  # primeras filas
  mlr browse datos.csv           # browser interactivo del CSV
  qsv stats datos.csv            # estadisticas por columna
  qsv frequency datos.csv -c categoria

Los CSV que subis desde el telefono al share quedan en ~/work del VPS, asi
que tambien podes mirarlos desde alla, sin copias:

  ssh vps 'mlr --csv head -n 5 ~/work/datos.csv'
  ssh vps 'duckdb -c "select * from ~/work/datos.csv" limit 5'

Para actualizar qsv:

  QSV_VERSION=<nueva> sh $PROG
EOF
