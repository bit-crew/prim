# AGENTS.md — prim

Instrucciones operativas de este repositorio. Complementan las reglas
globales del harness (PI/AGENTS.md) para lo que es propio de `prim`.

## Este repo administra un VPS real

El componente `files` no es un simulacro: instala Samba, escribe
`/etc/samba/smb.conf`, configura el firewall (nftables), ajusta permisos
y propietarios de `~/work`, y reinicia servicios del sistema.

Por eso la diferencia entre el lado del repo y el lado del host es
importante:

## Regla del host: explicar antes de tocar, y esperar el ok

1. **Leer y diagnosticar el host siempre esta permitido.** Configs,
   logs, permisos, estado de servicios, `smbstatus`, `testparm`,
   `tailscale status`. Es la parte habitual del trabajo.

2. **Cualquier mutacion del host pide permiso explicito, ANTES de
   ejecutarla.** Es decir:

   - escribir, regenerar o parchear configs (`/etc/samba/smb.conf`,
     `/etc/nftables`, `/etc/systemd/system/...`);
   - reiniciar, recargar, habilitar o deshabilitar servicios;
   - instalar o remover paquetes (`apt-get`);
   - cambiar permisos, propietarios o ACLs (`chmod`, `chown`, `setfacl`,
     `chgrp -R`);
   - mover knobs en runtime (`smbcontrol ... debug`, `sysctl`, etc.);
   - borrar o renombrar archivos del host.

   Para cada una: **que** se va a hacer, **por que**, **como se revierte**,
   y despues esperar la respuesta. Si la accion esta autorizada de
   antemano (el usuario la pidio explicitamente), se ejecuta sin volver
   a preguntar, pero se informa que se hizo.

3. **Un "commit" no es una mutacion del host.** Cambios en el repo
   (codigo, tests, docs) son parte del trabajo pedido y no necesitan
   permiso extra. **Publicar** (push, PR, release) es decision del
   usuario salvo que la autorice explicitamente.

4. **Diagnostico con mutaciones minimas.** Si hace falta tocar algo solo
   para poder observar (subir el nivel de log, por ejemplo), se avisa
   antes y se revierte al terminar.

5. **No dejar el host a medio camino.** Si se interrumpe una accion a
   medias, se termina o se revierte, y se reporta el estado final.

## Reglas del repo que ya se aplican

- `~/work` nunca se borra ni se renombra: `files/install.sh` solo cambia
  permisos de directorios (nunca de archivos) y jamas borra contenido.
- El secreto SMB nunca viaja en argv, env, logs ni receipts: se guarda
  en `~/.local/state/prim/files/smb-password` con modo `600`.
- Commits en formato Conventional Commits, sin emojis.
- Checks antes de dar por terminado: `cargo fmt --all --check`,
  `cargo clippy --workspace -- -D warnings`, `cargo test`, `shellcheck`
  sobre los scripts que entries el CI.
