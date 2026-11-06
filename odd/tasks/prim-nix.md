# prim-nix — Nix as the single software manager

Head: Prim orchestrates; Nix manages software; home-manager manages
configuration; direnv activates per-repo environments. Rig is rebuilt around
Nix; the agent harness stays independent and self-updating, outside Nix.
Prerequisite merged: prim-lifecycle (8c75600).

## Specs

- S1 (user, proposal "Propuesta definitiva: Prim + Nix"): "Objetivo: mantener un VPS minimo en reposo, instalar herramientas segun las necesidades reales de sus proyectos y eliminar lo que deje de utilizarse."
- S2 (user, software table): global via Nix: git, curl, jq, ripgrep, fd, fzf, zsh, starship, zoxide, atuin, eza, helix, lazygit, delta, yazi, direnv, nix-direnv. Per-repo (flake.nix): toolchains (rust/cargo, node, python, go), infra cloud (aws, gcloud, gsutil), terraform, wrangler, sops, java, clang. "APT queda reservado para el sistema operativo y sus requisitos fundamentales. No habra mise, rustup ni Devbox."
- S3 (user, environments): "Cada organizacion conserva .envrc y .env.local. Cada proyecto declara dependencias en flake.nix. flake.lock fija las versiones. direnv + nix-direnv activa automaticamente el entorno. Las credenciales nunca se almacenan en Nix."
- S4 (user, versions): global = recent; projects = pinned; updates explicit via `nix flake update`.
- S5 (user, prim changes 1-9): rig delegates installs to Nix; prim keeps org/project management; redundant installers removed; nix GC handles removal (no manual registry); prim distributed via GitHub Releases; binary integrity via Nix hashes (manual sha256 only for bootstrap).
- S6 (user decisions 2026-11-04): (a) "el binario de prim instala nix y actualiza el os como primer paso de ahi en adelante nix instala" — prim owns the bootstrap; (b) harness independent, rig first; (c) Nix single-user, no daemon; (d) home-manager REPLACES chezmoi (accepted with its trade-offs: edit-the-flake model, single point of failure, prompts become a static identity file read by prim once); (e) deps per REPO, identity/secrets per ORG; same version = one store copy (store-level dedup; declarative boilerplate acceptable, shared template later if it bothers).
- S7 (parent decisions, reversible): official Nix installer (nixos.org/nix/install --no-daemon), no Determinate vendor; agent binaries (pi, engram, gentle-ai, moshi) stay OUT of Nix (self-updating); nix-direnv GC roots protect active environments.
- S8 (user, 2026-11-05, binding invariant): "la idea es que prim pueda instalar todo, no que me pida instales 10 cosas antes de usar prim" — prim is self-sufficient from a bare machine: one command (curl | sh) brings prim, and prim itself updates the OS, installs Nix, activates the flake and configures. No manual prerequisites beyond SSH access; Nix is an internal dependency of prim, never a user prerequisite.

## Phases (tasks)

| ID | Specs | Route | Commit |
|----|-------|-------|--------|
| P1: prim bootstrap — apt update/upgrade + install Nix single-user + verify; identity prompt writes ~/.config/prim/identity.toml | S6a,S7 | TBD | |
| P2: global flake — rig/nix/flake.nix + home-manager modules porting every rig/home template (zshrc+PRIM_LANDING+herdr, gitconfig, helix, lazygit+delta, atuin, herdr); profiles dc1/vps as flake args | S2,S3,S6d | TBD | |
| P3: rig rebuilt — verbs install/update/uninstall over Nix (nix profile / home-manager switch / uninstall); receipts shrink; apt minimal list; retire old binary installers | S5,S2 | TBD | |
| P4: repos per-org — .envrc `use flake` scaffolding guidance; org identity untouched (prim org) | S3,S6e | inline, templates + guide |
| P5: retirement + GC — remove binaries.rs installers, chezmoi, old paths on machines; `prim doctor`/prune wrappers over nix-collect-garbage | S1,S5 | TBD | |
| P6: migrate THIS VPS — install Nix (needs explicit user OK), switch to flake, migrate gt/knt/prim orgs, retire old ~/.local stack, GC | S1,S6 | TBD | |

## Log

- L1 (2026-11-04, user): full proposal document "Propuesta definitiva: Prim + Nix" (sections 1-6 as quoted in S1-S5).
- L2 (2026-11-04, user answers): bootstrap owned by prim binary; harness independent (rig first); single-user Nix; deps per repo with org-level secrets/.envrc; confirmed store dedup understanding.
- L3 (2026-11-04, user): "1. Si / 2. Si" — home-manager replaces chezmoi; feat/lifecycle merged BEFORE the pivot (merged as 8c75600, CI 3/3 green, binaries published).
- L4 (2026-11-04, parent): P6 (machine migration) requires explicit user consent per machine-mutation rule; open question asked.
- L5 (2026-11-05, user): consent to install Nix on this VPS now (development-time), with the S8 reminder; parent installs it via the exact command P1 will codify, so dev-time validation equals product validation.
