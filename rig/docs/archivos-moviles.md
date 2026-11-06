# Archivos moviles — ~/work por SMB dentro del tailnet

El componente opcional `files` comparte `~/work` por SMB (Samba) para que
iPhone (app Archivos), Android, Windows y Linux lo abran de forma nativa,
como una carpeta mas. El servidor escucha **solo** en la interfaz
`tailscale0`: sin Tailscale no hay acceso y no se abre ningun puerto a
Internet (S9-S11). Sin Nextcloud ni servicios de sincronizacion.

## Instalacion

```
prim install files
```

Es un componente opcional: `prim install` con "todo" NO lo instala; hay que
pedirlo explicito (la clave `files` o su numero en el menu).

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
