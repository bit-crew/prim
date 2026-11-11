# Mosh — sesión remota que sobrevive al roaming

Mosh (mobile shell) mantiene la sesión cuando tu conexión cambia de Wi-Fi a
datos móviles (o al revés), cuando la IP cambia, o cuando perdés Internet
temporalmente: al volver, el terminal está donde lo dejaste. Es ligero (un
binario, UDP, sin daemon) y encaja con la arquitectura: viaja **solo por
Tailscale** y **SSH sigue siendo la autenticación**.

Diseño en una línea: `mosh-server` en el VPS (perfil `vps` del rig), UDP
60000–60010 abierto únicamente por `tailscale0`, y cada dispositivo con su
cliente Mosh.

## Por qué así

- **SSH no se toca.** El cliente Mosh abre primero una conexión SSH normal,
  lanza `mosh-server` en el VPS y recibe puerto + clave de sesión; a partir de
  ahí habla UDP. La autenticación, las claves y el login siguen siendo 100%
  SSH: no hay claves nuevas ni puerto nuevo expuesto.
- **Solo por Tailscale.** El rango UDP se acepta únicamente si entra por
  `tailscale0`. Desde Internet el puerto no existe; no hay superficie nueva.
- **Ligero.** `mosh` es un paquete de nixpkgs sin configuración ni servicio
  systemd: si nadie conecta, no corre nada.

## Servidor (VPS)

`mosh-server` viene con el rig: el perfil `vps` del flake lo instala
(`rig/nix/modules/tools.nix`). Nada manual — `prim install rig` (o el próximo
`prim update rig`) lo deja en PATH.

Requisito blando de mosh: locale UTF-8 en ambos extremos. Debian 13 ya trae
`C.UTF-8`; si algún cliente se queja de locale, `export LANG=C.UTF-8` antes de
conectar.

## Firewall: UDP 60000–60010 solo por Tailscale

Regla nftables (misma tabla que `rig/host/harden-vps.sh`, rama
`feat/host-hardening`):

```
iifname "tailscale0" udp dport 60000-60010 accept
```

Va en la cadena `input` de la tabla `vps_hardening`, antes del `drop` final
(que hoy solo permite lo, established/related, icmp, TCP 22 y TCP 8443). Sin
esa línea, mosh no recibe los paquetes UDP y muere con "No connection
received".

- 11 puertos = hasta 11 sesiones Mosh simultáneas (una por puerto).
- Aplicación en el host (hasta que `feat/host-hardening` fusione y la regla
  viva en el script, ver `rig/TODO.md`):

```
sudo nft add rule inet vps_hardening input position 5 iifname "tailscale0" udp dport 60000-60010 accept
```

(`position 5` la deja antes del `drop`; verificar con `sudo nft list chain
inet vps_hardening input`.)

## Clientes (por dispositivo)

Todos conectan a la **IP del tailnet** del VPS (`100.x.y.z`, `tailscale ip -4`
en el VPS), no a la IP pública. Y todos con el rango acotado:
`mosh -p 60000:60010`.

| Dispositivo | Cliente | Instalación |
|-------------|---------|-------------|
| iPhone / iPad | Blink Shell o Moshi | App Store |
| Android (DC-1) | Termux | `pkg install mosh` |
| Windows | WSL | `sudo apt install mosh` dentro de WSL, con Tailscale activo en el mismo entorno |

En Windows/WSL2, corré `tailscaled` dentro de WSL (o usá Tailscale en modo
userspace) para que el UDP llegue de verdad al cliente.

## Conexión

```
mosh -p 60000:60010 debian@100.x.y.z
```

El login, la clave y todo el arranque van por SSH (puerto 22); después la
sesión vive en UDP dentro del tailnet. Cambiá de red, cerrá la laptop, vuelva
Internet: la sesión sigue.

Límites conocidos: mosh no guarda scrollback del lado del servidor (Blink lo
resuelve en el cliente) y si el VPS reinicia, la sesión muere igual que una
SSH (pero el cliente reconecta solo).

## Diagnóstico

En el **VPS**:

```
command -v mosh-server          # instalado por el perfil vps del rig
sudo nft list table inet vps_hardening | grep 60000   # regla presente
tailscale ip -4                 # IP que usan los clientes
```

Desde un **cliente**:

```
mosh -p 60000:60010 debian@100.x.y.z
```

Síntomas típicos:

- **"No connection received"** — el SSH inicial funcionó (si no, es problema
  de clave/SSH), pero el UDP no llega: falta la regla nft, el cliente no está
  en el tailnet, o el puerto quedó fuera del rango 60000–60010.
- **"mosh requires a UTF-8 native locale"** — fijar `LANG=C.UTF-8` en el
  extremo que se queja.
