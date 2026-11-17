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

### Estilos: Tailwind primero

El diseño va con utilidades de Tailwind v4 en las plantillas; el CSS a mano es la
excepción y tiene que tener un motivo. Se queda en CSS lo que Tailwind no expresa
bien o no debe separarse de su contexto: `@keyframes`, `@property`, `::before`/
`::after` con máscaras, selectores `:has()` sobre la isla, y las medidas que Rust
replica (muescas, `island-morph`). Eso vive en `style.css` (isla) y en el `<style>`
de `IslandApp.vue` (muescas).

- El panel de ajustes tiene su propia hoja, `settings.css`: esa ventana no carga
  `style.css`, así que no hereda su tema ni sus variables.
- Un elemento no debe llevar dos utilidades que fijen la misma propiedad (por
  ejemplo dos `size-*` o dos `bg-*`): Tailwind las emite en un orden que no es el
  del atributo y gana una u otra. Compón cada variante con sus clases completas.
- Las clases tienen que aparecer **literales** en el archivo; Tailwind no ve
  `` `bg-${color}` ``. Por eso hay constantes de clases en el `<script>`.
- No añadas reglas globales sin capa del estilo `button { font: inherit }`: ganan
  a todas las utilidades (que van en una capa) y rompen tamaños de texto sin
  avisar. Preflight ya normaliza los botones.
- Hay clases sin estilos que son asas: `island-stage`, `track-progress` (la
  localiza `startScrub` con `querySelector`). No las borres por «vacías».

### Contrato frontend ↔ Rust

Los comandos se registran en `generate_handler!` de `src-tauri/src/lib.rs`.
Añadir uno exige tocar ese array, o la llamada falla en tiempo de ejecución sin
error de compilación.

| Grupo | Comandos | Módulo Rust |
| --- | --- | --- |
| Multimedia | `media_state`, `media_play_pause`, `media_next`, `media_previous`, `media_seek` | `smtc.rs` |
| Ventana | `island_resize`, `island_set_interactive` | `lib.rs` |
| Ajustes | `settings_get`, `settings_set`, `settings_close` | `settings.rs` |
| Volumen | `volume_get`, `volume_set`, `volume_toggle_mute` | `volume.rs` |
| Letras | `lyrics_current` | `lyrics.rs` |
| Alimentación | `power_state` | `power.rs` |
| Sistema | `system_stats` | `sysstats.rs` |
| Ventana | `monitors_list` | `lib.rs` |
| Tiempo | `weather_current`, `weather_search`, `weather_locate_here` | `weather.rs` |
| Red | `speedtest_run` | `speedtest.rs` |
| Notificaciones | `toast_open`, `toasts_access`, `toasts_open_privacy` | `toasts.rs` |

Eventos que emite el backend:

- `smtc://media-changed` — estado multimedia; lo escucha `src/lib/media.ts`.
- `settings://changed` — ajustes nuevos; lo escucha `IslandApp.vue`.
- `power://changed` — el cargador se conectó o se retiró; lo escucha
  `src/lib/power.ts`. Solo se emite al cambiar el enchufe, no por el
  porcentaje, y respeta el ajuste `notifications`.
- `power://low` — la batería cruzó el 20 % o el 10 % sin cargador. Se emite una
  vez por umbral y se rearma al enchufar o pasar del 25 %. Lo escucha
  `src/lib/systemEvents.ts`.
- `network://wifi` — el equipo se conectó a una red Wi-Fi o cambió de red
  (`WifiState` en `network.rs`, vía la API WLAN). Sondeo de 3 s; el estado al
  arrancar es la línea base y no avisa. Lo escucha `src/lib/systemEvents.ts`.
- `network://internet` — se perdió (o recuperó) la salida a internet, según
  `INetworkListManager` (`InternetState`). La pérdida exige dos lecturas
  seguidas; la recuperación se omite si se acaba de anunciar una red Wi-Fi.
  Interruptor `notifyInternet`.
- `bluetooth://changed` — un dispositivo Bluetooth se conectó o desconectó
  (`BluetoothChange` en `bluetooth.rs`). Compara cada 3 s la lista de dispositivos
  conectados (clásicos y LE); la primera lectura es la línea base y una lectura
  fallida se salta, para que Bluetooth apagado no cuente como «todo desconectado».
  Interruptor `notifyBluetooth`; lo escucha `src/lib/systemEvents.ts`.
- `usb://changed` — unidad extraíble conectada o retirada (`UsbChange` en
  `usb.rs`). Compara cada 2 s las letras de unidad con volumen montado; un lector
  de tarjetas vacío no cuenta hasta que lleva tarjeta. Los discos duros externos
  USB se ven como «fijos» en Windows y no se detectan. Interruptor `notifyUsb`.
- `keys://lock` — Bloq Mayús (`caps`) o Bloq Num (`num`) cambió (`keys.rs`). La
  lectura (`GetKeyState`) se hace **en el hilo principal** vía `run_on_main_thread`:
  desde un hilo sin ventanas el estado se queda viejo. Interruptor `notifyLockKeys`.
- `focus://changed` — No molestar se activó o apagó (`focus.rs`, con
  `SHQueryUserNotificationState` → `QUNS_QUIET_TIME`). **No verificado**: Windows no
  documenta una API para esto; si no distingue el modo en alguna versión, el aviso no
  llega. Interruptor `notifyFocus`.
- `brightness://changed` — brillo de la pantalla integrada (`brightness.rs`, vía
  `IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS` sobre `\\.\LCD`; verificado contra WMI).
  Sin panel integrado (sobremesa, monitor externo) no hace nada. Sondeo de 250 ms.
  A diferencia del volumen, se anuncia también con la isla abierta. Interruptor
  `notifyBrightness`.
- `privacy://changed` — qué aplicación usa el micrófono y la cámara (`PrivacyState` en `privacy.rs`), leído de
  `CapabilityAccessManager\ConsentStore` (una aplicación lo usa si tiene hora de inicio y su hora de fin es 0; una entrada
  abierta desde hace más de 12 h se ignora: es el resto de un cierre brusco). Solo nombres, nunca audio ni vídeo.
  Interruptor `notifyPrivacy`.
- `audio://levels` — cinco bandas del audio del sistema entre 0 y 1 (`audio.rs`: loopback WASAPI + FFT propia de 1024
  muestras, sin dependencias). **Solo si `audioVisualizer` está activo** (nace apagado: captura el audio del sistema
  aunque solo mida el nivel); apagado, el hilo suelta el dispositivo. Probado con un tono: 440 Hz enciende la banda
  200–600 Hz. `TheIsland.vue` baja `animation` de las barras y fija su altura con `--wave-max`.
  **Cómo está hecho** (no lo cambies sin leer esto): ocho barras en escala logarítmica de 50 Hz a 16 kHz; cada barra mide la *energía* de su
  tramo (no el pico de un bin), así todas se mueven parecido; se normaliza al pico reciente (ganancia automática, `RANGE_DB`) con una puerta
  de ruido absoluta (`GATE_DB`, sin ella la ganancia amplificaría el silencio). Bloques de 2048 muestras con paso de 512 (~94/s), un aviso
  cada 16 ms (~60/s). **Los niveles no son estado reactivo**: van a `audioFeed` (audioFeed.ts) y cada `WaveBars.vue` los lee en su
  `requestAnimationFrame`, con suavizado propio (sube 26 ms, baja 105 ms) y `transform: scaleY`. Si pasaran por una prop de Vue, cada aviso
  repintaría `IslandApp` y la isla entera 60 veces por segundo.
- `downloads://progress` y `downloads://done` — descargas del navegador (`downloads.rs`). Vigila los temporales de Descargas
  (`.crdownload`, `.part`, `.opdownload`): tamaño, nombre y fechas, nunca contenido. **No hay porcentaje**: el navegador no lo
  publica. Un temporal que no crece en 4 s no cuenta. El nombre del archivo terminado es contenido privado: nunca a logs.
  Interruptor `notifyDownloads`.
- `speedtest://progress` y `speedtest://done` — test de velocidad (`speedtest.rs`), solo al pulsar el botón. Cloudflare
  rechaza (403) peticiones de bajada de más de 50 MB: se piden trozos de 25 MB hasta cumplir el tiempo. El resultado llega por
  evento y por el valor del comando; `applySpeedResult` (IslandApp.vue) lo aplica una sola vez, el que llegue primero.
  **Cloudflare limita por volumen (429)**: repetir la prueba varias veces seguidas deja de dar bajada durante un rato (las
  peticiones pequeñas, como la latencia, siguen funcionando). `SpeedResult.error` distingue `limit` de `network` para que el
  mensaje no diga «revisa tu conexión» cuando la conexión está bien, y tras un 429 no se vuelve a llamar en 3 minutos. Las
  peticiones son de 50 MB (el máximo) para pedir pocas, y la prueba dura unos 4–5 s por fase. No pruebes en bucle.
- `toast://received` — notificación nueva de otra aplicación (`AppToast` en
  `toasts.rs`). Solo se emite si `appNotifications` está activo y la app está
  activada en `notificationApps`. **Contenido privado: nunca a logs ni a disco.**
- `toasts://apps` — lista de aplicaciones detectadas, para el panel de ajustes.
  `toasts.rs` las añade a `notification_apps` (desactivadas) al verlas; lo
  escucha `SettingsApp.vue`.
- `volume://changed` — el volumen o el silencio cambiaron desde el sistema
  (`VolumeState` en `volume.rs`). Es un sondeo de 120 ms; la isla lo ignora con
  el panel abierto, que ya muestra su propio control.

Cada aviso del sistema (cargador, volumen, batería baja, Wi-Fi) tiene su
interruptor propio —`notifyCharger`, `notifyVolume`, `notifyLowBattery`,
`notifyWifi`, `notifyBluetooth`, `notifyUsb`, `notifyLockKeys`, `notifyFocus`,
`notifyBrightness`, `notifyTrack`— además de
`notifications`, el general. Un tipo nuevo necesita el suyo, y los dos se
consultan en cada vuelta de su sondeo para que apagarlos surta efecto en caliente.

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

**La ventana nativa tiene tamaño fijo y se recorta.** Mide siempre lo que cabe
la isla más grande (540 px con la letra). Redimensionarla obligaba a WebView2 a
actualizar su viewport unos fotogramas después y se veía el fotograma viejo (la
píldora duplicada). En su lugar `place_island` la recorta con `SetWindowRgn` a
la forma exacta de la isla: píldora redondeada más las dos muescas cóncavas.

**Bordes del estilo flotante: anillo opaco del color del escritorio.** Un píxel a medias (el que el CSS dibuja con antialiasing) dentro de la
ventana **no se mezcla con el escritorio sino con blanco**: medido en la app real, con un escritorio de 39 y una isla de 10, los píxeles del borde valían
89, 69 y 101. **Una prueba con fondo blanco no lo detecta** (blanco sobre blanco): comprueba siempre también sobre un fondo oscuro. La solución: el
recorte sube 1 px (`g` en `set_island_region`) y la isla lleva un anillo de 2 px **opaco** (`box-shadow: 0 0 0 2px var(--island-ring)`,
`.island-wrap.collapsed-round`). El suavizado se hace entre la isla y el anillo, dos colores opacos, y el borde duro del recorte cae sobre el anillo, que
es del color del escritorio y no se ve. Ese color lo muestrea Rust (`sample_backdrop`, cuatro puntos a 6 px de la isla, unas dos veces por segundo, solo con
la isla quieta) y llega por `island://backdrop`. Medido después: sobre escritorio oscuro ningún píxel del borde pasa del fondo; sobre blanco, píxeles
intermedios (118, 204, 64) entre la isla y el blanco, y fuera 255 limpio. Sustituye a la sombra de caída en este estilo (el recorte ya la cortaba). Con
muescas o con la pestaña, pegadas al borde de la pantalla, no se puede (un píxel de más toca el borde o la juntura de la muesca): ahí el borde sigue
escalonado. **Los bordes con muescas no se pueden suavizar, solo ordenar.** `SetWindowRgn` recorta a píxeles enteros, sin antialiasing, y
el borde de la región además sale **pálido** si el píxel que lo toca es transparente (se comprobó agrandando el
recorte 6 px: aparecían líneas blancas). Por eso la región queda siempre un pelo *dentro* de la forma del CSS. Para que
el escalón sea lo más regular posible, `pill_region` / `pill_spans` (lib.rs, con pruebas) construyen la forma fila a
fila con la circunferencia exacta, **muescas incluidas**: un píxel entra solo si queda entero dentro. No vuelvas a
`CreateRoundRectRgn` (aproxima la elipse con tramos planos) ni a las holguras a ojo de las muescas. Se probó un aro negro de 2 px por dentro del borde para disimular el escalón y se descartó
(se veía como un marco feo): no lo repongas. El suavizado de verdad exigiría otra arquitectura de ventana
(capa con alfa por píxel); no es un ajuste.

**El recorte debe seguir a la píldora del CSS.** Windows pinta de pálido
cualquier píxel transparente dentro de la región, así que si el recorte cambia
de golpe asoma una forma vieja durante los 0,58 s del CSS. `animate_island`
(`lib.rs`) lo interpola en un hilo con la misma curva y duración que
`.island-morph`, y `island_resize` se avisa a la vez que arranca el CSS, al
crecer y al encoger. La duración no es fija: 0,58 s cuando abre o cierra el usuario, 0,40 s cuando un
aviso del sistema abre la isla y 0,46 s cuando se retira. `IslandApp.vue` la fija en la variable
CSS `--morph` (píldora y muescas) y la pasa a `island_resize` (`duration_ms`) en la misma llamada:
las dos deben coincidir, o el recorte asoma por detrás de la píldora. El recorte **no debe ir por delante** de la píldora (se
vería una franja pálida) y las medidas de muesca y radio replican las del CSS:
si cambias una, cambia la otra. Los ajustes de uno o dos píxeles de
`set_island_region` evitan puntos y líneas claras en las muescas; no los
simplifiques sin probar a la vista.

**La vista minimizada tiene dos estilos** (`collapsedStyle`: píldora o pestaña).
La pestaña mide 48 × 8 px y `Shape::for_size` elige la forma por la altura
(≤ 12 px pestaña, ≤ 40 píldora, más es panel). La pestaña usa una muesca de 5 px
y no la de 14, que colgaría por debajo de ella: esa medida está también en
`.island-wrap:has(.island.is-tab)` de `IslandApp.vue`, y las dos deben coincidir.

El contenido entra solo con opacidad: un `translateY` se percibe como un salto
sobre una píldora que ya se redimensiona.

**No se usa `set_ignore_cursor_events`.** En Windows añade `WS_EX_TRANSPARENT`
sobre una ventana transparente, lo que además de dejar pasar los clics impide
que se dibuje. El comando `island_set_interactive` existe, pero la isla compacta
mide 104 × 32 px y por eso no hace falta.

**Los avisos del sistema despliegan la isla.** `src/lib/notifications.ts` es
una cola: un aviso interrumpe lo que muestre la isla, vive unos segundos y se
retira dejándola como estaba. Dos del mismo `kind` se reemplazan en el sitio en
vez de encolarse —al enchufar y desenchufar seguido interesa el estado final—,
y uno de otro tipo espera a que el visible cumpla su tiempo mínimo, porque
relevarlo antes lo dejaría a medio dibujar.

Cerrada, la isla se abre al tamaño del panel (330 × 108) con un icono grande, un
título y un subtítulo: es el formato `data.deployed` de Wi-Fi, internet,
volumen, batería baja y cargador desconectado (`NoticeIcon.vue` dibuja el icono;
`level` añade una barra). El cargador conectado (`data.large`) y las
notificaciones de apps (`data.tall`) tienen su propio panel, del mismo tamaño que el expandido (330 × 108). Cada
formato necesita su rama en `targetSize` (`IslandApp.vue`) y su estado en
`TheIsland.vue`, o la ventana nativa no crece con él.

**El avatar de una notificación** es la foto del remitente, si no el icono de la app y si no la
inicial. El icono sale de `AppDisplayInfo.GetLogo` (apps empaquetadas) o, para las clásicas, de
`shell_icon.rs`. Ojo con el filtro `is_plausible`: los AUMID de apps sin identificador propio
se derivan de su ruta (`{ID de carpeta}\Fortinet\FortiClient\FortiTray.exe`) y **llevan barras
invertidas**; rechazarlas dejó sin icono a FortiClient. Solo se prohíbe `..`. Los avisos de
globo de un icono de bandeja llegan con un AUMID inventado,
`NotifyIconGeneratedAumid_<n>`: el número es una clave de
`HKCUControl PanelNotifyIconSettings`, y su `ExecutablePath` da el programa. Ni ese AUMID
ni los de ruta se pueden abrir con `toast_open` (no son entradas del menú Inicio).

El mensaje de una notificación ocupa hasta **tres líneas** (`ToastMarquee.vue`) y, si pasa de ahí,
**sube** en lugar de crecer o cortarse con «…»: la isla y la ventana nativa son de tamaño fijo. La
distancia se mide sobre el texto dibujado, pero cuánto debe durar el aviso se **estima** por
caracteres (`toastDuration` en `IslandApp.vue`), porque el temporizador vive en la cola, antes de que
haya nada dibujado.

`island_resize` reafirma la ventana por encima de las demás (`bring_to_front`,
sin activarla) en cada cambio de tamaño: `always_on_top` se aplica una vez, y si
otra ventana «siempre visible» pasa por delante, un aviso saldría tapado.

**Un aviso ocupa la isla entera, esté como esté.** Cerrada, crece al tamaño de su
panel. Abierta conserva su tamaño (con o sin letra) y el panel del aviso
sustituye al contenido —`.island-content` se desmonta mientras dure— para
devolverlo al retirarse. Antes entraba como franja encima del panel y apretaba
el reproductor; se descartó porque tapaba o comprimía justo lo que el usuario
estaba usando. Con un aviso en pantalla, el primer toque lo descarta, esté la
isla abierta o cerrada. El volumen es la excepción: con la isla abierta se
ignora, porque su panel ya tiene el deslizador.

Si un aviso no trae panel propio, la píldora se ensancha: el ancho sale de `noticeWidth()`, que lo **estima a partir del texto**: la
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

**YouTube Music (y los reproductores web en general) tiene tres peculiaridades.**
Publica el título antes que la miniatura, y durante un instante esta sigue
siendo la de la canción anterior: por eso `cached_artwork` relee la carátula
unas veces al principio de cada pista (`ARTWORK_SETTLE_READS`) en vez de
guardar la primera, y el sondeo compara la *longitud* de la carátula para
enterarse de que llegó. El artista llega como «Nombre - Topic» o «NombreVEVO» y
el título con «(Official Video)» o «[Lyrics]»: `tidy_metadata` lo limpia en
`read_state`, de modo que la isla, la clave de la carátula y la búsqueda de
letras ven todos el mismo texto (tiene pruebas: `cargo test --lib`). Y la sesión
«actual» de Windows es la última con la que se interactuó, no la que suena:
`current_session` prefiere la que está reproduciendo.

**Abrir al pasar el ratón** (`hoverOpen`, en `IslandApp.vue`): un retardo de 140 ms al
entrar y de 320 ms al salir evita aperturas al cruzarla de paso y parpadeos. Una isla abierta
así se recoge sola, pero un clic encima la «fija» (`openedByHover`). Con un aviso en pantalla
no se abre. Si el ratón sale con un botón pulsado (arrastrando la barra o el volumen) se espera
a que lo suelte. Depende de que WebView2 reciba `mouseleave` al salir de la región recortada;
si alguna vez no llegara, el clic sigue cerrándola.

**El cambio de pista no es un salto.** La carátula (`ArtSwap.vue`, en la píldora y en el panel) gira, se
desenfoca y se desvanece mientras la nueva entra girando al revés, unidas por un destello; el
título y el artista se desenfocan en el sitio (`txt-swap`), y el fondo tintado y las muescas se
funden (`art-fade`, con animación y no con transición porque su opacidad final depende de si
la isla está abierta). Todo en `style.css`, sección «Cambio de pista». Las muescas deben fundirse
con el mismo tiempo que el fondo, o saltarían de color mientras el resto se funde.

**La fila de arriba del reproductor tiene el orden y el sitio fijos:** letra, volumen y la onda. El botón de letra
no desaparece sin letra, queda atenuado y deshabilitado (`lyricsAvailable`), para que nada se mueva al
cambiar de pista. El deslizador de volumen flota a la izquierda de su botón (absoluto) y no empuja a
nadie. No se distingue YouTube Music de un vídeo de YouTube: donde LRCLIB no tiene letra, queda bloqueado.

**Las esquinas de la carátula son concéntricas con las de la isla:** radio de la carátula = radio de la isla (18) − margen.
Reproductor abierto: margen 13 → carátula de 34 px con radio 5. Canción presentada (54 px de alto): margen 10 → radio 8. (Antes la isla
llevaba 24 y las carátulas 11 y 14: se veía demasiado redondo y la curva de la isla acababa dentro del alto de la carátula.)
Si cambias el margen o el radio de la isla, recalcula el de la carátula.

**El selector de ciudad** (ajustes → vista de inicio) busca con `weather_search` (Open-Meteo, solo con el ajuste `weather` activo) y guarda `weatherLat`/`weatherLon`; escribir a mano los borra y se vuelve a buscar por nombre. «Usar mi ubicación» (`weather_locate_here`) usa el `Geolocator` de Windows, no la IP; el error `denied` significa permiso de ubicación apagado en Windows. El formato de la hora es un selector sobre el booleano `homeSeconds`.

**Tiempo (`weather.rs`) y test de velocidad (`speedtest.rs`) salen a la red** y cada uno tiene su interruptor (`weather`, apagado por
defecto; `speedtest`, activo pero solo mide al pulsar). El tiempo envía a Open-Meteo solo la ciudad que el usuario escribió (para
buscar coordenadas) y esas coordenadas; los servidores van fijos en el código y la ciudad es solo un parámetro de consulta. Sin
redirecciones y con tiempo límite, como el resto de peticiones.

**Pantalla elegida** (`monitor`: 0 es la principal; n ≥ 1, la n-ésima que informe Windows). `chosen_monitor` (lib.rs) vuelve a la principal
si esa pantalla ya no existe. `monitors_list` rellena el selector de ajustes.

**La interfaz no lleva tooltips nativos** (atributo `title`): ni en la isla ni en el panel. Se quitaron todos a petición expresa; para los lectores de
pantalla basta el `aria-label`. No los repongas, ni uno suelto (el primero en salir fue «Abrir la isla»).

**La posición de la isla no es un ajuste:** siempre centrada y pegada al borde superior. `position` y `top_margin` siguen en `Settings`
(para leer un `settings.json` antiguo) pero `pinned()` (settings.rs) los fuerza a `Center` y `0` al leer y al guardar. No añadas un selector.

**El panel de ajustes va en pestañas** (`TABS` en `SettingsApp.vue`: Isla, Inicio, Música, Avisos, Apps), con la cabecera y la barra de
pestañas fijas (`sticky`). Cada pestaña es un `v-show`, no un `v-if`: los paneles siguen montados y el estado (lista de pantallas,
aplicaciones detectadas) no se pierde al cambiar. Un ajuste nuevo va en la pestaña de lo que toca.

**Los esqueletos de carga repiten la estructura del contenido real** (mismas clases de texto, mismas anchuras de columna, un espacio duro
para que la línea mida lo mismo): cada bloque cae donde luego caerá el texto y al llegar los datos nada se mueve. Se midió: filas,
barras y pie coinciden al píxel. Si cambias el contenido de una página, cambia también su esqueleto (`.skeleton`, style.css).

**La isla abierta es un carrusel de páginas** (`homePage`: `system`, `music`, `clock`, `speed`, en ese orden y dando la vuelta). El reproductor es una
página más y solo entra en la rueda si hay algo en reproducción (`hasMedia`); `carousel` activa todo esto en la ventana real y la demo web
lo deja apagado (ahí manda `mode`, una actividad por vez). Dos indicadores rotan entre páginas (`rotateHome` en `TheIsland.vue`) y **van en el mismo sitio en todas las páginas**: los extremos de la
fila de abajo, que es donde caen en el reproductor (la fila de controles, que es donde sobra sitio: a media altura chocarían con los tiempos).
Una sola definición, `NAV_ARROW`; medidas: izquierda (13, 67), derecha (293, 67), 24 × 28 px, relativas a la isla. Si cambias una, cambia todas. **El reproductor es lo principal mientras suena algo:** la isla se abre en él (si no suena nada,
en la hora), y si se rota a otra página y se deja sin tocar 15 s vuelve sola (`armBackToPlayer`, IslandApp.vue; cada movimiento del puntero
reinicia la cuenta, y no corre durante un test de velocidad). Al salir del reproductor se cierra la letra, o la página siguiente se quedaría a
540 px. CPU y RAM (ajuste `homeStats`) solo se miden con su página a la vista. El test de velocidad (ajuste `speedtest`) guarda su estado en
`IslandApp.vue`, no en la página. Para añadir una página: ampliar `HomePage`, `homePages` y su `<template v-else-if>`; los indicadores la recogen solos.

**La isla tiene dos estilos, y valen para cerrada y abierta a la vez** (`islandStyle`: `notch` o `round`, ajuste «Estilo de la isla»; antes
`expandedStyle`, que solo valía para la abierta: queda un `alias` en `settings.rs` para leer archivos antiguos). Son dos «packs»: **con muescas**
(cuelga del borde, esquinas invertidas) y **flotante redondeada** (sin muescas, esquinas curvas, 6 px por debajo del borde). Mismo tamaño en los
dos; solo cambian los bordes. `round` quita las muescas (`--notch: 0` en `.island-wrap.expanded-round` y `.island-wrap.collapsed-round`, que se ponen
juntas) y curva las esquinas de arriba: 18 px la abierta, 17 la píldora cerrada (media altura). La pestaña mínima solo pierde las muescas y sigue
pegada. **Rust lo replica**: `Shape` lleva `top_radius` y `top_offset`; `for_size` pone notch 0 y el radio que toque según la altura (más de 40 px:
abierta; más de 12: píldora; el resto: pestaña); `set_island_region` quita a cada esquina lo que queda fuera del cuarto de círculo. Si cambias una
medida, cambia la otra, o Windows pinta de pálido lo que sobra. La separación de 6 px (`margin-top` en CSS y `ROUND_GAP` en `lib.rs`, que tienen que
coincidir) queda fuera del recorte (Windows pintaría de pálido un píxel transparente dentro de la región), así que la página no ve el ratón sobre
ella: `start_gap_watch` (lib.rs) vigila el cursor cada 40 ms y emite `island://gap`; con el ratón ahí, la isla abierta por hover no se recoge y la
cerrada se abre, porque la franja es parte de la isla.

**El movimiento sigue unos principios** (sección «Movimiento» de `style.css`): una sola curva, la de las hojas de
iOS `(.32,.72,0,1)`, sin rebote; entrar tarda más que salir; lo que entra va escalonado (carátula o icono
primero, texto después) y lo que sale se va entero antes de que la isla se cierre (`notice-panel`,
`content`). Solo opacidad, desenfoque y escala, nunca desplazamiento. Ojo: una animación con relleno (`animation: … both`, como la de entrada de `.island-content`) pisa a
cualquier transición de las mismas propiedades; por eso la salida de `.island-content` y de
`.island-collapsed` es otra animación (`content-out`) y no una transición. Si añades un panel de aviso,
envuélvelo en `<Transition name="notice-panel">` y marca su parte izquierda `np-lead` y su texto `np-copy`.

**La canción presentada mide la mitad del panel** (330 × 54 px, `showsTrackBar` en `TheIsland.vue`,
`TRACK` en `IslandApp.vue`) con relleno, carátula (38 px) y texto reajustados. Rust la trata como panel
(más de 40 px de alto: muesca 22, radio 18), así que no necesita una forma propia.

**La canción se presenta sola** (`notifyTrack`, tipo de aviso `track`, en `IslandApp.vue`) al
empezar a sonar o al cambiar de pista, venga de donde venga: todo llega por SMTC. La primera
lectura es la línea base, con la isla abierta no se presenta, y se espera 700 ms porque SMTC
pasa por estados intermedios (título nuevo con la carátula vieja). Si la carátula llega después,
el aviso se actualiza en el sitio. El panel desplegado admite `data.art` en lugar del icono.

**La vista de inicio mide CPU y memoria** (`homeStats`, `system_stats`) solo mientras está
abierta y a la vista: `IslandApp.vue` consulta cada 1,5 s y descarta la primera lectura,
porque el uso de CPU se calcula entre dos consultas seguidas.

**Perder los medios tiene 3 s de margen** (`hasMedia` sobre `mediaPresent`, `MEDIA_LOSS_GRACE_MS`, IslandApp.vue): al cambiar de pista SMTC pasa un instante sin sesión o con sesión vacía (según equipo y reproductor), y sin margen la isla saltaba a la hora. Ganarlos es inmediato.

**Sin medios, la isla muestra la vista de inicio (`home`).** `displayMode` en `IslandApp.vue` cambia
`music` por `home` cuando no hay sesión SMTC, o hay una vacía y parada; una pista en pausa
cuenta como medio. `mode` no cambia, así que al sonar algo la isla vuelve sola al reproductor.
`home` no se elige a mano y no está en la lista de actividades de la demo.

**Un comando que espera algo no puede correr en el hilo principal.** En Tauri
los comandos sin `async` se ejecutan en el hilo que dibuja la ventana: si se
queda esperando, la isla se congela y Windows la marca «no responde», cosa que
en la máquina de desarrollo no se nota y en otros equipos sí. Por eso los
comandos que hacen red (`lyrics_current`), WinRT (`media_*`) o COM (`volume_*`)
llevan `#[tauri::command(async)]`, que los pasa a un hilo aparte. Y toda
petición HTTP necesita límite de tiempo: el cliente de `lyrics.rs` tiene 4 s
para conectar y 8 s en total, y tras un fallo de red no insiste durante un
minuto ni guarda «sin letra» como si fuera el resultado de la pista.

**Las consultas a LRCLIB necesitan dos pasadas.** Apple Music informa el artista
como `Miranda! — El Templo del Pop`, con el álbum incluido: hay que recortar por
el separador antes de consultar. Y `/api/get` exige que la duración coincida,
cosa que falla con recopilatorios, así que se reintenta con `/api/search`, más
tolerante.

**Las letras, la foto del remitente, el tiempo y el test de velocidad son las únicas salidas de red del proyecto.**
El ajuste `lyrics` desactiva la primera por completo; `lyrics_current` comprueba
`settings::current().lyrics` antes de consultar. Cualquier función nueva que
envíe datos fuera del equipo necesita su propio interruptor en los ajustes y una
línea en el README.

**La foto del remitente (`avatars.rs`) lee contenido privado y sale a la red**,
y se apoya en algo no documentado: la base `wpndatabase.db` de Windows. Reglas
que no se deben relajar: no tiene interruptor propio (se quitó a propósito),
así que el permiso es activar la app en `notificationApps` y el panel de ajustes
debe seguir avisando de que se descarga la foto; solo para apps que el usuario
activó; base abierta **solo para lectura** y con la ruta con barras normales
(con barras invertidas SQLite no la abre); descarga únicamente de
`ALLOWED_HOSTS`, sin redirecciones, con tope de tamaño y tipo decidido por el
contenido; nada a disco ni a logs. El `Id` del listener coincide con la columna
`Id` de `Notification` (verificado). La hora de los archivos de `wpnidm` no sirve
para emparejar: es la del servidor. Para añadir otra app se amplía la lista de
hosts y se prueba; no se abre a cualquier dirección.

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
