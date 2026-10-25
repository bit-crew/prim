#!/bin/sh
# install.sh - bootstrap for prim-skills.
#
# Downloads the prebuilt `prim-skills-install` binary (published by GitHub
# Actions in the "latest" release), verifies its sha256, and runs it. The binary
# installs the agent-agnostic skills (own + external) into ~/.agents/skills,
# which Pi scans.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/prim-skills/main/install.sh | sh
#
# Pass `update` to update external skills:
#   curl -fsSL .../install.sh | sh -s -- update

set -eu

REPO="bit-crew/prim-skills"
RELEASE_TAG="latest"
BASE_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}"

log() { printf '\033[1;32m[prim-skills]\033[0m %s\n' "$1"; }
err() { printf '\033[1;31m[prim-skills]\033[0m %s\n' "$1" >&2; }

asset_suffix() {
    _os="$(uname -s)"
    _arch="$(uname -m)"
    case "$_os" in
        Linux) ;;
        *) err "SO no soportado todavia: $_os (por ahora solo Linux)."; return 1 ;;
    esac
    case "$_arch" in
        x86_64|amd64)  echo "x86_64-linux" ;;
        aarch64|arm64) echo "aarch64-linux" ;;
        *) err "Arquitectura no soportada: $_arch (soportadas: x86_64, aarch64)."; return 1 ;;
    esac
}

command -v curl >/dev/null 2>&1 || { err "'curl' no esta instalado."; exit 1; }

suffix="$(asset_suffix)" || exit 1
bin_url="${BASE_URL}/prim-skills-install-${suffix}"
sum_url="${bin_url}.sha256"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
bin="${tmp}/prim-skills-install"

log "Descargando prim-skills-install (${suffix})..."
curl -4 -fsSL -o "$bin" "$bin_url" || { err "no se pudo descargar el binario."; exit 1; }

# Integrity check (non-fatal if no sha256sum/shasum).
if curl -4 -fsSL -o "${tmp}/sum" "$sum_url" 2>/dev/null; then
    expected="$(awk '{print $1}' "${tmp}/sum")"
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$bin" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$bin" | awk '{print $1}')"
    else
        actual=""
    fi
    if [ -n "$actual" ] && [ "$actual" != "$expected" ]; then
        err "checksum no coincide. Abortando."
        exit 1
    fi
    [ -n "$actual" ] && log "Checksum verificado."
else
    err "no se pudo descargar el .sha256 (continuo sin verificar)."
fi

chmod +x "$bin"
log "Ejecutando prim-skills-install ${*}..."
"$bin" "$@"
log "Listo."
