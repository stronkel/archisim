# ArchiSim en el NAS

La versión web de ArchiSim (`docs/IPAD.md`) servida desde el NAS de casa, con usuario y contraseña. Se pone al día sola: cada 10 minutos mira si hay una versión nueva publicada en GitHub y, si la hay, la descarga.

## Instalarlo (una vez)

1. **Copia esta carpeta al NAS**, por ejemplo a `docker/archisim` (con el Explorador de archivos de UGOS Pro o por red).
2. **La contraseña.** Abre Terminal en el NAS (o entra por SSH) y escribe, con tu contraseña entre las comillas:

   ```
   docker run --rm caddy:2-alpine caddy hash-password --plaintext 'tu-contraseña'
   ```

   Sale una línea que empieza por `$2a$14$…`: es la contraseña cifrada.
3. **Copia `ejemplo.env` como `.env`** en la misma carpeta y cambia los dos valores: el usuario y la contraseña cifrada (entre comillas simples).
4. **Arráncalo.** En Docker de UGOS Pro: *Proyecto* → *Crear*, elige esta carpeta (usa `docker-compose.yml`) y *Desplegar*. O en Terminal, dentro de la carpeta: `docker compose up -d`.
5. **Pruébalo en casa**: `http://<ip-del-nas>:4200` (la del NAS, la misma con la que entras a UGOS). Pide usuario y contraseña y abre ArchiSim con la casa de ejemplo. La primera vez tarda un par de minutos en llegar la versión (`docker compose logs actualizador` dice cuándo está lista).

## Desde el iPad y fuera de casa: Tailscale Funnel

Safari solo deja usar la gráfica a una página con **HTTPS**, y la dirección `http://<ip>:4200` no lo es. Tailscale lo pone, como con Ensembli:

```
tailscale funnel --bg --https=8443 4200
```

Si Ensembli ya está en el puerto 443 de Funnel, este va en el 8443 y no se pisan. La dirección es la que dice `tailscale funnel status`, algo como `https://nas.tu-red.ts.net:8443`.

Si prefieres que solo entren tus dispositivos de Tailscale (sin salir a Internet), usa `serve` en lugar de `funnel`: `tailscale serve --bg --https=8443 4200`.

## En el iPad

1. Abre la dirección en **Safari**, escribe el usuario y la contraseña y deja que Safari los guarde.
2. **Compartir → Añadir a pantalla de inicio**: ArchiSim queda como una app más, a pantalla completa.
3. Hace falta iPadOS 26 o posterior (Safari 26 trae WebGPU).

## Mantenimiento

- **Versiones nuevas**: llegan solas. Recarga la página.
- **Cambiar la contraseña**: repite el paso 2, cambia `.env` y reinicia el proyecto (`docker compose up -d`).
- **Volver a la versión anterior**: la anterior queda en `web/anterior`; cambia los nombres de las carpetas `actual` y `anterior`.
