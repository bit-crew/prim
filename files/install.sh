#!/bin/sh
# install.sh - optional `files` component (S9-S11): SMB share of ~/work for
# iPhone (Files), Android, Windows and Linux — reachable ONLY inside the
# tailnet, because smbd binds to tailscale0 and no port is exposed.
#
# Verb contract (prim machinery): $1 = install|update|uninstall (default
# install). Everything is idempotent; re-running any verb is safe.
#
# Design notes:
#   S4   - ~/work is never deleted or renamed; the perms change is dirs-only.
#   S10  - only smbd ships (always-on, ~30MB idle); nmbd is disabled+masked;
#          smbd socket activation was evaluated and rejected as unreliable.
#   S11c - dedicated system user `filesmb` + group `primfiles` (RW via group
#          + setgid on ~/work), never the login user.
#   S11f - tailscale/tailscaled are KEPT on uninstall (network base).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/files/install.sh | sh
#   curl -fsSL .../files/install.sh | sh -s -- update
#   curl -fsSL .../files/install.sh | sh -s -- uninstall

set -eu

log() { printf '\033[1;32m[files]\033[0m %s\n' "$1"; }
err() { printf '\033[1;31m[files]\033[0m %s\n' "$1" >&2; }

GROUP="primfiles"
SMB_USER="filesmb"
INVOKER="$(id -un)"
WORK_DIR="${HOME:?HOME no definido}/work"
SMB_CONF="/etc/samba/smb.conf"
SMB_CONF_BAK="/etc/samba/smb.conf.prim.bak"
CONF_MARKER="prim:files"
RECEIPT="${HOME}/.local/share/prim/files.receipt.json"
TS_INSTALL_URL="https://tailscale.com/install.sh"

require_sudo() {
    if ! sudo -n true 2>/dev/null; then
        err "este componente necesita sudo con credencial cacheada."
        err "ejecuta 'sudo -v' en esta terminal y reintenta: prim install files"
        exit 1
    fi
}

ensure_user_group() {
    if getent group "$GROUP" >/dev/null 2>&1; then
        log "grupo $GROUP ya existe."
    else
        sudo groupadd "$GROUP"
        log "grupo $GROUP creado."
    fi
    if id "$SMB_USER" >/dev/null 2>&1; then
        log "usuario $SMB_USER ya existe."
    else
        sudo useradd --system --gid "$GROUP" --home-dir /nonexistent \
            --no-create-home --shell /usr/sbin/nologin "$SMB_USER"
        log "usuario de sistema $SMB_USER creado en $GROUP."
    fi
    sudo usermod -aG "$GROUP" "$INVOKER"
    log "$INVOKER pertenece a $GROUP (aplica tras re-login)."
}

apply_share_perms() {
    mkdir -p "$WORK_DIR"
    # S4: directories only, and never delete or rename anything under ~/work.
    # setgid makes new children inherit $GROUP, so filesmb and the invoking
    # user share RW access (S11c).
    sudo find "$WORK_DIR" -type d -exec chgrp "$GROUP" {} +
    sudo find "$WORK_DIR" -type d -exec chmod g+rwxs {} +
    log "permisos: directorios de $WORK_DIR en grupo $GROUP con setgid."
}

smb_conf_body() {
    # Minimal S10 config: no printing, no nmbd dependencies, tailnet-only.
    cat <<EOF
# $CONF_MARKER — generado por prim (componente files); no editar a mano.
[global]
    security = user
    server role = standalone server
    load printers = no
    disable spoolss = yes
    log level = 1
    interfaces = tailscale0
    bind interfaces only = yes

[work]
    path = $WORK_DIR
    valid users = @$GROUP
    read only = no
    force create mode = 0664
    force directory mode = 0775
    follow symlinks = yes
EOF
}

write_smb_conf() {
    # Backup any pre-existing smb.conf once; re-runs see our marker in the
    # file and never overwrite the backup with a prim-written config.
    if sudo test -f "$SMB_CONF" && ! sudo grep -q "$CONF_MARKER" "$SMB_CONF"; then
        if ! sudo test -f "$SMB_CONF_BAK"; then
            sudo cp "$SMB_CONF" "$SMB_CONF_BAK"
            log "smb.conf previo respaldado en $SMB_CONF_BAK."
        fi
    fi
    smb_conf_body | sudo tee "$SMB_CONF" >/dev/null
    log "smb.conf minimo escrito: escucha solo en tailscale0."
}

mask_nmbd() {
    sudo systemctl disable --now nmbd >/dev/null 2>&1 || true
    sudo systemctl mask nmbd >/dev/null 2>&1 || true
    log "nmbd deshabilitado y enmascarado (S10: solo smbd)."
}

tailscale_login() {
    if sudo tailscale status >/dev/null 2>&1; then
        log "tailscale ya tiene sesion activa."
        return 0
    fi
    if [ -t 0 ]; then
        log "tailscale: abre la URL de autenticacion que aparece y autoriza esta maquina..."
        sudo tailscale up || err "tailscale up fallo; ejecuta 'sudo tailscale up' a mano."
    else
        err "tailscale sin sesion y sin TTY interactivo."
        err "mas tarde ejecuta manualmente: sudo tailscale up"
    fi
}

set_smb_password() {
    # TTY only: smbpasswd itself prompts twice; passwords never travel in
    # args or env (S11).
    if [ -t 0 ]; then
        log "define la clave SMB para '$SMB_USER' (se pide dos veces)."
        sudo smbpasswd -a "$SMB_USER"
        log "clave SMB fijada para '$SMB_USER'."
    else
        err "sin TTY no se puede fijar la clave SMB de '$SMB_USER'."
        err "re-ejecuta interactivo: prim install files"
        exit 1
    fi
}

write_receipt() {
    now_unix="$(date +%s)"
    now_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    mkdir -p "${HOME}/.local/share/prim"
    # Line-oriented JSON (one field per line, quoted strings) so prim's
    # receipt scanner (lifecycle::json_field) reads it without a parser.
    {
        printf '{\n'
        printf '"component": "files",\n'
        printf '"schema": 1,\n'
        printf '"installed_at_unix": %s,\n' "$now_unix"
        printf '"installed_at_iso": "%s",\n' "$now_iso"
        printf '"share_path": "%s",\n' "$WORK_DIR"
        printf '"share_user": "%s",\n' "$SMB_USER"
        printf '"interface": "tailscale0",\n'
        printf '"services": "smbd tailscaled"\n'
        printf '}\n'
    } > "$RECEIPT"
    log "receipt: $RECEIPT"
}

summary() {
    ts_ip="$(sudo tailscale ip -4 2>/dev/null | head -n 1 || true)"
    if [ -n "$ts_ip" ]; then
        log "listo. IP del VPS en tailscale: $ts_ip"
        log "iPhone: Archivos > Conectar al servidor > smb://$ts_ip/work (usuario $SMB_USER)"
    else
        err "no se pudo leer la IP de tailscale; tras 'sudo tailscale up' conecta con:"
        err "iPhone: Archivos > Conectar al servidor > smb://<IP-TAILSCALE>/work"
    fi
}

cmd_install() {
    require_sudo
    log "instalando samba (apt)..."
    sudo apt-get update
    sudo apt-get install -y samba
    if command -v tailscale >/dev/null 2>&1; then
        log "tailscale ya esta instalado."
    else
        command -v curl >/dev/null 2>&1 || {
            err "curl no esta instalado (necesario para tailscale)."
            exit 1
        }
        log "instalando tailscale (instalador oficial, idempotente)..."
        curl -fsSL "$TS_INSTALL_URL" | sudo sh
    fi
    ensure_user_group
    apply_share_perms
    write_smb_conf
    mask_nmbd
    sudo systemctl enable --now tailscaled
    log "tailscaled habilitado y arrancado."
    tailscale_login
    sudo systemctl enable --now smbd
    log "smbd habilitado y arrancado."
    set_smb_password
    write_receipt
    summary
}

cmd_update() {
    require_sudo
    log "actualizando samba..."
    sudo apt-get install --only-upgrade -y samba
    if sudo tailscale update --help >/dev/null 2>&1; then
        sudo tailscale update
        log "tailscale actualizado."
    else
        log "'tailscale update' no disponible; se conserva la version actual."
    fi
    ensure_user_group
    apply_share_perms
    write_smb_conf
    mask_nmbd
    sudo systemctl restart smbd
    log "smbd reiniciado."
    write_receipt
    log "files actualizado."
}

restore_smb_conf() {
    if ! sudo test -f "$SMB_CONF"; then
        return 0
    fi
    if ! sudo grep -q "$CONF_MARKER" "$SMB_CONF"; then
        log "smb.conf actual no fue escrito por prim; no se toca."
        return 0
    fi
    if sudo test -f "$SMB_CONF_BAK"; then
        sudo cp "$SMB_CONF_BAK" "$SMB_CONF"
        sudo rm -f "$SMB_CONF_BAK"
        log "smb.conf previo restaurado."
    else
        sudo rm -f "$SMB_CONF"
        log "smb.conf de prim eliminado."
    fi
}

cmd_uninstall() {
    if [ ! -r "$RECEIPT" ]; then
        err "sin receipt legible ($RECEIPT) no se desinstala."
        err "corre 'prim update' una vez para regenerarlo, o 'prim install files' para reinstalar."
        exit 1
    fi
    # S4 notice FIRST: uninstall never touches ~/work.
    log "~/work NO se toca: tus archivos y permisos quedan como estan (S4)."
    require_sudo
    sudo systemctl disable --now smbd >/dev/null 2>&1 || true
    log "smbd detenido y deshabilitado."
    restore_smb_conf
    sudo smbpasswd -x "$SMB_USER" >/dev/null 2>&1 || true
    if id "$SMB_USER" >/dev/null 2>&1; then
        sudo userdel "$SMB_USER"
        log "usuario $SMB_USER eliminado."
    fi
    if getent group "$GROUP" >/dev/null 2>&1; then
        sudo groupdel "$GROUP" || log "grupo $GROUP sigue en uso; revisalo a mano."
    fi
    log "tailscale y tailscaled se CONSERVAN (base de red del VPS)."
    log "para quitarlos de verdad: sudo tailscale down && sudo apt-get remove tailscale"
    rm -f "$RECEIPT"
    log "receipt eliminado. Desinstalacion completa; ~/work intacto (S4)."
}

case "${1:-install}" in
    install)   cmd_install ;;
    update)    cmd_update ;;
    uninstall) cmd_uninstall ;;
    *)
        err "uso: files/install.sh [install|update|uninstall] (por defecto: install)"
        exit 2
        ;;
esac
