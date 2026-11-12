# rig TODO

Pendientes operativos del VPS (`rig/`). Los items que pertenecen a una
aplicación concreta viven en el repo de esa aplicación (p. ej. WorkSpaces va
en `gt`); acá queda lo que toca **al VPS como host**: entorno base, mosh,
samba, tailscale.

---

## 1. Mosh — activación en el host pendiente 🟡

**Qué falta.** La regla UDP 60000–60010 por `tailscale0` ya vive en
`rig/vps/harden-vps.sh` (main). Falta en el host: re-aplicar el script
(`sudo ./rig/vps/harden-vps.sh --confirm`, tras actualizar el repo) y
activar el paquete con `prim update rig`. Ojo: re-aplicar también cierra el
8443 (gt) hasta que su pila lo re-abra — lo hace sola al arrancar su
servicio.

**Cómo se verifica.** `nft list table inet vps_hardening` muestra la regla y
un cliente mosh conecta y sobrevive un cambio Wi-Fi↔datos.
