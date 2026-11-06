# prim-files — Samba over Tailscale: native mobile access to ~/work

Head: optional prim component `files` — SMB share of ~/work for iPhone
Files / Android / Windows / Linux, reachable ONLY inside the tailnet.
System-layer component managed with prim's existing verb machinery.

## Specs

- S9 (user, 2026-11-07, proposal): "Añade Samba al diseño del VPS para compartir /home/debian/work mediante SMB con iPhone, Android, Windows y Linux." Bullets verbatim: "- iPhone: integración nativa con Archivos. - Android: gestor compatible con SMB. - Acceso remoto seguro mediante Tailscale. - Usuario dedicado con permisos de lectura y escritura. - Sin exponer SMB a Internet. - Integrar instalación, configuración y diagnóstico en Rig/Prim. - Mantener bajo consumo de RAM, CPU y disco. No instalar Nextcloud ni servicios de sincronización."
- S10 (user, 2026-11-07, final recommendation): "Mantendría Samba + Tailscale, con una configuración mínima: - Solo smbd, sin nmbd. - Compartir exclusivamente ~/work. - Escuchar únicamente en tailscale0. - Sin Nextcloud ni sincronización. - Servicio bajo demanda si conseguimos una activación fiable. Samba no es especialmente pesado para un VPS. Su consumo en reposo no justifica sacrificar la integración nativa que buscas. No cambiaría de tecnología por ahorrar unas decenas de MB de RAM."
- S11 (design, agreed): (a) iPhone Files connects to smb:// natively — WebDAV rejected because iOS needs a third-party app for it; (b) new OPTIONAL component `files` (not in default installs), driven by prim's existing install/update/uninstall machinery (files/install.sh verb contract); (c) dedicated system user (NOT debian) for the share, RW via group + setgid on ~/work; (d) bind interfaces only = tailscale0; no ports exposed; (e) receipt ~/.local/share/prim/files.receipt.json (line-oriented, prim status/uninstall compatible); (f) S4 applies: uninstall NEVER touches ~/work; tailscale is kept on uninstall (network base, other uses) with a note; (g) on-demand service: socket activation of smbd is unreliable — v1 ships always-on smbd (idle ~30MB, accepted by user) with nmbd disabled; documented for future reopening; (h) `prim doctor` gains a files section when the receipt exists.

## Tasks

| ID | Specs | Route | Commit |
|----|-------|-------|--------|
| F1: files/install.sh (install/update/uninstall verbs: apt samba+tailscale, smb.conf, user+group+smbpasswd, tailscale up with auth URL, receipt) + smoke tests with stubbed sudo | S9,S10,S11c-f | writer | |
| F2: prim integration — COMPONENTS entry, selector copy, doctor files section | S11b,h | writer | |
| F3: docs (rig/docs/archivos-moviles.md: phone connection steps, password handling, doctor, uninstall) | S9,S10 | writer | |
| F4: parent review + sandbox verification + CI green + machine activation WITH user | all | parent | |

## Log

- L1 (2026-11-07, user): S9 proposal (asked whether it fits prim or another stack is better).
- L2 (2026-11-07, parent): evaluation — requirement fits prim; flagged Samba's root surface; proposed WebDAV alternative.
- L3 (2026-11-07, user): correction — iOS Files is SMB-native, not WebDAV; SMB beats WebDAV for uploads/sync. Parent concedes both points.
- L4 (2026-11-07, user): S10 final recommendation — Samba + Tailscale stays, minimal config, don't trade native integration for RAM.
