# Toolchains por repo — que el VPS sea reproducible desde el codigo

El rig declara el perfil global S2. Los toolchains (rust, go, python, uv) y los
CLIs de infra (gcloud, terraform, sops) vivian instalados a mano en el host.
Este trabajo los mueve a flakes por repo, con la receta que ya estaba escrita
en `rig/common/entornos-repo.md` y `rig/nix/templates/repo/`.

## Specs

- S1: "Todo lo que necesita el vps esta en el codigo?, incluidos uv y rust?"
  Todo lo que el VPS necesita para trabajar debe estar declarado en el repo.
  Hoy, en el host: rust via rustup (`~/.cargo` + `~/.rustup`, 782M), go
  (`~/.local/go`, 283M), node (`~/.local/node`, 517M), uv (`~/.local/bin/uv`),
  python (`/usr/bin/python3`), uv tools (cxas-scrapi 603M, google-adk 42M,
  ruff 24M) y los CLIs de infra (aws, gcloud 584M, terraform, sops).
  Ninguno esta en el rig: `rig/nix/modules/tools.nix` los excluye a proposito
  ("per-repo flakes later, S3/S6e") y la receta existe pero solo esta aplicada
  en un repo de todo el arbol (`~/work/gt/setup/flake.nix`).

- S2: "Si implementa pero nonfijes version usa latest"
  Sin versiones pineadas: se referencian los paquetes por nombre sin sufijo de
  version (`rustc`, no `rustc_1_99`), `nixpkgs` se toma de la rama
  `nixos-unstable` y NO se commitea `flake.lock` por repo, para que cada uso
  resuelva el nixpkgs vigente.

- S3: el cambio aprobado incluye, en este orden: flakes para `prim` (rust),
  `gt` (go) y `knt/ap` (python + uv + terraform + google-cloud-sdk); `uv` y
  `ruff` en el perfil `vps` del rig; verificar que `prim` compila y que `go` y
  `uv` resuelven con el toolchain de nix; y solo entonces borrar del host
  `~/.rustup`, `~/.cargo`, `~/.local/go` y el tool uv `ruff`.

- S4: no se borra nada del host que el repo no declare. `cxas-scrapi` y
  `google-adk` no existen en nixpkgs: siguen siendo uv tools y los instala
  `make setup` de `knt/ap` (que necesita `uv` disponible, por eso `uv` va
  declarado en el flake de ese repo y en el perfil vps).

- S5: los `.envrc` de `prim` y `gt` ya existen y apuntan a los `.env.local`;
  el usuario dijo explicitamente que no se toquen ("el .envrc tampoco
  [se debe tocar], aunque es mas facil de recuperar, apuntan solo a los
  .env.local"), y el filtro de seguridad bloquea leerlos y escribirlos.
  Consecuencia: no hay activacion automatica por direnv de los entornos por
  repo, asi que el perfil `vps` del rig lleva tambien los toolchains que
  hospeda (cargo, rustc, go) para que `cargo build` y `go build` sigan
  funcionando en una shell normal. El `flake.nix` de cada repo sigue siendo
  la declaracion precisa de lo que ese repo necesita; la copia global
  existe solo hasta que el repo active su entorno con direnv.

## Tasks

- T1 (S2, S3) flake + .envrc de `~/work/prim`: rustc, cargo, rust-analyzer, clippy, rustfmt.
- T2 (S2, S3) flake + .envrc de `~/work/gt`: go.
- T3 (S2, S3, S4) flake + .envrc de `~/work/knt/ap`: python3, uv, terraform, google-cloud-sdk.
- T4 (S1, S3) `uv` + `ruff` en el perfil `vps` de `rig/nix/modules/tools.nix`.
- T5 (S3) verificar: `cargo build` en prim con el flake, `go version` en gt, `uv --version` en knt/ap, perfil vps evaluando.
- T6 (S3) `direnv allow` en los tres repos.
- T7 (S3, S4) borrar `~/.rustup`, `~/.cargo`, `~/.local/go` y el tool uv `ruff`.
- T8 (S1) documentar en `rig/common/entornos-repo.md` que los tres repos quedaron migrados.

## Log

- L1 (usuario): "Todo lo que necesita el vps esta en el codigo?, incluidos uv y rust?"
- L2 (usuario): "Si implementa pero nonfijes version usa latest"
- L3 (2026-11-20, medicion): en nixpkgs del rig existen `uv 0.13.5`, `ruff 0.17.3`,
  `cargo 1.99.0`, `rustc 1.99.0`, `python3 3.14.8`, `google-cloud-sdk 591.0.0`,
  `terraform 1.17.1`, `awscli2 2.36.31`; NO existen `cxas-scrapi` ni `google-adk`.
  `prim` no fija version de Rust (no hay `rust-toolchain.toml`, `edition = "2021"`),
  asi que adoptar el rustc de nixpkgs no rompe el proyecto.
