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
#      dropped. Service-specific ports are owned by their stacks: e.g. gt's
#      WorkSpaces web client opens its own tcp 8443 allow inside this table
#      (its activate.sh + unit re-add it after every re-apply/reboot).
#      Docker's own tables are NOT flushed.
#
# Usage (root):
#   harden-vps              swap + firewall with a 120s auto-revert timer
#   harden-vps --confirm    verify passed: cancel the revert timer and keep
#                           the ruleset applied (re-applying is idempotent)
#   harden-vps --status     show current state
set -eu

SWAP_FILE=/swapfile
SWAP_SIZE_MB=2048
NFT_TABLE=vps_hardening
NFT_RULES_FILE=/etc/nftables.d/${NFT_TABLE}.nft
REVERT_UNIT=vps-hardening-revert.timer
ALLOW_TCP_PORTS="22"
MOSH_IFACE="tailscale0"
MOSH_UDP_RANGE="60000-60010"

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
        systemctl stop "${REVERT_UNIT}" 2>/dev/null && say "revert timer cancelled" || true
        ensure_swap
        ensure_firewall
        say "confirmed: state is persistent across reboots"
        ;;
    --status)
        swapon --show 2>/dev/null || echo "swap: none"
        nft list table inet "${NFT_TABLE}" 2>/dev/null || echo "firewall: table absent"
        ;;
    *)
        ensure_swap
        ensure_firewall
        schedule_revert
        say "VERIFY NOW (new ssh login + allowed ports), then run: $0 --confirm"
        ;;
esac
