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
SMB** del componente `files` (ver `rig/docs/archivos-moviles.md`). Dejas
la foto en `~/work/`, pegas la ruta en moshi y el agente la lee.

## Instalar la limpieza a mano

La limpieza ya esta instalada en el VPS, pero se puede replicar en
cualquier maquina:

```
install -Dm755 rig/tools/prim-inbox-cleanup ~/.local/bin/prim-inbox-cleanup
install -Dm644 rig/systemd/prim-inbox-cleanup.service ~/.config/systemd/user/prim-inbox-cleanup.service
install -Dm644 rig/systemd/prim-inbox-cleanup.timer ~/.config/systemd/user/prim-inbox-cleanup.timer
systemctl --user daemon-reload
systemctl --user enable --now prim-inbox-cleanup.timer
```

Y para ver que borraria sin borrar nada:

```
prim-inbox-cleanup --dry-run
```

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
