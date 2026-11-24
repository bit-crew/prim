# Hardening del VPS — qué aplica el host y por qué

El script `rig/vps/harden-vps.sh` deja el host con tres piezas de base. Es
idempotente: se puede re-correr cuando cambia el repo.

**Aplicación automática**: `prim install rig` encadena el hardening después de
activar el perfil (igual que encadena `files/install.sh`), así que cada
actualización del rig re-aplica el firewall y el drop-in de sshd. A mano:

```
sudo ./rig/vps/harden-vps.sh              # aplica y arma auto-revert de 120s
sudo ./rig/vps/harden-vps.sh --confirm    # verificado: cancela el revert
sudo ./rig/vps/harden-vps.sh --status     # estado actual
```

## La política de puertos

| Puerto | Quién entra | Nota |
|---|---|---|
| **22 (ssh)** | **Internet** | el único puerto público. sshd acepta clave pública **o** usuario + password; nada más |
| 445 (SMB) | solo `tailscale0` | el share de `~/work` |
| 60000-60010 (mosh) | solo `tailscale0` | ver `rig/vps/mosh.md` |
| cualquier otro | nadie | `drop` |

**Si se apaga Tailscale**: el 22 sigue abierto (el VPS se alcanza desde fuera
del tailnet), y se caen el share SMB y mosh hasta que vuelva. Ningún otro
puerto queda expuesto: no hay superficie pública además del ssh.

El puerto 22 se mantiene público a propósito: desde la red del usuario (ISP
móvil en Chile) los logins entran por la IP pública, no por el tailnet.
Moviéndolo a `tailscale0` el VPS quedaría inalcanzable fuera de la VPN.

## Las tres piezas

1. **Swap de emergencia** — `/swapfile` de 2G con `vm.swappiness=10`. El VPS
   tiene 3.7G de RAM y ~1.3G libres con los agentes corriendo; sin swap el OOM
   killer mata procesos al azar en los picos de login.
2. **Filtro nftables de entrada** — tabla `inet vps_hardening`: acepta `lo`,
   `established/related`, ICMP/ICMPv6, TCP 22 y el rango UDP 60000-60010 de
   mosh (solo por `tailscale0`, ver `rig/vps/mosh.md`). Todo lo demás cae en
   `drop`. Los puertos que pertenecen a una app (el 445 de files) viven en
   `/etc/nftables.d/prim-ext/*.nft`, se aplican **después** de la base y
   sobreviven cada re-aplicación.
3. **sshd del puerto 22** — instala `/etc/ssh/sshd_config.d/00-prim-hardening.conf`
   desde `rig/vps/sshd-hardening.conf`:
   - `PasswordAuthentication yes` + `PubkeyAuthentication yes` +
     `AuthenticationMethods publickey password` → entra con **clave pública o
     usuario y password**;
   - `KbdInteractiveAuthentication no`, `PermitEmptyPasswords no`,
     `PermitRootLogin no`, `AllowUsers debian` → **ningún otro tipo de
     conexión**, ni otro método de autenticación ni otra cuenta;
   - `LoginGraceTime 20`, `MaxAuthTries 5`, `MaxStartups 10:30:30` → el
     fuerza bruta que ve el puerto se topa con los topes de sshd.

## El detalle de cloud-init (no lo borres)

El drop-in se llama `00-...` a propósito. `sshd_config` hace
`Include /etc/ssh/sshd_config.d/*.conf` en la línea 12, y **OpenSSH se queda con
el primer valor que obtiene** para cada palabra clave. cloud-init escribe
`50-cloud-init.conf` con `PasswordAuthentication yes`: si ese gana, el
`PasswordAuthentication no` de la línea 57 del config principal queda anulado y
los logins con password se abren sin que se note.

```
sudo sshd -T | grep -E '^(passwordauthentication|permitrootlogin|authenticationmethods|allowusers)'
```

Debe decir `yes`, `no`, `publickey password` y `debian`. Si el `password` sale
`no` con el archivo presente, el drop-in de cloud-init ganó: revisá que
`00-prim-hardening.conf` exista y sea anterior.

## Revertir el sshd a la config del sistema

```
sudo rm /etc/ssh/sshd_config.d/00-prim-hardening.conf && sudo systemctl reload ssh
```

## El auto-revert

El auto-revert de 120s borra la tabla de firewall si no confirmás: es la red
de seguridad para un cambio que te puede dejar afuera. **El revert solo toca
el firewall**: el drop-in de sshd se valida con `sshd -t` antes de recargar, y
la recarga (`reload`, nunca `restart`) deja vivas las sesiones ya abiertas.

## Por qué el puerto 22 acepta password

Porque es lo que se pidió: entrar con clave pública **o** con usuario y
password, y bloquear cualquier otro tipo de conexión. La red desde la que el
usuario entra (ISP móvil, Chile) llega por la IP pública, y no todos los
clientes (la app del iPhone) llevan clave a mano.

El costo de esa decisión es que el puerto 22 ve fuerza bruta constante
(67k intentos en 14 días, 28k contra `root` y 454 contra `debian`). Contra
eso el drop-in pone topes de sshd (`LoginGraceTime 20`, `MaxAuthTries 5`,
`MaxStartups 10:30:30`), cierra el login de `root`, deja `AllowUsers debian` y
no acepta teclado-interactivo. Lo que **no** cubre es un password débil: si
ese password cae, el login entra. Por eso el password de `debian` tiene que
ser largo y único; `fail2ban` es el complemento natural si se quiere banear
los reincidentes (hoy no está instalado).
