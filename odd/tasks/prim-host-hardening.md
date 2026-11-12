# prim-host-hardening — baseline de seguridad general del VPS

Head: prim es quien configura el VPS de forma general; este feature
consolida el hardening de host (swap de emergencia + firewall nftables) como
piezas idempotentes y reproducibles desde el repo. El live del 2026-11-10
coincide con el estado objetivo del script.

## Specs

- S1 (user, 2026-11-10): "aPlica el swap" — swap de 2 GB inmediato.
- S2 (user, 2026-11-10, decision "Ambos (recomendado)"): firewall que solo
  abre 22 y 8443, aplicado con auto-revert para no perder SSH.
- S3 (user, 2026-11-10, correccion de scoping, verbatim): "lo qeu es swap y
  esas cosas deben de ir en prim directamtnre qeu es quin configura el vps
  de forma genral" — el hardening de host es propiedad de prim, no de los
  proyectos de organizacion (gt/knt).

## Tasks

| ID | Spec | Route | Commit |
|----|------|-------|--------|
| H1 | S1,S2,S3 | Parent inline: `rig/host/harden-vps.sh` idempotente (swap 2G + tabla nftables `vps_hardening` 22/8443 con auto-revert 120s) | c693d12 |
| H2 | S1,S2 | Parent inline: aplicar en vivo, verificar SSH activo, confirmar y dejar persistente | aplicado 2026-11-10 (sin commit: estado host) |

## Log

- L1 (2026-11-10): swap creado y activado en vivo antes de la correccion de
  scoping (fallocate 2G /swapfile, fstab, vm.swappiness=10 en
  /etc/sysctl.d/99-workspaces-swap.conf). El script lo detecta como estado
  objetivo y no lo recrea (nota: sysctl file distinto, mismo efecto).
- L2 (2026-11-10): correccion de scoping S3; este feature queda en prim.
- L3 (2026-11-10, aplicacion H2): nftables 1.1.3 via apt (unico paquete
  nuevo); tabla `vps_hardening` aplicada, egress y sesion SSH verificados,
  auto-revert cancelado con --confirm, persistente via /etc/nftables.conf
  include + nftables.service enabled. sysctl renombrado a 99-swap.conf
  (nombre canonico del script). Observacion del apt: kernel 6.12.116
  pendiente de reinicio (preexistente, decision del usuario, no parte de
  este feature).
- L4 (2026-11-12, puerto de gt fuera de prim, verbatim): "biene entoces
  quita solo es aparte y agregala al repo de gt y aqeu este es quien
  necesita activarlo apr apoder usarlo". El swap queda en prim (sirve a
  todo el host); el allow TCP 8443 sale del ruleset de harden-vps.sh: es el
  puerto del web client de gt y ahora lo abre su propia pila (activate.sh +
  ExecStartPre de su unidad, idempotentes, dentro de esta misma tabla
  vps_hardening porque el default-deny de prim dejaria caer una tabla
  separada).
