# Herramientas de datos y markdown — cliente (DC-1)

El DC-1 (Android + Termux) es un cliente del VPS: ahi viven tus manos, no
los datos. Los CSV y los markdown que subis (por el share SMB o por `croc`)
caen en `~/work` **del VPS**.

Esta pagina instala en el cliente las herramientas para mirar esos datos de
todos modos, y explica que hay de cada lado y por que.

## Instalar

```bash
curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/rig/client/install-herramientas.sh | sh
```

Es idempotente: correrla de nuevo no rompe nada. Instala:

| Herramienta | Como llega | Que es |
|---|---|---|
| `glow` | `pkg install glow` | renderizador de Markdown para la terminal |
| `miller` (`mlr`) | `pkg install miller` | procesador de CSV/TSV/JSON, con browser interactivo |
| `chafa` | `pkg install chafa` | dibuja imagenes en la terminal con ANSI truecolor |
| `qsv` | binario estatico musl aarch64 | toolkit de analisis de CSV, en Rust |

`qsv` no existe como paquete de Termux, asi que el script baja el release
oficial de upstream (`dathere/qsv`), el build `aarch64-unknown-linux-musl`:
es estatico, asi que corre en Android sin glibc. Para actualizarlo:

```bash
QSV_VERSION=<nueva> sh install-herramientas.sh
```

## Que hay en cada lado, y por que

| | glow | miller | duckdb | qsv | chafa |
|---|---|---|---|---|---|
| **VPS** (`rig/nix/modules/tools.nix`, perfil `vps`) | ✅ | ✅ | ✅ | — | ✅ |
| **DC-1** (este script) | ✅ | ✅ | — | ✅ | ✅ |

La razon del reparto es simple: **el visor va donde esta el archivo**.

- Los CSV que subis al share viven en el VPS. Ahi corre `duckdb` (SQL) y
  `mlr browse` (navegar el archivo), en nativo, sin copias. Y como el render
  ocurre en el VPS, el cliente solo recibe ANSI: por eso estas herramientas
  funcionan igual desde SSH o Mosh, sin instalar nada en el telefono.
- `duckdb` no esta en el DC-1 porque no existe para Termux, y porque no
  hace falta: los datos no estan ahi. Si alguna vez los bajaste al telefono
  y queres SQL local, es `python3` + `sqlite3` de la stdlib.
- `qsv` si esta en el cliente: es el que te da las estadisticas de un CSV
  rapido, sin pensar SQL.
- `chafa` esta en los dos lados: en el VPS y en el DC-1. Es la que dibuja
  una imagen en la terminal con ANSI truecolor, sin depender de que el
  terminal de afuera soporte el protocolo grafico. En el VPS queda porque
  las imagenes del share estan ahi; en el telefono, para las fotos que
  tenes en el dispositivo.

Y si estas en el cliente y el CSV esta en el VPS, no copies nada (en el VPS
estan `duckdb` y `miller`):

```bash
ssh vps 'mlr --csv head -n 5 ~/work/datos.csv'
ssh vps 'duckdb -c "select categoria, count(*) from ~/work/datos.csv" group by 1'
```

## Que hace cada una

### `glow` — ver Markdown

```bash
glow README.md                    # render a stdout
glow -p docs/guia.md              # modo pager (navegas con j/k)
glow -w 120 README.md             # ancho de wrap (default 80; las tablas se cortan)
glow -s light README.md           # tema: dark | light | auto | dracula | pink
```

Exporta a HTML, que es la via para leerlo en el iPhone: lo generas, lo
dejas en el share y lo abris con Safari desde Archivos.

```bash
glow -e html nota.md > nota.html
```

### `miller` (`mlr`) — CSV como Unix

```bash
mlr --csv head -n 5 datos.csv              # primeras filas
mlr --csv cut -f id,precio datos.csv       # quedarse con 2 columnas
mlr --csv filter '$precio > 100' datos.csv # filtrar
mlr --csv sort -f precio datos.csv         # ordenar
mlr --icsv --ojson cat datos.csv | jq .    # CSV -> JSON -> jq
mlr browse datos.csv                       # browser interactivo
```

### `qsv` — analizar CSV

```bash
qsv stats datos.csv              # por columna: min/max/mean/median, nulos, cardinalidad
qsv frequency datos.csv -c cat   # valores mas frecuentes
qsv schema datos.csv             # tipos inferidos
qsv search datos.csv -s producto # buscar
qsv join a.csv b.csv -k id       # join entre dos CSV
qsv sample datos.csv --random 10
```

### `duckdb` — SQL (solo en el VPS)

```bash
duckdb -c "select * from 'datos.csv' limit 10"
duckdb -c "select categoria, count(*) n, avg(precio) from 'datos.csv' group by 1"
duckdb -c "copy (select * from 'datos.csv') to 'out.parquet'"
```

Y la composicion con `glow`: `duckdb` puede devolver markdown y `glow` lo
renderiza.

```bash
duckdb -c ".mode markdown
select categoria, count(*) n from 'datos.csv' group by 1" | glow -
```

### `chafa` — ver imagenes (en el VPS)

Helix no dibuja imagenes (el pedido upstream esta cerrado) y Yazi solo las
muestra si el terminal de afuera habla el protocolo kitty graphics.
`chafa` no depende de nada de eso: dibuja con ANSI truecolor, asi que se ve
en cualquier terminal, por SSH pelado o en un pipe.

```bash
chafa foto.png                     # a stdout, con el tamano del terminal
chafa --size 60x30 foto.png        # tamano fijo
chafa -f symbols foto.png          # solo simbolos (ASCII, util por SSH)
chafa --format kitty foto.png      # protocolo kitty, si el terminal lo banca
chafa foto.png > foto.txt          # vuelca el render a un archivo
```

Formatos que lee: PNG, JPEG, GIF, WebP, SVG, TIFF, AVIF, JXL, QOI y XWD.

La imagen tiene que estar en el VPS (o en el share), asi que desde el
cliente:

```bash
ssh vps 'chafa ~/work/captura.png'
```

Si la terminal del cliente soporta el protocolo kitty graphics (WezTerm,
kitty y Ghostty lo hacen), `chafa --format kitty ~/work/captura.png` la
dibuja con la fidelidad nativa del terminal, en vez del render ANSI.

## Nota: por que esto es un script y no el perfil `dc1`

El modulo `rig/nix/modules/termux.nix` lo dice: Termux vive en
`/data/data/com.termux/files/home`, no en `/home/<user>`, y hasta que ese
home apunte al de Termux el perfil `dc1` no puede escribirse en el
dispositivo. Por eso el cliente instala a mano lo que necesita, igual que
`rig/client/install-img-grab.sh`.
