# Rig reorg: vps / client / common (prim)

Flatten the accumulated .sh sprawl into the user's three-bucket layout, drop
the files with no consumer (chezmoi leftovers, pre-Nix hand-install wrappers),
and give each bucket its own documentation. Behavior does not change.

## Specs

- S1 (user, 2026-11-12, verbatim): "reorganicemso mejor al carpeta rig,
  pongamos tres internas: vps, client, common, así e más entendible y dentro
  de cad auan su propia docuemtnación"
- S2 (user, 2026-11-12, verbatim): "creo qeu te di un patron antisolid pero
  veo un montos de archivos que no se si sean tan necesarios" — no files
  without a consumer.
- S3 (user, 2026-11-12, verbatim): "si hazlo" — map and deletions approved.

## Tasks

| ID | Spec | Route | Commit |
|----|------|-------|--------|
| R1 | S1 | Parent inline: git mv into rig/vps (harden-vps.sh, prim-inbox-cleanup + service + timer, docs vps-ovh/mosh/archivos-moviles), rig/client (img-grab, img-grab.ps1, imagenes.md), rig/common (lifecycle, entornos-repo, organizaciones, multi-cuenta, transferir-archivos) | pending |
| R2 | S2 | Parent inline: delete rig/home/ (chezmoi retired, ported to nix modules in P2) + install-img-grab.sh + install-inbox-cleanup.sh (pre-Nix hand-install plane) | pending |
| R3 | S1 | Parent inline: re-cable — systemd.nix and termux.nix imports, doc cross-references, TODO.md and harden-vps.sh path mentions | pending |
| R4 | S1,S2 | Parent inline: verify — nix eval both profiles (home.file/systemd targets resolve), grep for stale rig/{docs,tools,systemd,host,home} references outside odd history | pending |
| R5 | S3 | Parent inline: commit + push to main | pending |

Living docs get new paths; odd/* historical log entries keep their original
wording (history). nix/ stays (it IS the environment); rig/install.sh stays
(the prim binary references it); files/ is repo-root, out of scope.

## Log

- L1 (2026-11-12, original request, verbatim): "Ya estab viendo pero vos
  fuiste el qeu creo es emonton de archivos .sh, par aver uan sola cosa me
  toca navedgar un monton de carpetas, la verdad es qeu no me gust amucho qeu
  digamos, adicioanl reorganicemso mejor al carpeta rig, pongamos tres
  internas: vps, client, common, así e más entendible y dentro de cad auan su
  propia docuemtnación, adempas creo qeu te di un patron antisolid pero veo
  un montos de archivos que no se si sean tan necesarios, porque los creas?"
- L2 (2026-11-12): why the sprawl existed — the clipboard feature's S8
  constraint shipped raw scripts + hand-install wrappers BEFORE Nix activation
  existed; with the flake live that plane is dead code. rig/home/ was the
  retired chezmoi source, already ported into the nix modules (P2, S6d).
- L3 (2026-11-12, approval): "si hazlo" — after confirming chezmoi's
  replacement (home-manager writes the dotfiles from the nix modules).
