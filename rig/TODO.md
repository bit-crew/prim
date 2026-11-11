# rig TODO

Pendientes operativos del VPS (`rig/`). Los items que pertenecen a una
aplicación concreta viven en el repo de esa aplicación (p. ej. WorkSpaces va
en `gt`); acá queda lo que toca **al VPS como host**: entorno base, mosh,
samba, tailscale.

---

## 1. Mosh — regla UDP en el host pendiente 🟡

**Qué falta.** El diseño mosh (`rig/docs/mosh.md`) pide `iifname "tailscale0"
udp dport 60000-60010 accept` en la tabla `vps_hardening`. `harden-vps.sh`
(rama `feat/host-hardening`) aún no la tiene: al fusionar esa rama, sumar la
regla al ruleset del script (antes del `drop`) en vez de la regla a mano.

**Cómo se verifica.** `nft list table inet vps_hardening` muestra la regla y
un cliente mosh conecta y sobrevive un cambio Wi-Fi↔datos.
