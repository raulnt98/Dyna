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
`notifyWifi`— además de
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

**El recorte debe seguir a la píldora del CSS.** Windows pinta de pálido
cualquier píxel transparente dentro de la región, así que si el recorte cambia
de golpe asoma una forma vieja durante los 0,58 s del CSS. `animate_island`
(`lib.rs`) lo interpola en un hilo con la misma curva y duración que
`.island-morph`, y `island_resize` se avisa a la vez que arranca el CSS, al
crecer y al encoger. El recorte **no debe ir por delante** de la píldora (se
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

**Las letras y la foto del remitente son las únicas salidas de red del proyecto.**
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
