#!/bin/sh
# install.sh - the `files` component (S9-S11): SMB share of ~/work for
# iPhone (Files), Android, Windows and Linux — reachable ONLY inside the
# tailnet, because smbd binds to tailscale0 and no port is exposed.
#
# Part of the workstation (odd/tasks/rig-workstation-files.md, S1/S2):
# `prim install rig` with the vps profile chains this installer right after
# the home-manager activation, so a fresh VPS ends with ~/work shared,
# empty or full. Client profiles never run it; an explicit `prim install
# files` reinstalls/repairs it.
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
# The generated SMB password: owner-only, never in the receipt, never in args.
CRED_DIR="${HOME}/.local/state/prim/files"
CRED_FILE="${CRED_DIR}/smb-password"
TS_INSTALL_URL="https://tailscale.com/install.sh"

# Firewall: prim owns the host baseline (rig/vps/harden-vps.sh, table
# vps_hardening, default-deny). The SMB port belongs to THIS stack, so it
# lives in the baseline's extension dir: harden-vps.sh re-applies these
# fragments after every re-apply, and smbd re-adds the rule on every start.
NFT_TABLE="vps_hardening"
NFT_EXT_DIR="/etc/nftables.d/prim-ext"
NFT_EXT_FILE="${NFT_EXT_DIR}/files.nft"
SMB_FIREWALL_RULE='insert rule inet vps_hardening input iifname "tailscale0" tcp dport 445 accept'
SMBD_DROPIN="/etc/systemd/system/smbd.service.d/10-prim-files-firewall.conf"

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
    ensure_home_traversable
}

ensure_home_traversable() {
    # The share lives INSIDE the user's home. Samba authenticates as
    # filesmb, and that account must be able to TRAVERSE the home to
    # reach ~/work at all: with a 0700 home every file operation fails
    # with NT_STATUS_ACCESS_DENIED even when ~/work's group rights are
    # perfect (verified: listing the share worked, put/get/ls did not).
    #
    # The fix is traversal only (x): the home stays unlistable, only
    # reachable by a known path. Preferred form is an ACL (g:primfiles:--x)
    # because it grants traversal to the share group alone; the mode
    # change (o+x) reaches EVERY local account, so it is only kept while
    # the ACL path is unavailable or unproven.
    if sudo -u "$SMB_USER" test -x "$HOME" 2>/dev/null; then
        log "home atravessable por '$SMB_USER' (ya estaba bien)."
        return 0
    fi
    if command -v setfacl >/dev/null 2>&1 && sudo setfacl -m "g:$GROUP:--x" "$HOME" 2>/dev/null; then
        # Only drop the world-execute bit once the ACL is proven to give
        # filesmb the traversal: a silent ACL failure must not lock the
        # share out again.
        if sudo -u "$SMB_USER" test -x "$HOME" 2>/dev/null; then
            sudo chmod o-x "$HOME" 2>/dev/null || true
            log "home con ACL de traversing para el grupo $GROUP (traversal, no listado)."
        else
            sudo chmod o+x "$HOME"
            log "home con permiso de traversing (o+x): el ACL no dio traversal a $SMB_USER."
        fi
        return 0
    fi
}

smb_conf_body() {
    # Minimal S10 config: no printing, no nmbd dependencies, tailnet-only.
    #
    # Samba CANNOT bind to tailscale0: its interface enumeration skips
    # point-to-point interfaces, so both "interfaces = tailscale0" and
    # "interfaces = <ip>" leave smbd with no socket to bind and it dies
    # with "open_sockets_smbd() failed". The tailnet restriction is
    # therefore enforced one layer up, on the client address: only the
    # Tailscale CGNAT range (100.64.0.0/10) is allowed, everything else
    # is denied. The nftables rule (445 only from tailscale0) is the
    # layer below that one.
    cat <<EOF
# $CONF_MARKER — generado por prim (componente files); no editar a mano.
[global]
    security = user
    server role = standalone server
    load printers = no
    disable spoolss = yes
    log level = 1
    hosts allow = 100.64.0.0/10
    hosts deny = ALL

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
    log "smb.conf minimo escrito: solo clientes del tailnet (100.64.0.0/10)."
}

firewall_apply() {
    sudo mkdir -p "$NFT_EXT_DIR"
    printf '%s\n' "$SMB_FIREWALL_RULE" | sudo tee "$NFT_EXT_FILE" >/dev/null
    if sudo nft list chain inet "$NFT_TABLE" input 2>/dev/null | grep -q 'dport 445'; then
        log "firewall: tcp 445 (smb) ya permitido en ${NFT_TABLE}."
    else
        sudo nft -f "$NFT_EXT_FILE" &&
            log "firewall: tcp 445 (smb) permitido solo por tailscale0."
    fi
    # smbd re-adds the rule before it starts: the baseline deletes and
    # recreates the table on every re-apply, and a reboot loads it fresh.
    sudo mkdir -p "$(dirname "$SMBD_DROPIN")"
    sudo tee "$SMBD_DROPIN" >/dev/null <<EOF
[Unit]
# The baseline table must exist before smbd re-adds its rule: nftables
# and smbd both start at boot and nothing orders them by default.
After=nftables.service

[Service]
ExecStartPre=-/bin/sh -c 'nft list chain inet ${NFT_TABLE} input 2>/dev/null | grep -q "dport 445" || nft -f ${NFT_EXT_FILE}'
EOF
    sudo systemctl daemon-reload >/dev/null 2>&1 || true
}

firewall_remove() {
    sudo rm -f "$NFT_EXT_FILE" "$SMBD_DROPIN"
    sudo systemctl daemon-reload >/dev/null 2>&1 || true
    for handle in $(sudo nft -a list chain inet "$NFT_TABLE" input 2>/dev/null |
        grep 'dport 445' | awk '{print $NF}'); do
        sudo nft delete rule inet "$NFT_TABLE" input handle "$handle" >/dev/null 2>&1 || true
    done
    log "firewall: tcp 445 (smb) retirado de ${NFT_TABLE}."
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
    # The password is GENERATED, never typed and never asked for
    # interactively: the user does not want a manual step, and a prompt
    # in a non-TTY install is exactly what wedged this component before.
    #
    #   - stored once, owner-only, in ~/.local/state/prim/files/
    #   - the secret never travels in args (ps), env, or the receipt
    #   - 'smbpasswd -a -s' takes the password from stdin, so no TTY is
    #     needed at all
    #
    # Idempotency: if our credential file AND the passdb agree, the
    # password is kept untouched (a re-install never invalidates the key
    # already saved in the phone). If the passdb has a password we cannot
    # read back (file lost, host migrated), the only honest fix is a new
    # one: we rotate it and store the new truth.
    if [ -s "$CRED_FILE" ] && sudo pdbedit -L -u "$SMB_USER" >/dev/null 2>&1; then
        log "clave SMB de '$SMB_USER' ya definida; se conserva."
        SMB_PASSWORD_SET=true
        return 0
    fi
    if sudo pdbedit -L -u "$SMB_USER" >/dev/null 2>&1; then
        log "hay una clave SMB previa sin copia local; se genera una nueva."
    fi
    rotate_smb_password
}

rotate_smb_password() {
    # 24 bytes from the kernel CSPRNG, hex: 48 chars, 192 bits. od with a
    # fixed count avoids the SIGPIPE that a `tr | head` pipeline risks.
    pw="$(LC_ALL=C od -An -tx1 -N24 /dev/urandom | tr -d ' \n')"
    if [ "${#pw}" -lt 32 ]; then
        err "no se pudo generar la clave SMB (PRNG sin datos)."
        exit 1
    fi
    mkdir -p "$CRED_DIR"
    (umask 077 && printf '%s\n' "$pw" > "$CRED_FILE")
    chmod 600 "$CRED_FILE" 2>/dev/null || true
    # smbpasswd reads "password" and "confirmation" from stdin with -s.
    if ! printf '%s\n%s\n' "$pw" "$pw" | sudo smbpasswd -a -s "$SMB_USER"; then
        err "smbpasswd fallo; la clave no quedo aplicada."
        rm -f "$CRED_FILE"
        exit 1
    fi
    SMB_PASSWORD_SET=true
    log "clave SMB generada para '$SMB_USER' (48 chars, aleatoria, 192 bits)."
    log "  $pw"
    log "  guardada en $CRED_FILE (permiso 600). Volvela a ver con: prim files password"
    log "  Copiala al keychain del iPhone; despues no hace falta volver a escribirla."
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
        printf '"smb_password_set": "%s",\n' "${SMB_PASSWORD_SET:-false}"
        printf '"credential_file": "%s",\n' "$CRED_FILE"
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
    if [ "${SMB_PASSWORD_SET:-false}" = "false" ]; then
        log "PENDIENTE: la clave SMB de '$SMB_USER' no quedo definida."
        log "  generala con: prim files password --rotate"
    fi
}

cmd_rotate_password() {
    # Verb for 'prim files password --rotate': only the credential
    # changes, nothing else in the host.
    require_sudo
    ensure_user_group
    rotate_smb_password
    write_receipt
}


cmd_install() {
    require_sudo
    log "instalando samba (apt)..."
    sudo apt-get update
    # noninteractive + needrestart in list mode: an apt run must never
    # block on a debconf dialog (needrestart pops one for the kernel and
    # another asking which daemons to restart, and restarting sshd there
    # kills the very session running the install).
    sudo env DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l apt-get install -y samba acl
    if command -v tailscale >/dev/null 2>&1; then
        log "tailscale ya esta instalado."
    else
        command -v curl >/dev/null 2>&1 || {
            err "curl no esta instalado (necesario para tailscale)."
            exit 1
        }
        log "instalando tailscale (instalador oficial, idempotente)..."
        curl -fsSL "$TS_INSTALL_URL" | sudo env DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l sh
    fi
    ensure_user_group
    apply_share_perms
    write_smb_conf
    mask_nmbd
    sudo systemctl enable --now tailscaled
    log "tailscaled habilitado y arrancado."
    tailscale_login
    firewall_apply
    # enable + restart: smbd may already be running with the previous
    # smb.conf, and 'enable --now' would NOT rebind it to tailscale0.
    sudo systemctl enable smbd
    sudo systemctl restart smbd
    log "smbd habilitado y arrancado."
    set_smb_password
    write_receipt
    summary
}

cmd_update() {
    require_sudo
    log "actualizando samba..."
    sudo env DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l apt-get install --only-upgrade -y samba
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
    firewall_apply
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
    # shellcheck disable=SC2088 # literal ~/work in a user-facing message
    log "~/work NO se toca: tus archivos y permisos quedan como estan (S4)."
    require_sudo
    sudo systemctl disable --now smbd >/dev/null 2>&1 || true
    log "smbd detenido y deshabilitado."
    firewall_remove
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
    rotate-password) cmd_rotate_password ;;
    *)
        err "uso: files/install.sh [install|update|uninstall|rotate-password] (por defecto: install)"
        err "para fijar tu propia clave: prim files password --set"
        exit 2
        ;;
esac
