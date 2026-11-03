# prim-lifecycle — install/update/uninstall per component

Head: full lifecycle for prim pieces. prim coordinates verbs; each component
owns its logic and script. Scope this round: prim + rig (harness next round).

## Specs

- S1: "este debería de permitir no solo instalar sino tambien actualizar e incluso desinstalar" — prim gains `update`, `uninstall` and `status` besides install.
- S2: "me gustaría que cada pieza tenga su propio script es decir si solo uso el rig solo se instale" — every component install.sh accepts `install|update|uninstall` (default install); prim forwards the verb; actions touch only the components present on the machine.
- S3: User decisions (2026-11-03): (a) uninstall also removes the apt packages the receipt recorded; (b) binary updates use version-check (receipt stores version; re-download only when the upstream latest differs); (c) scope prim + rig first.
- S4: Uninstall never touches user data: `~/work`, org SSH keys, `.env.local` files, repos, agent memory stay intact. Only prim-owned tooling goes.
- S5: The permanent `prim` binary is owned by prim itself: root install.sh copies it to `~/.local/bin/prim`; `prim update` self-updates from the repo's `latest` release before updating components; rig no longer installs prim.

## Tasks

| ID | Specs | Route | Commit |
|----|-------|-------|--------|
| T1 | S5 | install.sh: install prim permanently; prim: self-update | |
| T2 | S1,S2 | prim: `install`/`update`/`uninstall`/`status` subcommands forwarding verbs | |
| T3 | S2,S3 | rig/install.sh verb contract; rig-install receipts on install | |
| T4 | S1,S3(b) | rig-install update: chezmoi re-apply, apt upgrade, version-check binaries | |
| T5 | S1,S3(a),S4 | rig-install uninstall: receipt-driven, apt remove, dotfiles purge, shell restore | |
| T6 | S1-S5 | docs (rig/docs/lifecycle.md) + sandbox verification suite | |

### Routes (delegation evidence)

- Commit A (fixes): inline — bounded single check (b3efa6b).
- T1+T2 → gentle-ai-worker background `mw063gu6-1-xgiz` (Writer rule: parallelism + Context backstop at 150k tokens; surfaces: src/main.rs, src/lifecycle.rs, install.sh).
- T3 → gentle-ai-worker background `mw063gue-2-jt44` (same triggers; surfaces: rig/install.sh, rig/installer/src/{main,binaries,receipt}.rs).
- T4/T5: sequential after T3 (depend on receipt.rs API); route decided when T3 lands.
- T6: parent inline (docs + end-to-end sandbox suite + work-unit commits).

## Log

- L1 (2026-11-03, user): "para que sea realmente utilizable, este debería de permitir no solo instalar sino tambien actualizar e incluso desinstalar, no te parece, comomhacemos eso, hay uan forma mejor de abordarlo, eso si me gustaría que cada pieza tenga su propio script es decir si solo uso el rig solo se instale"
- L2 (2026-11-03, user decisions): quitar apt siempre en uninstall; version-check para binarios; alcance prim+rig primero.
- L3 (2026-11-03, design agreed): verbs per component script; receipts at ~/.local/share/prim/<piece>.receipt.json; prim self-owns its binary; uninstall protects user data (S4).
- L4 (2026-11-03, user, install-set audit): go stays (user decision), openjdk stays (Spark), aws/gcloud/terraform/sops/wrangler stay in rig until core ships. Applied: codelldb installer restored (was orphaned), nerd font now dc1-only.
