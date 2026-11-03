#!/bin/sh
# install.sh - entry point for the prim ecosystem.
#
# Downloads the prebuilt `prim` orchestrator binary (published by GitHub Actions
# in the "latest" release), verifies its sha256, installs it permanently as
# ~/.local/bin/prim (prim owns its own binary; `prim update` self-updates it
# later), and runs it. The orchestrator manages the component lifecycle
# (install/update/uninstall/status) for the ecosystem pieces (rig = dev
# environment, harness = agent config, core = operating core) and invokes each
# component's own installer on demand, forwarding the verb.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/install.sh | sh

set -eu

REPO="bit-crew/prim"
RELEASE_TAG="latest"
BASE_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}"

log() { printf '\033[1;32m[prim]\033[0m %s\n' "$1"; }
err() { printf '\033[1;31m[prim]\033[0m %s\n' "$1" >&2; }

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
bin_url="${BASE_URL}/prim-${suffix}"
sum_url="${bin_url}.sha256"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
bin="${tmp}/prim"

log "Descargando prim (${suffix})..."
curl -4 -fsSL -o "$bin" "$bin_url" || { err "no se pudo descargar el binario."; exit 1; }

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

# Permanent install: prim owns its own binary in ~/.local/bin so `prim update`
# has something to self-update later. Non-fatal: if the copy fails we keep
# running the downloaded tmp binary for this session.
perm_bin="${HOME}/.local/bin/prim"
if mkdir -p "${HOME}/.local/bin" 2>/dev/null \
    && cp "$bin" "$perm_bin" 2>/dev/null \
    && chmod 755 "$perm_bin" 2>/dev/null; then
    log "prim instalado en ${perm_bin} (auto-actualizable con 'prim update')."
else
    err "no se pudo copiar a ${perm_bin}; sigo con el binario temporal."
fi

chmod +x "$bin"
"$bin" "$@"
