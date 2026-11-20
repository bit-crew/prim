# Entornos por repo — flake.nix + direnv

Cada repo declara SUS dependencias (S3/S6e); las organizaciones declaran
identidad y secretos (ver [organizaciones.md](./organizaciones.md)). Nix
deduplica: dos repos que piden la misma versión de gcloud comparten la
misma copia en el store.

## Receta por repo

```
cd ~/work/<org>/repo
cp /ruta/al/repo/prim/rig/nix/templates/repo/flake.nix .
cp /ruta/al/repo/prim/rig/nix/templates/repo/envrc .envrc
# descomenta lo que este proyecto necesite en flake.nix
nix flake update        # genera flake.lock (versiones fijadas, S4)
direnv allow
```

Al entrar a la carpeta, `direnv + nix-direnv` activa el entorno; al salir,
se descarga. Composición: el `.envrc` del repo (`use flake`) y el de la
org (`dotenv_if_exists .env.local`) conviven — direnv evalúa del más
cercano hacia arriba.

## Ejemplos

- Repo de infra de un cliente: descomentá `google-cloud-sdk`, `terraform`,
  `sops` — nada de eso vive en el VPS global.
- Repo de datos con Spark: `openjdk`, `python3` (+ las libs que use el
  flake del repo).
- Repo Rust: `rustc cargo rust-analyzer` (o `rustup` fuera de Nix si
  preferís toolchains propias).

## Actualizar y limpiar

- Subir versiones = explícito: `nix flake update` y commit del
  `flake.lock` (S4).
- Los entornos activos tienen GC root (nix-direnv) — el recolector no los
  borra. Lo huérfano se limpia con el GC de Nix (ver
  [lifecycle.md](./lifecycle.md); `prim doctor`/prune llega con P5).

## Repos ya migrados

| Repo | `flake.nix` | Que aporta |
| --- | --- | --- |
| `prim` | `flake.nix` | `rustc cargo rust-analyzer clippy rustfmt` |
| `gt` | `flake.nix` | `go` + `gcc` (para `go test -race`) |
| `knt/ap` | `flake.nix` | `python3 uv terraform google-cloud-sdk` |

Ninguno fija versiones: `nixpkgs` se toma de `nixos-unstable` y **no se
commitea `flake.lock`**, asi que cada uso resuelve el arbol vigente. Para
revisar que el entorno quedo bien, sin tocar el repo:

```
cd <repo> && nix develop --command bash -c 'tool --version'
```

`cxas-scrapi` y `google-adk` no estan en nixpkgs: siguen siendo uv tools y
los instala `make setup` de `knt/ap`, que necesita `uv` disponible.

## Activacion sin direnv

Activar el entorno de un repo necesita `use flake` en su `.envrc`. Los
`.envrc` de `prim` y `gt` ya existen y apuntan a los `.env.local` de cada
organizacion: **no se tocan**. Mientras tanto, el perfil `vps` del rig lleva
los toolchains que hospeda (`cargo rustc go uv ruff`), para que `cargo build`
y `go build` sigan funcionando en una shell normal sin activar nada.

Cuando un repo tenga su `.envrc` con `use flake` y `direnv allow` ejecutado,
sus toolchains dejan de hacer falta en el perfil global y vuelven a ser
exclusivos del repo.
