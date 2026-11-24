#!/bin/sh
# harden-vps - general host hardening owned by prim (the VPS-wide config).
#
# Idempotent. Applies and owns two host-level baseline pieces:
#   1. A 2G emergency swapfile (/swapfile, vm.swappiness=10). The VPS has
#      3.7G RAM and ~1.3G free with agents running; without swap the OOM
#      killer shoots random processes during login peaks.
#   2. An nftables input filter that only allows lo, established/related,
#      ICMP/ICMPv6, TCP 22 (ssh) and the mosh UDP range 60000-60010 (only
#      from tailscale0, see rig/vps/mosh.md). Everything else inbound is
#      dropped. TCP 22 is the ONLY port this host exposes to the Internet:
#      it stays open because the VPS is reached from outside the tailnet,
#      and sshd itself is locked down to key-or-password (see 3). Every
#      other port (SMB 445, the mosh UDP range, whatever an app adds) is
#      reachable only through tailscale0, so a tailnet outage leaves SSH
#      working and takes the rest down with it.
#      Service-specific ports are owned by their stacks: e.g. gt's
#      WorkSpaces web client opens its own tcp 8443 allow inside this table
#      (its activate.sh + unit re-add it after every re-apply/reboot).
#      Application-owned ports live in /etc/nftables.d/prim-ext/*.nft (one
#      file per stack, e.g. files' SMB 445): plain nft statements applied
#      AFTER the baseline, so an app rule survives every re-apply.
#      Docker's own tables are NOT flushed.
#   3. An sshd drop-in (/etc/ssh/sshd_config.d/00-prim-hardening.conf,
#      from rig/vps/sshd-hardening.conf): port 22 accepts a public key OR
#      user + password, and nothing else (no keyboard-interactive, no root,
#      only the 'debian' account). The name sorts before 50-cloud-init.conf
#      on purpose: OpenSSH keeps the first value it obtains for a keyword
#      and the Include sits at the top of sshd_config, so cloud-init's
#      "PasswordAuthentication yes" would otherwise win.
#
# Usage (root):
#   harden-vps              swap + firewall with a 120s auto-revert timer
#   harden-vps --confirm    verify passed: cancel the revert timer and keep
#                           the ruleset applied (re-applying is idempotent)
#   harden-vps --status     show current state
set -eu

# The rig installs this as part of `prim install rig` (prim runs it with
# sudo) and by hand as `sudo ./rig/vps/harden-vps.sh`. Root is required:
# it owns /etc, the nftables table and the sshd drop-in.
[ "$(id -u)" = "0" ] || {
    echo "[harden-vps] needs root (sudo)" >&2
    exit 1
}

SWAP_FILE=/swapfile
SWAP_SIZE_MB=2048
NFT_TABLE=vps_hardening
NFT_RULES_FILE=/etc/nftables.d/${NFT_TABLE}.nft
NFT_EXT_DIR=/etc/nftables.d/prim-ext
REVERT_UNIT=vps-hardening-revert.timer
ALLOW_TCP_PORTS="22"   # the only port open to the Internet; the rest is tailscale0-only
MOSH_IFACE="tailscale0"
MOSH_UDP_RANGE="60000-60010"
SSHD_DROPIN_SRC="$(dirname "$0")/sshd-hardening.conf"
SSHD_DROPIN_DST="/etc/ssh/sshd_config.d/00-prim-hardening.conf"

say() { echo "[harden-vps] $*"; }

ruleset() {
    cat <<EOF
table inet ${NFT_TABLE} {
    chain input {
        type filter hook input priority filter; policy accept;
        iif "lo" accept
        ct state established,related accept
        meta l4proto icmp accept
        meta l4proto ipv6-icmp accept
        meta l4proto tcp th dport { ${ALLOW_TCP_PORTS} } accept
        iifname "${MOSH_IFACE}" udp dport ${MOSH_UDP_RANGE} accept
        drop
    }
}
EOF
}

ensure_swap() {
    if swapon --noheadings 2>/dev/null | grep -q "${SWAP_FILE}"; then
        say "swap: already active"
    else
        [ -e "${SWAP_FILE}" ] || {
            say "swap: creating ${SWAP_SIZE_MB}M ${SWAP_FILE}"
            fallocate -l "${SWAP_SIZE_MB}M" "${SWAP_FILE}"
            chmod 600 "${SWAP_FILE}"
            mkswap "${SWAP_FILE}" >/dev/null
        }
        swapon "${SWAP_FILE}"
        say "swap: activated"
    fi
    grep -q "^${SWAP_FILE} " /etc/fstab || \
        echo "${SWAP_FILE} none swap sw 0 0" >> /etc/fstab
    if ! grep -rqs '^vm.swappiness' /etc/sysctl.d/; then
        echo 'vm.swappiness=10' > /etc/sysctl.d/99-swap.conf
        sysctl -w vm.swappiness=10 >/dev/null
    fi
}

ensure_firewall() {
    command -v nft >/dev/null 2>&1 || {
        say "nft: installing nftables package"
        apt-get update -qq && apt-get install -y -qq nftables
    }
    mkdir -p /etc/nftables.d
    ruleset > "${NFT_RULES_FILE}"

    # Boot persistence: nftables.service loads /etc/nftables.conf; include
    # our table without flushing anything else (docker owns other tables).
    touch /etc/nftables.conf
    grep -q "${NFT_RULES_FILE}" /etc/nftables.conf || \
        echo "include \"${NFT_RULES_FILE}\"" >> /etc/nftables.conf
    systemctl enable nftables.service >/dev/null 2>&1 || true

    nft delete table inet "${NFT_TABLE}" 2>/dev/null || true
    nft -f "${NFT_RULES_FILE}"
    say "firewall: table ${NFT_TABLE} applied (lo/established/icmp + tcp ${ALLOW_TCP_PORTS} + mosh udp ${MOSH_UDP_RANGE} via ${MOSH_IFACE})"

    # Application-owned rules (one file per stack). The table was just
    # recreated, so the app rules have to be re-applied along with it.
    for frag in "${NFT_EXT_DIR}"/*.nft; do
        [ -e "$frag" ] || continue
        if nft -f "$frag"; then
            say "firewall: applied $(basename "$frag") (app rule)"
        else
            say "firewall: could not apply $(basename "$frag")"
        fi
    done
}

ensure_sshd() {
    [ -f "${SSHD_DROPIN_SRC}" ] || {
        say "sshd: drop-in source missing (${SSHD_DROPIN_SRC}), skipping"
        return 0
    }
    mkdir -p /etc/ssh/sshd_config.d
    install -m 644 "${SSHD_DROPIN_SRC}" "${SSHD_DROPIN_DST}"
    # Validate BEFORE reloading: a broken drop-in must not take sshd down.
    # reload (not restart) keeps the sessions that are already open alive.
    if ! sshd -t; then
        say "sshd: config invalid, removing the drop-in"
        rm -f "${SSHD_DROPIN_DST}"
        return 1
    fi
    systemctl reload ssh 2>/dev/null || systemctl reload sshd 2>/dev/null || true
    say "sshd: puerto 22 con clave publica o usuario+password; sin root, sin teclado-interactivo, solo 'debian'"
}

schedule_revert() {
    systemctl stop "${REVERT_UNIT}" 2>/dev/null || true
    systemd-run --unit="${REVERT_UNIT%.timer}" --on-active=120 \
        --description="harden-vps auto-revert (ssh must still work)" \
        sh -c "nft delete table inet ${NFT_TABLE} && echo '[harden-vps] REVERTED (timer expired, not confirmed)'" \
        >/dev/null
    say "firewall: auto-revert armed in 120s"
}

case "${1:-}" in
    --confirm)
        if systemctl stop "${REVERT_UNIT}" 2>/dev/null; then
            say "revert timer cancelled"
        fi
        ensure_swap
        ensure_firewall
        ensure_sshd
        say "confirmed: state is persistent across reboots"
        ;;
    --status)
        swapon --show 2>/dev/null || echo "swap: none"
        nft list table inet "${NFT_TABLE}" 2>/dev/null || echo "firewall: table absent"
        ;;
    *)
        ensure_swap
        ensure_firewall
        ensure_sshd
        schedule_revert
        say "VERIFY NOW (new ssh login + allowed ports), then run: $0 --confirm"
        ;;
esac
