# Imagenes — pegar capturas al agente sin que se queden dando vueltas

El portapapeles vive en el **cliente** y el agente vive en el **VPS**. Por
eso una imagen no se pega: **viaja como archivo** y su **ruta viaja como
texto**. Pegar texto por SSH funciona nativo (el terminal manda los
caracteres); OSC 52 solo hace falta en el sentido inverso, que es que el
VPS escriba en tu portapapeles local.

Regla de la casa: **Ctrl+V pega la imagen**. Lo que llega a la sesion es la
ruta del archivo, porque un terminal no transmite imagenes: transmite
texto.

El VPS recibe las imagenes en un inbox **efimero**:

```
~/.cache/prim/inbox/
```

XDG cache = disposable por definicion. Esta **fuera de `~/work`**: una
captura nunca entra a un repo, ni a un indice de git, ni al share de SMB.
La limpieza corre **dos veces por dia** (03:00 y 15:00) y borra **solo lo
que tiene mas de 12 horas**: el peor caso es ~24h.

## Ctrl+V en todos los clientes

| Cliente | Ctrl+V | Por que |
|---------|--------|---------|
| Linux/Windows con `herdr --remote <host> --remote-keybindings server` | **nativo** | herdr corre en tu maquina, lee el portapapeles, copia la imagen a un archivo temporal del VPS y pega la ruta en el pane |
| Linux/Windows por `ssh <host>` (el `.zshrc` del VPS hace `exec herdr`) | no puede | herdr corre entero en el servidor y no ve el portapapeles de tu escritorio |
| DC-1 (Termux) | **binding** | `extra-keys` de Termux: CTRL+V corre `img-grab` (sube la imagen y deja la ruta en el portapapeles) |
| iPhone (moshi) | no puede | no hay portapapeles de escritorio: la imagen entra por el share y se pega la ruta |

**El modo de conexion decide.** Por eso Ctrl+V "no hace nada" cuando se
entra por SSH: no es un bug del terminal, es que el herdr del servidor no
tiene acceso al portapapeles de la notebook. Para que el puente funcione
en el escritorio hay que entrar con el cliente:

```
herdr --remote vps --remote-keybindings server
```

Ahí el herdr local dibuja la UI y puede leer el portapapeles: copia la
imagen a un temp del VPS y pega la ruta. Con Ctrl+V tomado por la imagen,
el texto se pega con **CTRL+SHIFT+V**.

El `--remote-keybindings server` no es opcional: por default, `herdr
--remote` usa los keybindings **de tu maquina** y no manda los
`[[keys.command]]` (los popups de lazygit, `hx .` y lazysql viven en el
config del VPS). Con el flag, el attach usa el config del servidor y los
popups vuelven. Ver `rig/common/lifecycle.md`.

En los terminales donde Ctrl+V ya es el pegado de texto (Ghostty, kitty,
WezTerm, iTerm2) la imagen se pega con **CTRL+SHIFT+G** y el texto
sigue con Ctrl+V. Donde Ctrl+V esta libre (dentro de herdr, o en Termux
donde el pegado de texto es Ctrl+SHIFT+V) la imagen se pega con Ctrl+V.

El share SMB es para otra cosa: descargar archivos completos y directorios,
no capturas sueltas.

## Instalar

En el **cliente** (la maquina desde la que copias las imagenes):

```
curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/rig/client/install-img-grab.sh | sh
```

Instala `img-grab` en `~/.local/bin` y **cablea el binding solo**, segun el
terminal que detecte (Ghostty y kitty se editan en el sitio, con backup del
config; para WezTerm e iTerm2 imprime el snippet porque son archivos Lua y
no se tocan). Es idempotente: correrlo dos veces no duplica nada.

Si te conectas con `herdr --remote`, no instalas nada: herdr ya pega la
imagen con Ctrl+V. El `img-grab` es para los clientes sin herdr local
(DC-1) o para cuando queres una tecla propia.

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
extra-keys = CTRL+V,~/.local/bin/img-grab\n
```

En Termux el pegado de texto es CTRL+SHIFT+V, asi que CTRL+V queda libre
para la imagen. La imagen sube y la **ruta** queda en el portapapeles del
telefono: se pega con CTRL+SHIFT+V.

**Ghostty (Linux)** — en `~/.config/ghostty/config`:

```
keybind = CTRL+SHIFT+G=exec:/home/TU_USUARIO/.local/bin/img-grab
```

Aqui Ctrl+V queda como pegado de texto: en un terminal de escritorio la
imagen va por CTRL+SHIFT+G, y Ctrl+V pega imagenes solo dentro de herdr
(`herdr --remote`), donde el herdr local intercepta la tecla.

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
