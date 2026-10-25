#!/bin/sh
# install.sh - punto de entrada unico de devlab.
#
# Descarga el binario precompilado `devlab-install` (publicado por GitHub
# Actions en la release "latest") y lo ejecuta. Reemplaza a los antiguos
# setup.sh / bootstrap.sh: toda la logica de instalacion vive ahora en el
# binario Rust.
#
# Uso directo (sin clonar nada):
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/devlab/main/install.sh | sh
#
# Pregunta primero QUE configurar:
#   1) Cliente (este equipo)  -> baja y ejecuta el binario aqui (perfil dc1)
#   2) VPS remoto             -> se conecta por SSH y lo instala alla (perfil vps)
#
# Tambien acepta el rol como argumento:  sh install.sh dc1   |   sh install.sh vps

set -eu

REPO="bit-crew/devlab"
RELEASE_TAG="latest"
BASE_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}"
RAW_URL="https://raw.githubusercontent.com/${REPO}/main/install.sh"

log() { printf '\033[1;32m[install]\033[0m %s\n' "$1"; }
err() { printf '\033[1;31m[install]\033[0m %s\n' "$1" >&2; }

# Con `curl | sh`, stdin es el script, no el teclado. Leemos de /dev/tty para
# que las preguntas interactivas funcionen igual en pipe o ejecucion normal.
if [ -r /dev/tty ]; then
    TTY=/dev/tty
else
    TTY=/dev/stdin
fi

ask() {
    # ask <prompt> <default> -> imprime el valor leido (o el default)
    _prompt="$1"; _default="${2:-}"
    if [ -n "$_default" ]; then
        printf '%s [%s]: ' "$_prompt" "$_default" >/dev/tty
    else
        printf '%s: ' "$_prompt" >/dev/tty
    fi
    read -r _ans <"$TTY" || _ans=""
    [ -z "$_ans" ] && _ans="$_default"
    printf '%s' "$_ans"
}

# ---------------------------------------------------------------------------
# Mapea uname -s/-m al sufijo del asset publicado por el CI.
# El CI compila: x86_64-linux (glibc) y aarch64-linux (musl estatico). El
# binario aarch64 es estatico, asi que corre en Linux ARM y en Android/Termux
# (que usa bionic libc). Termux reporta uname -s = Linux.
# ---------------------------------------------------------------------------
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

# ---------------------------------------------------------------------------
# Cliente / local: descarga el binario para este host, verifica el sha256 y
# lo ejecuta con el perfil indicado (dc1 o vps).
# ---------------------------------------------------------------------------
install_local() {
    _profile="$1"
    command -v curl >/dev/null 2>&1 || { err "'curl' no esta instalado."; exit 1; }

    _suffix="$(asset_suffix)" || exit 1
    _bin_url="${BASE_URL}/devlab-install-${_suffix}"
    _sum_url="${_bin_url}.sha256"

    _tmp="$(mktemp -d)"
    trap 'rm -rf "$_tmp"' EXIT
    _bin="${_tmp}/devlab-install"

    log "Descargando devlab-install (${_suffix})..."
    curl -4 -fsSL -o "$_bin" "$_bin_url" || { err "no se pudo descargar el binario."; exit 1; }

    # Verificacion de integridad (no fatal si no hay sha256sum/shasum).
    if curl -4 -fsSL -o "${_tmp}/sum" "$_sum_url" 2>/dev/null; then
        _expected="$(awk '{print $1}' "${_tmp}/sum")"
        if command -v sha256sum >/dev/null 2>&1; then
            _actual="$(sha256sum "$_bin" | awk '{print $1}')"
        elif command -v shasum >/dev/null 2>&1; then
            _actual="$(shasum -a 256 "$_bin" | awk '{print $1}')"
        else
            _actual=""
        fi
        if [ -n "$_actual" ] && [ "$_actual" != "$_expected" ]; then
            err "checksum no coincide. Abortando."
            err "  esperado: $_expected"
            err "  obtenido: $_actual"
            exit 1
        fi
        [ -n "$_actual" ] && log "Checksum verificado."
    else
        err "no se pudo descargar el .sha256 (continuo sin verificar)."
    fi

    chmod +x "$_bin"
    log "Ejecutando devlab-install --profile ${_profile}..."
    DEVLAB_PROFILE="$_profile" "$_bin" --profile "$_profile"
    log "Instalacion local completa (perfil ${_profile})."
}

# ---------------------------------------------------------------------------
# VPS remoto: se conecta por SSH y corre este mismo install.sh en modo cliente
# alla (perfil vps). El VPS solo necesita curl; el binario hace el resto.
# ---------------------------------------------------------------------------
install_vps_remote() {
    command -v ssh >/dev/null 2>&1 || { err "'ssh' no esta instalado."; exit 1; }

    log "Datos del VPS (Enter para aceptar el valor entre corchetes):"
    _host="$(ask 'IP publica del VPS' '')"
    _user="$(ask 'Usuario SSH' 'debian')"
    [ -n "$_host" ] || { err "IP vacia. Aborto."; exit 1; }

    log "Objetivo: ${_user}@${_host}"
    log "Se conectara por SSH; escribe la password del VPS si la pide."
    log "Instalando el entorno en el VPS (puede tardar varios minutos)..."

    # -t: asigna TTY para que un eventual prompt de sudo pueda responderse.
    # En el VPS: aseguramos curl, luego corremos este install.sh en modo dc1-local
    # con perfil vps (DEVLAB_PROFILE=vps lo hace no-interactivo para el binario).
    ssh -t -o StrictHostKeyChecking=accept-new "${_user}@${_host}" "
        if ! command -v curl >/dev/null 2>&1; then
            sudo apt-get update -y &&
            sudo DEBIAN_FRONTEND=noninteractive apt-get install -y curl;
        fi &&
        curl -4 -fsSL ${RAW_URL} | DEVLAB_ROLE=vps-local sh
    "

    log "VPS configurado con el entorno devlab."
    log "Entra con: ssh ${_user}@${_host}  (y ejecuta 'zsh' para el entorno)"
}

# ---------------------------------------------------------------------------
# Punto de entrada: preguntar rol (o aceptarlo como argumento).
#   DEVLAB_ROLE=vps-local  -> salta la pregunta y hace install_local vps
#   (lo usa el modo remoto cuando ya esta dentro del VPS).
# ---------------------------------------------------------------------------
ROLE="${1:-${DEVLAB_ROLE:-}}"

if [ "$ROLE" = "vps-local" ]; then
    install_local "vps"
    exit 0
fi

if [ -z "$ROLE" ]; then
    printf '\n¿Que quieres configurar?\n' >/dev/tty
    printf '  1) Cliente (este equipo)\n' >/dev/tty
    printf '  2) VPS remoto\n' >/dev/tty
    CHOICE="$(ask 'Opcion' '1')"
    case "$CHOICE" in
        1|dc1|cliente|client) ROLE="dc1" ;;
        2|vps|servidor|server) ROLE="vps" ;;
        *) err "Opcion invalida: $CHOICE"; exit 1 ;;
    esac
fi

case "$ROLE" in
    dc1) install_local "dc1" ;;
    vps) install_vps_remote ;;
    *)   err "Rol invalido: $ROLE (usa dc1 o vps)"; exit 1 ;;
esac
