# Vistas desde helix — una tecla para ver el archivo con su visor

Helix es un editor de texto: no sabe dibujar una imagen ni renderizar un
markdown. Para eso están las herramientas (`glow`, `jq`, `chafa`, `mlr`),
que viven en el host y renderizan en el VPS.

La integración es una tecla: **Alt-v** abre un **popup** con el archivo
renderizado por el visor que le corresponde, y **Esc** lo cierra y te
devuelve a la edición. El buffer no se toca en ningún momento.

```
A-v = ':run-shell-command prim-view "%{buffer_name}"'
```

## Qué se ve según el tipo

| Archivo | Visor | Cómo se ve |
|---|---|---|
| `.md`, `.markdown` | `glow` | el markdown renderizado (tablas alineadas, títulos) |
| `.json` | `jq` | el JSON indentado y validado |
| `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.svg`... | `chafa` | la imagen dibujada con bloques de caracteres |
| `.csv`, `.tsv` | `mlr` | las filas del CSV |
| cualquier otro | `cat` | el texto tal cual |

## Por qué sin color

`prim-view` devuelve **texto plano**: quita los escapes ANSI de la salida
(glow emite colores SGR y hyperlinks OSC 8) porque **helix no interpreta
ANSI**: en el popup se vería la ESC y el código `[38;2;254;254;254m` como
texto literal. El layout se conserva, el color no.

Para ver una imagen **con color**, hay que salir a la shell, que es donde
el terminal sí interpreta los escapes:

```bash
chafa ~/work/captura.png          # con color
glow ~/work/prim/README.md | less # con color
```

## El dispatcher: `prim-view`

Está en `~/.local/bin/prim-view` (lo instala el rig) y sirve también
desde la shell:

```bash
prim-view ~/work/prim/README.md   # markdown renderizado, texto plano
prim-view ~/work/datos.csv         # las filas del CSV
prim-view captura.png              # la imagen en bloques
```

Útil cuando querés el contenido de un archivo **sin abrirlo en helix**: en
otro pane, en un `grep`, o piped a otra cosa:

```bash
prim-view notas.md | grep -i error
```

## Lo que helix no puede

- **Imágenes con color dentro del editor**: helix no tiene soporte de
  protocolo gráfico. El popup muestra la imagen en bloques, sin color.
- **Markdown con color**: mismo motivo. `glow` en la shell sí.
- **Navegar el CSV**: el popup muestra las filas, pero el análisis real
  (agregar, agrupar, joins) es de `duckdb`/`mlr`, que son de shell.

## Atajos relacionados

| Tecla | Qué |
|---|---|
| `Alt-v` | vista del archivo (este doc) |
| `Esc` | cerrar el popup y volver a editar |
| `Alt-g` | lazygit en un popup (integrado, recarga al volver) |
| `Ctrl-r` | recargar los buffers desde disco |
