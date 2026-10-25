# VPS en OVHcloud — compra manual y configuración

Este documento cubre la compra manual del VPS (una vez) y cómo configurarlo con
`install.sh`. El VPS se compra a mano; todo lo repetible (instalar el entorno) se
automatiza.

## Por qué la compra es manual

El VPS clásico de OVH es un producto que se **compra** (genera factura). La API
permite ordenarlo vía "shopping cart", pero el pago introduce fricción y no
aporta valor si solo creas el VPS una vez. Por eso: se compra a mano, y todo lo
repetible (reinstalar SO, configurar, bootstrap) se automatiza.

## Producto elegido

| Item | Valor |
|------|-------|
| Plan | **VPS-1 (2027)** |
| vCores | 2 |
| RAM | 4 GB |
| Disco | 40 GB SSD NVMe |
| Backup | Diario incluido |
| Tráfico | Ilimitado, 500 Mbps |
| Precio | ~$4.54 / mes (ex. taxes) |
| Sistema | **Debian 13 (trixie)** |

Dimensionado para el uso real: edición (Helix), LSPs, tests unitarios con
`SparkSession` local, y trabajo de datos con Polars/DuckDB. Spark **real** corre
en cluster externo, no en el VPS. Si algún día se necesita más RAM, migrar a
VPS-2/VPS-3 es trivial porque el entorno es reproducible desde código.

Sin add-ons: el backup diario ya viene incluido; los datos persistentes viven en
Cloudflare R2, no en disco local. No se contrata Premium backup, Snapshot ni
Additional storage.

## Pasos de compra (panel OVHcloud)

1. Entrar al [panel de OVHcloud](https://www.ovhcloud.com/) e iniciar sesión
   (o crear cuenta si no existe).
2. Menú **Bare Metal Cloud → VPS → Order / Pedir**.
3. Elegir el rango **VPS-1** (2 vCore / 4 GB / 40 GB NVMe). Si prefieres más
   holgura y hay stock, VPS-2 (8 GB) o VPS-3 (12 GB) son válidos; el resto del
   flujo no cambia.
4. Sistema operativo: pestaña **Distribution only** → **Debian**, versión
   **13**. La distro se puede reinstalar luego por API, pero elige Debian 13 ya
   para evitar un paso extra. No uses "Distribution with application" (mete
   stack que no necesitas).
5. Datacenter/región: elegir la más cercana disponible con baja latencia
   (ej. `BHS` Beauharnois CA para Latam; `GRA` Gravelines FR; `SGP` Singapur).
   Si un plan aparece "temporarily unavailable", probar otras regiones.
6. Facturación: **mensual sin compromiso**.
7. Confirmar y pagar.
8. Esperar el correo de aprovisionamiento (suele ser minutos). El correo trae la
   IP, el usuario (`debian`) y un enlace para generar la password inicial.

## Datos del VPS tras la compra

Del correo de aprovisionamiento de OVH necesitas:

- **IP pública** (IPv4).
- **Usuario inicial** (normalmente `debian`).
- **Password temporal**: se genera desde el enlace seguro del correo de OVH.

## Prerrequisito: cambiar la password (manual, una vez)

OVH obliga a cambiar la password temporal por una permanente en el **primer
login**. Este paso es interactivo (flujo de seguridad del sistema) y **se hace a
mano antes de usar `install.sh`**:

```
ssh debian@<IP>
```

Al entrar con la password temporal, el sistema pedirá la temporal de nuevo y la
nueva password permanente (dos veces). A partir de ahí, la **password
permanente** es la que usa `install.sh`.

No se automatiza: es una sola vez por VPS y es un flujo interactivo de PAM que no
encaja en la ejecución de un comando remoto.

Configurar el VPS **no requiere clave SSH**: se hace desde cualquier PC con
IP + usuario + password permanente. Enlazar un cliente (tablet, laptop) al VPS
por clave SSH es tarea de la configuración del cliente, no del VPS.

## Configurar el VPS con un comando

Con la password permanente ya establecida, desde cualquier PC (no requiere clonar
nada ni tener Python; solo `sh` y `ssh`):

```
curl -fsSL https://raw.githubusercontent.com/bit-crew/devlab/main/install.sh | sh
```

Elige la opción **2) VPS remoto** (o salta el menú con `sh install.sh vps`).
El script pregunta:
- IP pública del VPS
- Usuario SSH (default `debian`)

Se conecta por SSH y **pide la password permanente del VPS** cuando la necesita.
Luego, de forma idempotente:

1. Instala `curl` en el VPS si falta (arranque en frío).
2. Descarga el binario `devlab-install` desde la release `latest` y verifica su
   checksum.
3. Ejecuta el binario con perfil `vps`: paquetes del sistema, chezmoi + dotfiles,
   rustup/uv/cargo, binarios (helix, herdr, etc.), infra (AWS/Terraform/sops/GCP)
   y el agente (Pi + gentle-ai).

El resultado es un VPS con el entorno devlab instalado. El acceso por clave SSH
(para no volver a teclear la password) lo configura el **cliente** al enlazarse,
en su propio flujo (`install.sh` → opción dc1).

### VPS actual

| Campo | Valor |
|-------|-------|
| IPv4 | `149.56.110.211` |
| Usuario | `debian` |
| SO | Debian 13 |
| Región | Canadá (BHS) |
