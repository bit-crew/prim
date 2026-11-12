# Archivos moviles — ~/work por SMB dentro del tailnet

El componente `files` comparte `~/work` por SMB (Samba) para que
iPhone (app Archivos), Android, Windows y Linux lo abran de forma nativa,
como una carpeta mas. El servidor escucha **solo** en la interfaz
`tailscale0`: sin Tailscale no hay acceso y no se abre ningun puerto a
Internet (S9-S11). Sin Nextcloud ni servicios de sincronizacion.

## Instalacion

```
prim install rig        # perfil vps: activa el entorno y encadena files
prim install files      # reinstalacion/reparacion explicita
```

Es parte del workstation: `prim install rig` con perfil `vps` lo configura
automaticamente al final, vacio o lleno da igual (un VPS recien creado ya
queda compartiendo para cuando lleguen los archivos). No aparece en el
menu del instalador: viaja con el rig. La clave `files` queda como
comando explicito para reparar o desinstalar.

Durante la instalacion:

- Se pide **dos veces** la clave SMB del usuario dedicado `filesmb`
  (nunca viaja por argumentos ni variables de entorno).
- Si el VPS no estaba en tu tailnet, `tailscale up` muestra una URL de
  autenticacion: abrela, autoriza la maquina y vuelve a la terminal.

## Conectar

Necesitas la IP del VPS en el tailnet (empieza por `100.`): `tailscale ip -4`
en el VPS, o mira la maquina en la app de Tailscale. Ejemplo en adelante:
`100.101.102.103`.

- **iPhone** — app Archivos > menu de tres puntos > *Conectar al servidor*
  > `smb://100.101.102.103/work` > usuario `filesmb` y tu clave. El share
  queda disponible en "Compartido" para abrir y guardar archivos.
- **Android** — cualquier gestor con SMB (CX File Explorer, Solid
  Explorer): nueva conexion SMB a `100.101.102.103`, carpeta `work`.
- **Windows** — con Tailscale for Windows instalado: `\\100.101.102.103\work`
  en el Explorador.
- **Linux** — `smb://100.101.102.103/work` en Files/Nautilus.

Escribes con normalidad y los archivos caen en `~/work` del VPS con grupo
`primfiles` (archivos 0664, directorios 0775): tu usuario y `filesmb`
comparten lectura y escritura.

## Clave SMB

Se define durante la instalacion (se pide dos veces). Para cambiarla:

```
sudo smbpasswd filesmb
```

Si perdiste la clave, re-ejecuta `prim install files` desde una terminal:
todo es idempotente y vuelve a pedirla.

## Diagnostico

```
prim doctor
```

Con el componente instalado aparece una seccion `files`: estado de `smbd`
(activo), `nmbd` (enmascarado es lo esperado), `tailscaled`, la IP de
tailscale y si la ruta del share existe. Si el usuario `filesmb` o el grupo
`primfiles` ya no existen en el sistema, doctor lo avisa y
`prim install files` lo repara.

## Desinstalar

```
prim uninstall files
```

- `~/work` **nunca** se toca: tus archivos quedan intactos (S4).
- Tailscale **se conserva**: es la base de red del VPS y `tailscaled`
  sigue activo. Si de verdad quieres quitarlo: `sudo tailscale down` y
  `sudo apt-get remove tailscale`.
- Se retiran: el servicio smbd, la configuracion de Samba (se restaura el
  smb.conf previo si existia; si prim lo creo, se elimina), el usuario
  `filesmb` y el grupo `primfiles`.

## Taildrop

Para mandar un archivo suelto de un dispositivo a otro sin montar nada,
Taildrop (compartir directo desde la app de Tailscale) sigue siendo lo mas
rapido. `files` es para acceso continuo a `~/work` desde cualquier app.

## Consumo

- `smbd` siempre activo: ~30 MB de RAM en reposo (aceptado en S10).
- `nmbd` deshabilitado y enmascarado: no corre.
- La activacion de smbd por socket se evaluo y se descarto por poco fiable
  (S10); queda documentada para reabrir si la situacion cambia.

## Copiar en las dos direcciones sin conexion

Sin daemon de sincronizacion y sin servicios en la nube: el share es la
fuente de verdad y el cliente guarda una copia propia para los momentos sin
conectividad. Son dos operaciones manuales que se corren **siempre en este
orden**: `pull` (manda el VPS) antes que `push` (manda el cliente).

Borrar un archivo se hace **una sola vez**: lo borras en el cliente y el
`push` lo saca del VPS. Al reves, lo que borras en el VPS desaparece del
cliente en el `pull` siguiente. Nada queda borrado dos veces.

### Donde corre

En una tablet Android con Termux (el iPhone no puede correr rclone, asi que
alli sigue siendo copia archivo por archivo desde la app Archivos):

```
pkg install rclone
termux-setup-storage
```

El remoto de rclone se crea una vez, apuntando al share por la IP del
tailnet, con el usuario `filesmb` (la clave se pide en el momento y queda
ofuscada en el config de rclone, nunca en el script):

```
rclone config
#   n            nuevo remoto
#   name         vps
#   Storage      smb
#   host         100.101.102.103
#   user         filesmb
#   pass         (la clave, en el momento)
#   port         445
```

La copia local vive en `~/storage/documents/work`, o donde digas con
`--dest` (o con `WORK_SYNC_DEST`).

### El script

Se baja del repo y se deja en el Termux del cliente:

```
curl -fsSL https://raw.githubusercontent.com/bit-crew/prim/main/files/sync-work.sh \
  -o ~/sync-work.sh && chmod +x ~/sync-work.sh
```

Trae del VPS y deja la copia local al dia:

```
~/sync-work.sh pull
```

Manda la copia local al VPS (despues de un `pull`):

```
~/sync-work.sh push
```

La primera vez, en seco, para ver que se va a mover:

```
~/sync-work.sh pull --dry-run
```

### Que hace y que no hace

| Que | Como |
|-----|------|
| Orden | siempre `pull` antes que `push` |
| Por defecto | gana el archivo mas nuevo de los dos lados (`--update`) |
| Con `--force` | gana la direccion del comando (push: el cliente; pull: el VPS) |
| Borrados | se propagan: los hacés una vez y se van de los dos lados |
| Borrado recibido | el archivo queda en `.sync-deleted/<timestamp>/` del lado que lo recibe |
| Archivo pisado | queda como `<archivo>.bak` al lado, en el lado que recibio la copia |
| Archivo creado en el VPS mientras el cliente estaba offline | un `push` NUNCA lo borra; el `pull` siguiente lo baja |
| `.git`, `target`, `node_modules`, `.venv`, `dist`, `.next` | se excluyen siempre |
| La copia local | NO es un clon de git: `.git` queda en el VPS y los commits se hacen alla |

En este VPS el arbol completo pesa 915M, pero con las exclusiones lo que
se copia son 157M.

El script guarda `.sync-manifest` en la copia local: la lista de archivos
que ambos lados tenian en la ultima operacion. Es lo que permite propagar
los borrados con precision: solo se borra en el otro lado lo que vos borraste
explicitamente, nunca nada mas.

### Recuperar algo

Si borraste de mas, el archivo no se perdio: quedo apartado.

```
# en el VPS (recibio un borrado hecho en el cliente)
~/work/.sync-deleted/<timestamp>/ruta/del/archivo

# en el cliente (recibio un borrado hecho en el VPS)
~/storage/documents/work/.sync-deleted/<timestamp>/ruta/del/archivo
```

Copiá el archivo de vuelta a su ruta y segui. Lo mismo con un `<archivo>.bak`
cuando lo que se piso fue una version.

### Ojo

- La comparacion es por tamano y fecha de modificacion, no por contenido.
- No lo corras mientras estas editando en el otro lado: la copia no tiene
  merge, el merge lo hace git en el VPS.
- El share tiene que estar activo (`prim install files`) y Tailscale
  arriba en el cliente.
