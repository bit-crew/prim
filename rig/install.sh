#!/bin/sh
# install.sh - compat stub for the old rig entry point (P5).
#
# rig no longer has its own installer: prim manages it over Nix
# (home-manager against the global flake in rig/nix). This stub exists only
# so old links/bookmarks keep working: it downloads the repo root
# install.sh and pipes it to sh with the equivalent prim arguments.
#
# Old invocations still map to the prim lifecycle:
#   sh rig/install.sh [args]      -> prim install rig [args]
#   sh rig/install.sh update      -> prim update
#   sh rig/install.sh uninstall   -> prim uninstall rig

set -eu

ROOT_INSTALL_URL="https://raw.githubusercontent.com/bit-crew/prim/main/install.sh"

log() { printf '\033[1;32m[rig]\033[0m %s\n' "$1"; }
err() { printf '\033[1;31m[rig]\033[0m %s\n' "$1" >&2; }

log "rig ya no tiene instalador propio: prim lo gestiona sobre Nix."
log "Redirigiendo al instalador raiz de prim..."

# Map the old verb contract to prim's; anything else (bare curls, profiles)
# rides along as extra args of `install rig` (prim ignores unknown tokens
# and consumes --profile itself).
case "${1:-}" in
    install)   shift; set -- install rig "$@" ;;
    update)    shift; set -- update ;;
    uninstall) shift; set -- uninstall rig ;;
    *)         set -- install rig "$@" ;;
esac

command -v curl >/dev/null 2>&1 || { err "'curl' no esta instalado."; exit 1; }

curl -4 -fsSL "$ROOT_INSTALL_URL" | sh -s -- "$@"
