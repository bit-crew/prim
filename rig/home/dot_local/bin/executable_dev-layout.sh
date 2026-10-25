#!/bin/sh
# dev-layout — abre un workspace de trabajo en Herdr con la distribucion:
#   izquierda: editor Helix | derecha: lazygit (arriba) + terminal (abajo).
#
# Uso: `dev-layout [ruta]`  (por defecto, el directorio actual).
#
# Nota honesta: la CLI de Herdr (`pane split --direction right|down`) crea los
# paneles pero NO fija el ratio 70/30 desde script (no existe --size/--ratio en
# el core). La estructura queda correcta; ajusta el ancho a gusto una vez con
# los atajos de Herdr. Si algun dia quieres el 70/30 exacto automatico, se hace
# con un plugin (herdr-panes / herdr-default-workspace).
set -eu

DIR="${1:-$PWD}"

# Debe ejecutarse dentro de Herdr (expone HERDR_ENV / el socket).
if ! command -v herdr >/dev/null 2>&1; then
    echo "dev-layout: 'herdr' no esta instalado o no esta en PATH." >&2
    exit 1
fi

# 1) Workspace nuevo en el directorio objetivo, con foco.
WS_JSON="$(herdr workspace create --cwd "$DIR" --label dev --focus)"
# El id del pane inicial lo resolvemos listando los panes del workspace.
# (Herdr imprime JSON; extraemos el primer pane_id con un parse minimo.)
LEFT="$(herdr pane list 2>/dev/null | grep -o '"id":"[^"]*"' | head -n1 | sed 's/.*:"//;s/"//')"

if [ -z "$LEFT" ]; then
    echo "dev-layout: no se pudo resolver el pane inicial." >&2
    exit 1
fi

# 2) Editor Helix en el pane izquierdo.
herdr pane run "$LEFT" "hx ."

# 3) Split a la derecha -> columna derecha.
herdr pane split "$LEFT" --direction right --cwd "$DIR" --no-focus
RIGHT="$(herdr pane list 2>/dev/null | grep -o '"id":"[^"]*"' | sed 's/.*:"//;s/"//' | sed -n '2p')"

# 4) En la derecha: lazygit arriba, split hacia abajo -> terminal.
if [ -n "$RIGHT" ]; then
    herdr pane run "$RIGHT" "lazygit"
    herdr pane split "$RIGHT" --direction down --cwd "$DIR" --no-focus
    # El pane de abajo queda como shell normal (sin comando).
fi

# Devuelve el foco al editor.
herdr pane focus "$LEFT" 2>/dev/null || true
