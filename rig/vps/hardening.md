# Hardening del VPS — qué aplica el host y por qué

El script `rig/vps/harden-vps.sh` deja el host con tres piezas de base. Es
idempotente: se puede re-correr cuando cambia el repo.

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
3. **sshd solo por clave** — instala `/etc/ssh/sshd_config.d/00-prim-hardening.conf`
   desde `rig/vps/sshd-hardening.conf`: `PasswordAuthentication no`,
   `PermitRootLogin no`, `AuthenticationMethods publickey`.

## El detalle de cloud-init (no lo borres)

El drop-in se llama `00-...` a propósito. `sshd_config` hace
`Include /etc/ssh/sshd_config.d/*.conf` en la línea 12, y **OpenSSH se queda con
el primer valor que obtiene** para cada palabra clave. cloud-init escribe
`50-cloud-init.conf` con `PasswordAuthentication yes`: si ese gana, el
`PasswordAuthentication no` de la línea 57 del config principal queda anulado y
los logins con password vuelven a estar abiertos sin que se note.

```
sudo sshd -T | grep -E '^(passwordauthentication|permitrootlogin|authenticationmethods)'
```

Debe decir `no`, `no`, `publickey`. Si dice `yes`, el drop-in de cloud-init
ganó: revisá que `00-prim-hardening.conf` exista y sea anterior.

## Cómo se aplica

```
sudo ./rig/vps/harden-vps.sh              # aplica y arma auto-revert de 120s
sudo ./rig/vps/harden-vps.sh --confirm    # verificado: cancela el revert
sudo ./rig/vps/harden-vps.sh --status     # estado actual
```

El auto-revert de 120s borra la tabla de firewall si no confirmás: es la red
de seguridad para un cambio que te puede dejar afuera. **El revert solo toca
el firewall**: el drop-in de sshd se valida con `sshd -t` antes de recargar, y
la recarga (`reload`, nunca `restart`) deja vivas las sesiones ya abiertas.

## Revertir el sshd a password

```
sudo rm /etc/ssh/sshd_config.d/00-prim-hardening.conf && sudo systemctl reload ssh
```

## Por qué solo clave

En los últimos 60 días, **todos** los logins exitosos fueron `publickey` como
`debian` (234). Los intentos con password fueron fuerza bruta: 67k en 14 días,
28k contra `root` y 454 contra `debian`. Root nunca entró por ssh. Con el
puerto 22 abierto a Internet, desactivar el password no es opcional: es la
diferencia entre un servidor con ruido de fondo y uno sin superficie de
entrada por password.
