# prim-clipboard — imagenes del portapapeles del cliente al agente, efimeras

Head: el portapapeles vive en el cliente y el agente vive en el VPS. La
imagen viaja como ARCHIVO a un inbox efimero y su RUTA viaja como texto.
Nada persiste en `~/work`: la limpieza corre dos veces por dia y borra solo
lo que lleva mas de 12 horas.

## Specs

- S1 (user, 2026-11-07): "Parece que no me gusta tanto la idea generalmente se toman pantallzos que no se queiren persistir y se le pegna al agnte así que sinceramente prefiero bindera en todos los casoa sie sposible y si no como automatizamso al limpieza, peude ser cuando pase ciert acantidad e tiempo".
- S2 (user, 2026-11-07): "Prefiero qeu solo se ejcute unas 2 veces al ´dia y que solo bore lo que lleva más de 12 horas allí". Decision de unidades: "1. a y b" — el tool `img-grab` con el inbox y la limpieza, Y los bindings por cliente, salen juntos en esta feature.
- S3 (user, 2026-11-07, contexto de la maquina): "el principla se´ria un daylight DC1, dqeu es andprio, aun no elijo terminal" + "windows qeu tien wezterm y herdr y linux que tiene gosthy y herdr" + "en el iphone uso moshi". La VPS es el host de los agentes y las herramientas; los clientes son el Daylight DC-1 (Android/Termux), Windows (WezTerm + herdr), Linux (Ghostty + herdr) e iPhone (moshi).
- S4 (parent, design, derivado de la conversacion): el contrato es una RUTA, no una imagen. Pegar TEXTO por SSH funciona nativo (el terminal manda los caracteres); OSC 52 solo hace falta en el sentido inverso (VPS -> portapapeles del cliente). El portapapeles del VPS es irrelevante: la VPS no tiene X, ni Wayland, ni `wl-paste`/`xclip`, asi que ahi no hay nada que leer ni que escribir.
- S5 (parent, design): el inbox vive en `~/.cache/prim/inbox/` en la VPS: fuera de `~/work` (no aparece en git, no va al share, no se sincroniza al telefono) y en XDG cache, que es disposable por definicion. `~/.cache/prim/inbox/last` apunta a la imagen mas reciente, para que el binding baste con decir "leé la ultima imagen".
- S6 (parent, design): `img-grab` es un solo script con ramas por plataforma, mismo contrato: leer la imagen del portapapeles -> subirla al inbox de la VPS -> imprimir la ruta -> copiar la ruta al portapapeles local como TEXTO. Lecturas nativas: `termux-clipboard-get` (Termux:API) en DC-1, `wl-paste`/`xclip` en Linux, `Get-Clipboard -Format Image` en Windows. Subida por `scp` (SSH ya configurado). Modo `--file` para poder probar el flujo sin portapapeles.
- S7 (parent, design): la limpieza es un timer de systemd de usuario que corre DOS VECES POR DIA y borra UNICAMENTE los archivos con mas de 12 horas en el inbox. Sin prune-on-upload y sin borrado por parte del agente: una sola politica de limpieza, la del usuario.
- S8 (parent, constraint): los bindings de terminal, el tool en la lista global y el timer como unidad home-manager pertenecen a los modulos Nix del rig, y eso solo queda vivo cuando aterrice P6 (Nix). Por eso la feature tiene dos planos: (i) lo que funciona ya en la VPS (scripts planos que se instalan a mano), (ii) lo que aterriza con P6 (modulos Nix, bindings, unidad systemd). El repo guarda los archivos crudos en `rig/systemd/` y `rig/tools/`, y el modulo Nix los importa: una sola fuente, sin duplicar.
- S9 (parent, decision): el perfil `dc1` del rig es el Daylight DC-1 (Android/Termux) — el nombre coincide con el dispositivo. Los bindings de Linux (Ghostty) y Windows (WezTerm) se entregan como snippet documentado para aplicar a mano: esas maquinas no son parte de los perfiles del rig, y no se agrega un perfil nuevo en esta feature.

## Tasks

| ID | Specs | Route | Commit |
|----|-------|-------|--------|
| C1: inbox efimero + timer de limpieza (2x/dia, TTL 12h) como archivos crudos en `rig/systemd/`, instalables a mano en la VPS hoy | S5,S7,S8 | parent | 61b7bfc |
| C2: `rig/tools/img-grab` (POSIX sh, ramas termux/linux/windows, modo `--file`, sube al inbox, imprime la ruta, la copia al portapapeles local como texto) | S4,S6 | writer | |
| C3: `rig/tools/img-grab.ps1` (Windows: `Get-Clipboard -Format Image` -> scp -> ruta en el portapapeles) | S6 | writer | |
| C4: modulos Nix del rig: `img-grab` en la lista global de tools, importacion de las unidades systemd, bindings de Termux (extra keys) en el perfil `dc1` | S6,S8,S9 | writer | |
| C5: docs `rig/docs/imagenes.md`: el flujo por cliente, la convencion "leé la ultima imagen", la politica de limpieza | S1,S2,S3,S5 | writer | |
| C6: verificacion del padre (shellcheck, sandbox funcional de subida y TTL, reporte honesto del gap de Nix) + commits por unidad | all | parent | |

## Log

- L1 (2026-11-07, user): "Tengo un problema cuando estoy conectado por ssh no me esta dejando copiar y pegar imagenes como organizo esto quiero porder hacerlo".
- L2 (2026-11-07, parent): diagnostico — la VPS no tiene portapapeles (sin X, sin Wayland, sin `wl-paste`/`xclip`); herdr trae el comando `remote_image_paste` pero no esta boundado en la config del rig y no podria leer un portapapeles local del VPS. Ojo: pegar TEXTO por SSH funciona nativo; OSC 52 solo va en el sentido inverso.
- L3 (2026-11-07, user): correccion de contexto — "Parece que no tuvieras el proyecto, sabes claramente que hay un VPS en linea que es el que realmente tiene los agentes y las herramientas de desarrollo mas poderosas, adicional hay un cliente que se conecta a este y permite usarlo". El VPS es el host de los agentes; los clientes son DC1 (Android/Termux), Windows (WezTerm+herdr), Linux (Ghostty+herdr) e iPhone (moshi).
- L4 (2026-11-07, user): "en relidad no me gust atanto la idea genralmnte se toman pantallzos qeu no se queiren persistir y se le pegna al agnte asi que sinceramente prefiero bindera en todos los casoa sie sposible y si no como automatizamso al limpieza, peude ser cuando pase ciert acantidad e tiempo".
- L5 (2026-11-07, user): "Prefiero qeu solo se ejcute unas 2 veces al ´dia y que solo borre lo que lleva más de 12 horas allí" + "1. a y b" (el tool y los bindings, juntos).
- L6 (2026-11-07, parent): el peor caso de retencion es ~24h (una imagen que cae justo despues de una pasada espera la siguiente). Decision del usuario, anotada para que no surprise.
- L7 (2026-11-07, parent): C1 entregado en vivo (antes de P6): `rig/tools/prim-inbox-cleanup` instalado en `~/.local/bin/`, las unidades crudas en `~/.config/systemd/user/`, timer habilitado y con primer disparo `Result=success` (prxima corrida 03:20 UTC). El prune borra solo lo de mas de 12h, quita el puntero `last` si queda dangling y no toca `~/work`. Sandbox verificado: dry-run lista solo lo viejo, prune real borra solo eso, `last` sobrevive o se limpia.
- L8 (2026-11-07, parent): delivery strategy — la branch `feat/files-sync` (feature `files`, commit ea0da49, 383 lineas) queda como PR propio; esta feature `prim-clipboard` vive en `feat/clipboard-inbox`, ENCADENADA sobre la anterior (PRs apilados: el de clipboard se rebasa encima cuando el de files mergue). Motivo: el harness marco que la branch unica ya habia pasado el corte de ~400 lineas al mezclar dos features. Cada commit de esta feature se mantiene por debajo de ese corte: C1 (100 lineas) ya esta; C2+C3+C5 (tool + doc) y C4 (modulos Nix) van en commits separados.
