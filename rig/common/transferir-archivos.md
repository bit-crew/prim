# Pasar archivos al VPS — croc (puntual) y rclone mount (flujo)

Dos formas complementarias de llevar archivos de tu máquina al VPS sin
abrir FTP ni exponer puertos: **croc** para mandadas puntuales y **rclone
mount** para que el VPS sea una carpeta más de tu computadora (descargas
de Telegram/correo caen directo donde quieras).

## croc — mandar un archivo ya (ambos lados necesitan croc)

En tu máquina (instalalo desde https://croc.science o el release de
GitHub):

```
croc send informe.pdf          # te da un codigo-frase
```

En el VPS (ya instalado):

```
croc           # pega el codigo-frase cuando lo pida
```

Encriptado punto a punto; el archivo no pasa por almacenamiento ajeno.
Para recibir en una ruta concreta: `cd ~/work/knt && croc`.

## rclone mount — el VPS como carpeta local (configuracion unica)

Monta el VPS por SFTP como unidad/carpeta. Una vez montado, "Guardar
como..." del navegador o de Telegram apuntando a esa carpeta deja el
archivo directamente en el VPS, en la ruta que elijas.

1. Instala rclone (https://rclone.io/downloads) y el soporte de montaje:
   - **Windows**: instala WinFsp (https://winfsp.dev) — rclone basta aparte.
   - **macOS**: instala macFUSE (https://osxfuse.github.io).
   - **Linux**: nada extra.
2. Configura el acceso (unica vez):

   ```
   rclone config
   #   n  (nuevo remoto)
   #   nombre: vps
   #   tipo: sftp
   #   host: TU-IP-DEL-VPS
   #   user: debian
   #   todo lo demas: Enter (usa tu clave SSH habitual)
   ```

3. Montalo:
   - Windows: `rclone mount vps: V: --vfs-cache-mode writes` (deja la
     ventana abierta; aparece el disco V:)
   - macOS/Linux: `mkdir -p ~/VPS && rclone mount vps: ~/VPS --vfs-cache-mode writes`

4. Listo: descarga de Telegram/correo → "Guardar como" → `V:\work\knt\...`
   (o `~/VPS/work/knt/...`) → el archivo aparece en el VPS al instante.

El `--vfs-cache-mode writes` hace que las descargas grandes del navegador
suban confiables (escribe local y sincroniza en segundo plano).

## Cuál usar cuándo

- Un archivo ahora, de cualquier origen → **croc**.
- Descargas frecuentes (Telegram, correo, navegador) → **rclone mount**.
- Subir al VPS lo que ya bajaste localmente hace rato → arrastra al disco
  V: montado (es lo mismo que rclone, sin pasos extra).
