# Ciclo de vida — rig en la era Nix

Desde P5 rig no tiene instalador propio: prim lo activa sobre Nix
(home-manager contra el flake global `rig/nix`), escribe su receipt y lo
desinstala con `home-manager uninstall`. El binario rig-install y su
workflow de build están retirados; `rig/install.sh` quedó solo como stub de
compatibilidad que redirige al instalador raíz de prim (los viejos links
siguen funcionando: `install|update|uninstall` se mapean a los verbos de
prim).

```
prim install rig [--profile dc1|vps]  # bootstrap (os+nix+identidad) + home-manager switch + receipt
prim update                           # 1) prim se auto-actualiza  2) re-activa rig (ref flotante a main)
prim uninstall rig                    # home-manager uninstall + receipt fuera
prim status                           # versión de prim + receipts instalados
prim doctor [--prune]                 # diagnóstico de solo lectura; --prune = GC >30d
```

## Qué hace cada verbo

### `prim install rig`

1. `prim bootstrap` (S8): apt update/upgrade, Nix single-user (instalador
   oficial `--no-daemon`) e identidad estática en
   `~/.config/prim/identity.toml`. Todo idempotente: cada paso ya presente
   se omite.
2. Activación: `nix run home-manager -- switch --flake
   github:bit-crew/prim?dir=rig/nix#<perfil> -b prim-backup --impure`
   (con `PRIM_GIT_NAME`/`PRIM_GIT_EMAIL` desde la identidad). Los
   dotfiles preexistentes en conflicto se mueven a `*.prim-backup`, no se
   sobreescriben.
3. prim escribe el receipt.
4. Workstation (S1/S2 de `odd/tasks/rig-workstation-files.md`): con el
   perfil `vps`, prim encadena `files/install.sh` — el share SMB de
   `~/work`, solo por `tailscale0` (clave SMB por TTY; `tailscale up` si
   falta sesion). Los perfiles cliente nunca levantan el share. Con
   `PRIM_DRY_RUN=1` el encadenado se imprime y no se ejecuta.

### `prim update`

prim se auto-actualiza (release `latest`, sha256, salta si es idéntico) y
re-activa rig: la ref `github:...` se re-evalúa contra main en cada
ejecución, así que "update" es re-activar. El pin fino de versiones vive
en el `flake.lock` del repo (S4): se mueve con `nix flake update` en el
repo, no en la máquina.

El componente `files` (share de `~/work`) también se actualiza aquí cuando
su receipt existe: prim le reenvía `update` (apt upgrade de samba,
tailscale update, permisos y conf — sin prompts, TTY-free).

### `prim uninstall rig`

`home-manager uninstall` y el receipt se borra. El componente `files` NO
se desinstala junto al rig: `prim uninstall files` es explícito (y jamás
toca `~/work`, S4 del script). **No toca**: `/nix` (el
store lo limpia el GC de nix, S5), `~/work`, `~/.ssh`, los `.env.local`,
ni el stack del agente (harness). El binario `prim` tampoco se toca
(`rm ~/.local/bin/prim` si lo querés fuera).

## Receipts (la fuente de verdad)

`~/.local/share/prim/rig.receipt.json` — desde P3 lo escribe prim
(nadie más): perfil, fecha, `pkgmgr: nix`, `dotfiles: home-manager`.
Line-oriented, un campo por línea; un receipt ilegible hace que uninstall
se niegue (fail-closed) en vez de adivinar. Sin receipt: `prim update`
una vez y se regenera.

## doctor / prune

- `prim doctor`: diagnóstico de solo lectura — nix (versión y
  experimental features), receipt del rig, reachability del flake global
  (tolerante a timeouts de red), tamaño de `/nix/store`, GC roots de
  direnv activos (`~/.local/share/direnv`) y restos de la era pre-Nix
  (informativos; prim nunca los borra).
- `prim doctor --prune`: `nix-collect-garbage --delete-older-than 30d`
  (S1: el GC de nix es el dueño de la limpieza). Imprime el comando antes
  de ejecutarlo; con `PRIM_DRY_RUN=1` lo muestra y no ejecuta nada.

## Migración desde la era rig-install

Las máquinas instaladas con rig-install tienen restos que prim detecta
con `prim doctor` como líneas `legado (era pre-Nix):` y **nunca** remueve
(la limpieza final por máquina es P6, con consentimiento explícito):

- binario `chezmoi` en PATH y `~/.local/share/chezmoi` — home-manager
  gestiona los dotfiles ahora (S6d);
- binarios en `~/.local/bin` (hx, lazygit, delta, yazi, ya, eza, zoxide,
  starship, atuin, bat, marksman, herdr) — el perfil nix los provee
  (S2); revisa el orden de PATH para no ensombrecer al flake;
- `~/google-cloud-sdk` — gcloud quedó como dependencia por repo (S3).

Sobre esa base, `prim install rig` ya no reinstala nada de lo viejo:
activa el flake y listo; el receipt pasa a ser de prim.
