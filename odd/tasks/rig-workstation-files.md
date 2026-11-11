# Rig workstation installs the files share (prim)

Fold the `files` SMB share into the rig workstation flow: installing the rig
with the `vps` profile configures the `~/work` share automatically. Client-side
flows (Tailscale on devices, SMB mounts, Termux offline) are a later phase.

## Specs

- S1 (user, 2026-11-11, verbatim): "No e s solo por privilegio, es porque al
  se ruan estaciónde trabajo obviamnte s enecesita agregar todos los archivos
  qeu nos entren por ejemplo por telegram o correo manualemtne y eso es uan
  necesidad el rig entonces v aen el rig"
- S2 (user, 2026-11-11, verbatim): "la idea es qeu automaticamnte configure el
  VPS para compartir work, este vacia o llena no importa, si es un vps nuevo
  estara vacia de momento peor ais s ecomaprte"
- S3 (user, 2026-11-11, verbatim): "qeu cuando se configure un cliente
  pregutna el vps" — client setup asks the VPS (tailnet identity discovery).
- S4 (user, 2026-11-11, verbatim): "Luego haremos lo mismo con cliente, por
  ejemplo par el iphone sera moshi por loq eeu no hay qeu configurar mucho en
  la parte del vps"
- S5 (user, 2026-11-11, verbatim): "eso si debe configurar tailsacle en eambos
  pidiendo als variables qeu requiera"
- S6 (user, 2026-11-11, verbatim): "poner work de smb en el sistema de
  archivos de iphon o android"
- S7 (user, 2026-11-11, verbatim): "tambien en el caso de la table configurar
  trmux apr apoder trabajjar offlien cuando sea necesario"

## Tasks

| ID | Spec | Route | Commit |
|----|------|-------|--------|
| T1 | S1,S2 | Parent inline: `src/main.rs` — after a successful rig activation with profile `vps`, chain `files/install.sh` automatically (PRIM_DRY_RUN prints the chain, never runs it); menu/`COMPONENTS` copy updated (files is part of the workstation, explicit install = repair) | pending |
| T2 | S1,S2 | Parent inline: `src/lifecycle.rs` — `prim update` on the vps profile also forwards `update` to files (cmd_update is TTY-free) | pending |
| T3 | S1,S2 | Parent inline: `files/install.sh` header reframe (workstation piece, not optional); no behavior change | pending |
| T4 | S1,S2 | Parent inline: docs — `rig/docs/lifecycle.md`, `rig/docs/archivos-moviles.md` reframe; `rig/docs/vps-ovh.md` one line | pending |
| T5 | S1,S2 | Parent inline: verify — cargo build, sandbox-HOME dry run (`PRIM_DRY_RUN=1 prim install rig --profile vps` prints the chain), menu render | pending |
| T6 | S3-S7 | Later phase ("Luego haremos"): client flows — Tailscale on both ends with prompted variables, VPS discovery by clients, SMB mounts on iPhone/Android, Termux offline on the tablet | pending (user timing) |

Design calls (from S1/S2, recorded here): the chain applies ONLY to the
`vps` profile — client profiles never get an SMB server. `prim uninstall rig`
does NOT remove the share (`prim uninstall files` stays explicit, ~/work is
never touched). The files component remains directly addressable for repair.

## Log

- L1 (2026-11-11, original request, verbatim): "No e s solo por privilegio, es
  porque al se ruan estaciónde trabajo obviamnte s enecesita agregar todos los
  archivos qeu nos entren por ejemplo por telegram o correo manualemtne y eso
  es uan necesidad el rig entonces v aen el rig, la idea es qeu automaticamnte
  configure el VPS para compartir work, este vacia o llena no importa, si es
  un vps nuevo estara vacia de momento peor ais s ecomaprte, y qeu cuando se
  configure un cliente pregutna el vps. Luego haremos lo mismo con cliente,
  por ejemplo par el iphone sera moshi por loq eeu no hay qeu configurar mucho
  en la parte del vps eso si debe configurar tailsacle en eambos pidiendo als
  variables qeu requiera, poner work de smb en el sistema de archivos de iphon
  o android y tambien en el caso de la table configurar trmux apr apoder
  trabajjar offlien cuando sea necesario"
