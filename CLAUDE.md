# Dynamic Island

Isla dinámica para escritorio Windows: ventana Tauri transparente, sin bordes,
anclada al borde superior y por encima del resto de ventanas. Lee y controla el
reproductor del sistema vía SMTC, el volumen maestro vía Core Audio, y muestra
letras sincronizadas de LRCLIB.

El `README.md` documenta el proyecto para quien lo usa. Este archivo recoge lo
que hace falta para trabajar en el código.

## Idioma

**Todo el proyecto está en español**: comentarios, documentación, textos de la
interfaz y mensajes de commit. Mantenlo así.

Los comentarios explican *por qué*, no *qué*. Buena parte del código lleva
detrás una peculiaridad de Windows o de un reproductor concreto, y ese es el
contexto que no se puede reconstruir leyendo la línea.

## Comandos

```powershell
npm run dev          # Vite en el puerto 1420 (strictPort), demo web
npm run build        # vue-tsc -b && vite build
npm run tauri:dev    # App de escritorio
npm run tauri:build  # Bundle
```

Para comprobar tipos sin compilar: `npx vue-tsc --noEmit -p tsconfig.app.json`.

**La compilación de Rust requiere la toolchain de Windows** (Rust, MSVC Build
Tools, WebView2). Si el Control de aplicaciones inteligente está activo,
`cargo build` falla con `os error 4551`; el README explica el diagnóstico y las
alternativas. Rollup está sustituido por `@rollup/wasm-node` por esa misma clase
de restricción: no lo cambies por el Rollup nativo.

## Arquitectura

### Tres puntos de entrada, una base de código

| HTML | Monta | Qué es |
| --- | --- | --- |
| `index.html` → `src/main.ts` | `src/App.vue` | Demo web: escritorio Windows simulado con todas las actividades. |
| `island.html` → `src/island.ts` | `src/IslandApp.vue` | La isla real que carga la ventana `main` de Tauri. |
| `settings.html` → `src/settings.ts` | `src/SettingsApp.vue` | Panel de ajustes, en su propia ventana. |

Los tres entran como `rollupOptions.input` en `vite.config.ts`. `App.vue` e
`IslandApp.vue` comparten la píldora a través de
`src/components/TheIsland.vue`; añadir una actividad o un control casi siempre
significa tocar ese componente, no los dos contenedores.

Solo `island.ts` registra `MotionPlugin`.

### Contrato frontend ↔ Rust

Los comandos se registran en `generate_handler!` de `src-tauri/src/lib.rs`.
Añadir uno exige tocar ese array, o la llamada falla en tiempo de ejecución sin
error de compilación.

| Grupo | Comandos | Módulo Rust |
| --- | --- | --- |
| Multimedia | `media_state`, `media_play_pause`, `media_next`, `media_previous`, `media_seek` | `smtc.rs` |
| Ventana | `island_resize`, `island_set_interactive`, `island_cursor_over` | `lib.rs` |
| Ajustes | `settings_get`, `settings_set`, `settings_close` | `settings.rs` |
| Volumen | `volume_get`, `volume_set`, `volume_toggle_mute` | `volume.rs` |
| Letras | `lyrics_current` | `lyrics.rs` |
| Alimentación | `power_state` | `power.rs` |

Eventos que emite el backend:

- `smtc://media-changed` — estado multimedia; lo escucha `src/lib/media.ts`.
- `settings://changed` — ajustes nuevos; lo escucha `IslandApp.vue`.
- `power://changed` — el cargador se conectó o se retiró; lo escucha
  `src/lib/power.ts`. Solo se emite al cambiar el enchufe, no por el
  porcentaje, y respeta el ajuste `notifications`.

Las structs de Rust serializan en **camelCase** (`#[serde(rename_all =
"camelCase")]`). Las interfaces de TypeScript que las reflejan —`MediaState` en
`media.ts`, los ajustes en `SettingsApp.vue`— deben cambiarse a la vez que la
struct: no hay generación de tipos, la correspondencia es manual.

### Detección del entorno

`isDesktopApp()` en `src/lib/media.ts` comprueba `__TAURI_INTERNALS__` en
`window`. La API de Tauri se importa con `await import()` **solo** dentro de esa
rama, para que la compilación web no la incluya. Mantén ese patrón en cualquier
código nuevo que hable con el backend: un import estático de
`@tauri-apps/api` rompe la demo del navegador.

Sin Tauri, la música cae a `fallbackState` («Midnight City») y los controles son
visuales.

## Lo que no es obvio

**La ventana y el CSS se animan por separado.** La ventana nativa cambia de
tamaño de golpe; el CSS tarda medio segundo. Por eso al expandir la ventana
crece *antes* de la animación, y al contraer espera a que termine. La isla va
anclada al borde superior de la ventana, no centrada, porque la ventana se
encoge con retraso. El contenido entra solo con opacidad: un `translateY` se
percibe como un salto sobre una píldora que ya se redimensiona.

**No se usa `set_ignore_cursor_events`.** En Windows añade `WS_EX_TRANSPARENT`
sobre una ventana transparente, lo que además de dejar pasar los clics impide
que se dibuje. El comando `island_set_interactive` existe, pero la isla compacta
mide 104 × 32 px y por eso no hace falta.

**Los avisos no abren la isla, la ensanchan.** `src/lib/notifications.ts` es
una cola: un aviso interrumpe la píldora compacta, vive unos segundos y se
retira dejándola como estaba. Dos del mismo `kind` se reemplazan en el sitio en
vez de encolarse —al enchufar y desenchufar seguido interesa el estado final—,
y uno de otro tipo espera a que el visible cumpla su tiempo mínimo, porque
relevarlo antes lo dejaría a medio dibujar.

**Un aviso se dibuja de dos formas según el estado de la isla.** Cerrada,
ensancha la píldora. Abierta, entra como **franja** (`.notice-strip`) sin
cerrar el panel: el usuario lo abrió a propósito y puede estar usando el
volumen o la barra de progreso. La franja va fuera de `.island-content`,
porque ahí dentro `v-auto-animate` gestionaría su entrada y pelearía con su
propia animación. Y **ocupa sitio en lugar de superponerse**: colocada encima
tapaba justo la fila del título, y mover el contenido es preferible a
ocultarlo; de eso se encarga la regla `.island:has(.notice-strip)`.

El ancho sale de `noticeWidth()`, que lo **estima a partir del texto**: la
ventana nativa se redimensiona desde Rust antes de que el CSS pinte, así que no
se puede medir el elemento ya dibujado. `TheIsland.vue` llama a esa misma
función para el `style` de la píldora; si cada uno usara su propio número, la
píldora y su ventana se desincronizarían.

Cuidado con el `watch` de `targetSize`: la rama del aviso devuelve un **objeto
nuevo** en cada evaluación, mientras que las demás devuelven constantes
compartidas. Por eso compara por valor antes de redimensionar — sin ello, un
cambio de texto dispararía un `SetWindowPos` al mismo tamaño, que se ve
parpadear.

**La posición de SMTC se interpola.** `Position` se congela entre
actualizaciones, que llegan como mucho una vez por segundo. El backend le suma
el tiempo transcurrido desde `LastUpdatedTime` y sondea cada 250 ms. La carátula
está cacheada por pista para que ese sondeo no la redecodifique; se descartan
imágenes de más de 4 MB y se envían como data URI, no como archivo temporal.

**No todos los reproductores admiten saltar de posición.** SMTC expone
`IsPlaybackPositionEnabled`; **Apple Music lo declara `false`**, acepta la
llamada, devuelve `true` y no mueve nada — un salto grande llega a expulsarlo de
la sesión SMTC. Cuando `canSeek` es falso, los botones de salto quedan
deshabilitados y la barra no responde al arrastre.

**Las consultas a LRCLIB necesitan dos pasadas.** Apple Music informa el artista
como `Miranda! — El Templo del Pop`, con el álbum incluido: hay que recortar por
el separador antes de consultar. Y `/api/get` exige que la duración coincida,
cosa que falla con recopilatorios, así que se reintenta con `/api/search`, más
tolerante.

**Las letras son la única salida de red del proyecto.** El ajuste `lyrics` la
desactiva por completo; `lyrics_current` comprueba `settings::current().lyrics`
antes de consultar. Cualquier función nueva que envíe datos fuera del equipo
necesita su propio interruptor en los ajustes y una línea en el README.

Los avisos del cargador no salen del equipo, pero siguen el mismo patrón de
interruptor: el ajuste `notifications` se consulta **en cada vuelta** del
sondeo, no al montar el hilo, para que desactivarlos surta efecto en caliente.

## Persistencia

Los ajustes viven en `%APPDATA%\DynamicIsland\settings.json`, con la struct
`Settings` de `settings.rs` y `#[serde(default)]`, de modo que un campo nuevo no
invalida un archivo existente. Añade siempre el valor en `impl Default`.

El inicio automático se escribe en la clave `Run` del usuario actual, que no
requiere permisos de administrador.

`settings::CURRENT` es un `Mutex<Option<Settings>>` global compartido entre la
bandeja, las ventanas y el sondeo. Los cambios se aplican en caliente y se
propagan a la isla con el evento `settings://changed`.

## Plataformas

`smtc.rs` y `volume.rs` dependen de la crate `windows`, declarada bajo
`[target."cfg(windows)".dependencies]`, y traen una implementación vacía para
otras plataformas. Si tocas esos módulos, conserva ambas ramas: el proyecto debe
seguir compilando fuera de Windows.

## Atribución

La referencia de interacción es [Skiper UI — Skiper 2](https://skiper-ui.com/v1/skiper2),
que exige atribución. La demo conserva un enlace visible: no lo quites.
