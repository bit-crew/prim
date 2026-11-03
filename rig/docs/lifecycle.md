# Ciclo de vida — install / update / uninstall

prim gestiona el ciclo de vida de cada pieza; cada pieza es dueña de su
lógica (`install.sh` acepta `install|update|uninstall` como primer
argumento, default `install`). prim solo reenvía el verbo a las piezas
presentes en la máquina.

```
prim install [rig|harness|...]   # instala (selector interactivo sin args)
prim update                      # 1) prim se auto-actualiza  2) update de cada pieza instalada
prim uninstall <pieza>           # desinstala esa pieza (receipt-driven)
prim status                      # versión de prim + receipts instalados
```

También directo, sin prim: `sh rig/install.sh update` (o `uninstall`).

## Receipts (la fuente de verdad)

Cada instalador escribe `~/.local/share/prim/<pieza>.receipt.json` con lo
que hizo: perfil, gestor de paquetes, paquetes instalados, binarios con su
versión y repo, directorios, toolchains, cambio de shell.

- **Formato**: JSON válido pero *line-oriented* (un campo por línea). Lo
  escriben y leen solo los instaladores de prim — no lo reescribas con
  otra herramienta: un receipt ilegible hace que uninstall se niegue
  (fail-closed) en vez de adivinar.
- Máquina instalada antes de receipts: `prim update` una vez y el receipt
  se genera.

## Qué hace `update`

1. prim se auto-actualiza (release `latest`, sha256, salta si es idéntico).
2. Reinstala los paquetes de sistema (idempotente = upgrade).
3. Version-check de cada binario del receipt contra upstream (helix:
   versión pineada; node: dist index; resto: última release del repo).
   Solo re-descarga cuando difiere o falta.
4. `uv tool upgrade`, LSPs de npm refrescados, `rustup update`, dotfiles
   re-aplicados con chezmoi, receipt nuevo.

Notas: el **primer** update tras una instalación pre-receipts re-descarga
una vez los binarios con repo (versión registrada `null`); el toolchain de
**go** no se toca en update; las herramientas de infra VPS (aws, gcloud,
terraform, sops, wrangler) no están en el manifest y no se gestionan acá.

## Qué hace `uninstall` (y qué NO toca)

Remueve, según el receipt: paquetes apt/pkg/brew registrados, binarios,
directorios (`~/.local/node`), toolchains (rustup self uninstall, go, uv),
chezmoi (purge + estado + binario) y restaura el shell de login a bash si
rig lo había cambiado. El receipt se borra al final.

**Nunca toca datos tuyos**: `~/work`, las claves SSH de las
organizaciones (`~/.ssh`), los `.env.local`, los repos y los datos de
agentes quedan intactos. El binario `prim` tampoco se toca
(`rm ~/.local/bin/prim` si lo querés fuera).
