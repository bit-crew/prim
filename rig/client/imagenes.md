# Imagenes — pegar capturas al agente sin que se queden dando vueltas

El portapapeles vive en el **cliente** y el agente vive en el **VPS**. Por
eso una imagen no se pega: **viaja como archivo** y su **ruta viaja como
texto**. Pegar texto por SSH funciona nativo (el terminal manda los
caracteres); OSC 52 solo hace falta en el sentido inverso, que es que el
VPS escriba en tu portapapeles local.

El VPS recibe las imagenes en un inbox **efimero**:

```
~/.cache/prim/inbox/
```

XDG cache = disposable por definicion. Esta **fuera de `~/work`**: una
captura nunca entra a un repo, ni a un indice de git, ni al share de SMB.
La limpieza corre **dos veces por dia** (03:00 y 15:00) y borra **solo lo
que tiene mas de 12 horas**: el peor caso es ~24h.

## Instalar

En el **cliente** (la maquina desde la que copias las imagenes):

```
curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/rig/client/install-img-grab.sh | sh
```

Instala `img-grab` en `~/.local/bin` y **cablea el binding solo**, segun el
terminal que detecte (Ghostty y kitty se editan en el sitio, con backup del
config; para WezTerm e iTerm2 imprime el snippet porque son archivos Lua y
no se tocan). Es idempotente: correrlo dos veces no duplica nada.

En el **VPS** (donde corre el agente), la limpieza del inbox **viene con el
rig**: el perfil `vps` del flake instala el script, las dos unidades de
systemd de usuario y habilita el timer (`prim install rig`). No hace falta
instalar nada a mano.

Con el repo en una branch (antes del merge) hay que pasarle `RAW_BASE`:

```
RAW_BASE=https://raw.githubusercontent.com/bit-crew/prim/feat/clipboard-inbox \
  sh install-img-grab.sh
```

## Que necesitas

- Un alias SSH en tu `~/.ssh/config` que apunte al VPS (por defecto el
  script usa `vps`).
- `ssh` y `scp`.
- En el DC-1: Termux y el add-on **Termux:API** (da `termux-clipboard-get`).
- En Linux: `wl-paste` (Wayland) o `xclip` (X11).
- En Windows: nada extra (el portapapeles es del sistema).

## El comando

```
img-grab
```

Que hace, en orden:

1. Toma la imagen del portapapeles.
2. La sube al inbox del VPS como `<fecha>-<nombre>.png`.
3. Mueve el puntero `last` a esa imagen.
4. **Imprime la ruta remota** (esa es la que lee el agente).
5. Copia esa ruta al portapapeles local **como texto**, asi que el proximo
   `Cmd+V` pega la ruta en la sesion.

Despues solo tenes que decir *"leé la ultima imagen"* (o pegar la ruta si
prefieres ser explicito).

Flags:

| Flag | Que hace |
|------|----------|
| `--host NAME` | alias SSH del VPS (por defecto `vps`) |
| `--inbox PATH` | inbox remoto (por defecto `~/.cache/prim/inbox` del VPS) |
| `--file PATH` | agarra ese archivo en vez del portapapeles |
| `--dry-run` | muestra el plan y no sube nada |

El `~` del inbox lo expande el **shell remoto**, no el tuyo: el inbox vive
en el VPS y tu home es otro.

## Leer la ultima imagen

```
~/.cache/prim/inbox/last
```

es un symlink a la imagen mas reciente. El agente lo lee y abre el archivo.
Tambien podes pegarle la ruta directa, que es lo que queda en tu
portapapeles.

## Bindings

Un binding, una accion: la imagen sube y la ruta queda lista para pegar.

**Termux (DC-1)** — en `~/.termux/termux.properties`:

```
extra-keys = CTRL+ALT+G,~/.local/bin/img-grab\n
```

**Ghostty (Linux)** — en `~/.config/ghostty/config`:

```
keybind = CTRL+SHIFT+G=exec:/home/TU_USUARIO/.local/bin/img-grab
```

**WezTerm (Windows)** — en `wezterm.lua`:

```lua
config.keys = {
  { key = 'G', mods = 'CTRL|SHIFT', action = WezTermSpawnCommand.new('~/.local/bin/img-grab') },
}
```

El perfil `dc1` del rig escribe el `termux.properties` por vos (modulo
`rig/nix/modules/termux.nix`).

## iPhone

El iPhone no corre rclone ni Termux: ahi la imagen entra por el **share de
SMB** del componente `files` (ver `rig/vps/archivos-moviles.md`). Dejas
la foto en `~/work/`, pegas la ruta en moshi y el agente la lee.

## Ver que borraria la limpieza

```
prim-inbox-cleanup --dry-run
```

Sin `--dry-run` borra de verdad, y solo lo que tenga mas de 12 horas.

## Ojo

- Las imagenes quedan en el disco del VPS hasta ~24h (12h de umbral, dos
  pasadas por dia).
- `img-grab` necesita el alias SSH: si tu usuario remoto no se llama como
  el tuyo, el `~` remoto apunta a otro home y las imagenes se guardan ahi.
  Pasa `--inbox` con la ruta absoluta si hace falta.
- El script **no** mergea nada: solo sube el archivo. Si la imagen ya
  existia en el VPS con otro nombre, quedan las dos.
- El `.ps1` de Windows no lo cubre el CI (no hay runner de Windows) y no se
  ha ejercitado en una maquina real.
