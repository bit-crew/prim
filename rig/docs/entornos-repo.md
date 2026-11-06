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
