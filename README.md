# Dynamic Island para Windows

Una isla dinámica para el escritorio de Windows, construida con Vue 3 y Tauri 2.
Cuelga del borde superior de la pantalla, por encima del resto de ventanas, y
muestra lo que suena en el sistema, letras sincronizadas, avisos del equipo
(cargador, volumen, batería, Wi-Fi e internet) y notificaciones de otras
aplicaciones, con la foto de quien te escribe en WhatsApp. Sin nada sonando,
enseña la hora y la fecha. El repositorio incluye también una demo web que
simula un escritorio Windows con las actividades.

> **Estado del proyecto:** la aplicación de escritorio funciona como isla
> flotante real: ventana transparente y sin bordes, anclada al borde superior y
> por encima del resto de ventanas, con la música del sistema vía
> [SMTC](#música-del-sistema-smtc), control de volumen, [avisos del
> sistema](#avisos-del-sistema), [notificaciones de otras
> aplicaciones](#notificaciones-de-otras-aplicaciones), icono en la bandeja y
> [panel de ajustes](#panel-de-ajustes). La demo web del navegador se mantiene
> como vista previa interactiva.

## Descarga

La última versión está en [**Releases**](https://github.com/raulnt98/Dyna/releases/latest):
descarga `Dynamic.Island_<versión>_x64-setup.exe` y ejecútalo.

- Se instala **solo para tu usuario**, sin permisos de administrador.
- Requiere Windows 10 u 11 (64 bits) con WebView2, que ya viene en Windows 11.
- **El instalador no está firmado**, así que Windows mostrará «SmartScreen
  protegió su PC». Pulsa **Más información** y luego **Ejecutar de todas
  formas**. Con *Smart App Control* activado, Windows puede bloquearlo del todo.
- Cada release incluye la suma SHA-256 del instalador para comprobar la descarga.

Para compilarlo tú mismo, mira [Aplicación de escritorio con
Tauri](#aplicación-de-escritorio-con-tauri); `npm run tauri:build` genera el
instalador en `src-tauri/target/release/bundle/nsis/`.

### Novedades de la 0.3.1

- **Iconos en las notificaciones de apps:** el avatar es ahora el icono de la
  aplicación cuando no hay foto del remitente. Funciona con las apps de la Store,
  con las clásicas (Outlook de escritorio, antivirus…) y con los avisos de globo de
  un icono de la bandeja (FortiClient, clientes VPN). Sin red.
- **Diagnóstico del permiso de notificaciones:** si Windows no deja que la isla
  lea las notificaciones, Ajustes muestra un aviso naranja con un botón **Abrir
  Windows**. Antes la opción parecía activa y la isla se quedaba muda sin explicar
  por qué.
- **Segundos en la hora** de la vista de inicio, con un ajuste para quitarlos.
- **Arreglo:** «Ajustes…» no hacía nada si la ventana de ajustes estaba
  minimizada.
- **Documentación:** cómo usar solo la isla, sin el globo de Windows. Ver
  [Usar solo la isla](#usar-solo-la-isla-sin-el-globo-de-windows).

### Novedades de la 0.3.0

- **Avisos desplegados:** cargador, volumen, batería baja, Wi-Fi e internet abren la
  isla al tamaño del panel, con icono grande, título y subtítulo, también con la
  isla abierta. Ver [Avisos del sistema](#avisos-del-sistema).
- **Aviso de Wi-Fi y de internet** nuevos, con su animación y su interruptor.
- **Vista de inicio:** sin medios, la isla enseña la hora y la fecha en lugar de un
  reproductor vacío. Ver [Vista de inicio](#vista-de-inicio).
- **Notificaciones:** foto del remitente de WhatsApp, mensajes largos que se
  se desplazan y un clic que abre la app. Ver [Notificaciones de otras
  aplicaciones](#notificaciones-de-otras-aplicaciones).
- **Animaciones en todos los equipos:** la isla ya no se queda sin animar cuando
  Windows tiene desactivados los efectos de animación.
- **Interfaz:** el panel de ajustes, la demo web y la isla migran a Tailwind CSS.
  El aspecto no cambia.
- El aviso del volumen ya no repite su animación con cada pulsación.

## Contenido

- [Descarga](#descarga)
- [Requisitos](#requisitos)
- [Instalación y ejecución](#instalación-y-ejecución)
- [Tecnologías](#tecnologías)
- [Estructura del proyecto](#estructura-del-proyecto)
- [Uso de la demo](#uso-de-la-demo)
- [Estados de la isla](#estados-de-la-isla)
- [Animaciones](#animaciones)
- [Componentes y estilo](#componentes-y-estilo)
- [Aplicación de escritorio con Tauri](#aplicación-de-escritorio-con-tauri)
- [Música del sistema (SMTC)](#música-del-sistema-smtc)
- [Isla flotante y panel de ajustes](#isla-flotante-y-panel-de-ajustes)
- [Avisos del sistema](#avisos-del-sistema)
- [Vista de inicio](#vista-de-inicio)
- [Notificaciones de otras aplicaciones](#notificaciones-de-otras-aplicaciones)
- [Limitaciones actuales](#limitaciones-actuales)
- [Referencia y atribución](#referencia-y-atribución)

## Requisitos

### Para la demo web

- Node.js compatible con Vite 7 (Node.js 20.19+ o 22.12+).
- npm.

### Para ejecutar o empaquetar Tauri en Windows

Además de Node.js y npm, instala los requisitos de desarrollo de Tauri para
Windows:

- Rust estable y Cargo, mediante `rustup`.
- Microsoft C++ Build Tools, con la carga de trabajo de desarrollo de escritorio
  con C++.
- Microsoft Edge WebView2 Runtime.

Consulta la [guía oficial de prerrequisitos de Tauri para Windows][tauri-prereq]
para instrucciones de instalación actualizadas.

`rusqlite` compila SQLite desde su código C (feature `bundled`) con esa misma
toolchain de MSVC: la primera compilación tarda algo más, pero la app no depende
de ninguna DLL de SQLite del sistema.

> **Control de aplicaciones inteligente.** Si Windows tiene activo el Control de
> aplicaciones inteligente (Smart App Control), `cargo build` falla con
> `Una directiva de Control de aplicaciones bloqueó este archivo (os error
> 4551)` al ejecutar los scripts de compilación de las dependencias: ningún
> binario recién compilado está firmado ni tiene reputación, que es lo que esa
> función exige. Se comprueba con
> `Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy" -Name VerifiedAndReputablePolicyState`
> (`1` es activo, `0` desactivado). Solo se desactiva desde Seguridad de Windows
> → Control de aplicaciones y navegador, y **es irreversible**: volver a
> activarlo requiere reinstalar Windows. Las alternativas sin tocar el equipo
> son compilar en una máquina virtual o en CI. El mismo tipo de restricción
> explica el uso de `@rollup/wasm-node` en la parte web.

## Instalación y ejecución

Desde la raíz del proyecto:

```powershell
npm install
npm run dev
```

Vite muestra la dirección local en la consola; la configuración actual usa el
puerto `1420` y requiere que esté disponible (`strictPort`). También se puede
abrir directamente:

```text
http://localhost:1420/
```

Comprobar tipos y generar la compilación web de producción:

```powershell
npm run build
```

Servir localmente la compilación:

```powershell
npm run preview
```

Los comandos de Tauri, una vez instalados los requisitos de Rust:

```powershell
npm run tauri:dev
npm run tauri:build
```

## Tecnologías

- **Vue 3** con Composition API y `<script setup lang="ts">`.
- **TypeScript** y `vue-tsc` para validación de tipos.
- **Vite 7** como servidor de desarrollo y herramienta de compilación.
- **Tailwind CSS 4** mediante `@tailwindcss/vite`. El diseño va con utilidades en
  las plantillas; el CSS a mano queda solo para lo que Tailwind no expresa bien
  (animaciones, `@property`, pseudoelementos y las muescas de la isla). Ver
  [Componentes y estilo](#componentes-y-estilo).
- **Reka UI**, las primitivas accesibles utilizadas por los componentes de
  interfaz locales compatibles con shadcn-vue.
- **shadcn-vue** como convención de componentes: los componentes se mantienen
  como código fuente local en `src/components/ui/`.
- **@tabler/icons-vue** para la iconografía.
- **@formkit/auto-animate** para animar la inserción, eliminación y reordenación
  de elementos internos.
- **Tauri 2 + Rust** como contenedor de escritorio. Crates principales:
  `windows` (SMTC, Core Audio, WLAN, notificaciones, ventana recortada), `ureq`
  (letras y foto del remitente, con límites de tiempo) y `rusqlite` (lectura de
  solo lectura de la base de notificaciones de Windows, con SQLite incluido).

El proyecto sustituye Rollup por su distribución oficial WebAssembly,
`@rollup/wasm-node`, para que la compilación pueda ejecutarse en entornos Windows
donde las políticas de control de aplicaciones bloquean complementos nativos.

## Estructura del proyecto

```text
.
├── index.html                         # Demo web (monta App.vue)
├── island.html                        # La isla real de la ventana principal de Tauri
├── settings.html                      # Panel de ajustes, en su propia ventana
├── package.json                       # Scripts y dependencias npm
├── vite.config.ts                     # Plugins de Vue/Tailwind, alias, puerto y entradas
├── tsconfig.json                      # Referencias de TypeScript
├── tsconfig.app.json                  # Opciones de TypeScript para la aplicación
├── tsconfig.node.json                 # Opciones de TypeScript para Vite
├── components.json                    # Configuración de shadcn-vue
├── public/
│   └── logo.png                       # Logo y favicon
├── src/
│   ├── main.ts                        # Entrada de la demo web
│   ├── island.ts                      # Entrada de la isla real
│   ├── settings.ts                    # Entrada del panel de ajustes
│   ├── App.vue                        # Demo: escritorio simulado y actividades
│   ├── IslandApp.vue                  # La isla real: tamaño, avisos y letras
│   ├── SettingsApp.vue                # Panel de ajustes
│   ├── style.css                      # Tema de Tailwind y CSS de la isla (animaciones, muescas)
│   ├── settings.css                   # Tema de Tailwind del panel de ajustes (ventana aparte)
│   ├── lib/
│   │   ├── utils.ts                   # Utilidad cn() para clases
│   │   ├── activities.ts              # Actividades de la isla (incluye la vista de inicio)
│   │   ├── media.ts                   # Puente con la música del sistema (SMTC)
│   │   ├── lyrics.ts                  # Letras sincronizadas y línea activa
│   │   ├── clock.ts                   # Reloj local suave para la letra
│   │   ├── notifications.ts           # Cola de avisos de la isla
│   │   ├── power.ts                   # Estado del cargador
│   │   └── systemEvents.ts            # Volumen, batería, Wi-Fi, internet y notificaciones de apps
│   └── components/
│       ├── TheIsland.vue              # La píldora, compartida por demo e isla real
│       ├── NoticeIcon.vue             # Icono de cada tipo de aviso
│       ├── WifiGlyph.vue              # Icono de Wi-Fi con arcos que se encienden
│       ├── ToastMarquee.vue           # Mensaje de la notificación: hasta tres líneas, y si pasa, se desplaza
│       ├── SettingsSelect.vue         # Desplegable propio del panel de ajustes
│       ├── SettingsToggle.vue         # Fila con interruptor del panel de ajustes
│       └── ui/
│           ├── button/                # Botón local con variantes
│           ├── popover/               # Exportaciones de primitivas Reka UI
│           └── toggle-group/          # Exportaciones de primitivas Reka UI
└── src-tauri/
    ├── Cargo.toml                     # Paquete y dependencias Rust
    ├── build.rs                       # Integración de tauri-build
    ├── tauri.conf.json                # Ventana y comandos frontend de Tauri
    ├── capabilities/
    │   └── default.json               # Permisos de la ventana principal
    └── src/
        ├── lib.rs                     # Builder, comandos, ventana recortada y sondeos
        ├── smtc.rs                    # Lectura y control de SMTC en Windows
        ├── volume.rs                  # Volumen maestro (Core Audio)
        ├── power.rs                   # Batería y corriente
        ├── network.rs                 # Wi-Fi (API WLAN) y salida a internet (Network List Manager)
        ├── lyrics.rs                  # Letras desde LRCLIB
        ├── toasts.rs                  # Notificaciones de otras aplicaciones y apertura de su app
        ├── avatars.rs                 # Foto del remitente (WhatsApp), desde la base de Windows
        ├── shell_icon.rs              # Icono de una aplicación clásica, como en el menú Inicio
        ├── settings.rs                # Ajustes y persistencia
        └── main.rs                    # Entrada de la aplicación Windows
```

## Uso de la demo

1. Inicia el servidor con `npm run dev`.
2. Selecciona una actividad en el grupo de opciones a la izquierda.
3. La isla aparece en la parte superior de la vista del escritorio y muestra el
   contenido correspondiente.
4. Haz clic sobre la isla para expandirla o contraerla. Al estar expandida, los
   botones dentro de ella mantienen sus propias acciones y no la contraen.
5. Usa el botón de ajustes junto a la vista previa para abrir el selector de
   fondo. Las opciones disponibles son **Aurora**, **Atardecer** y **Azul**.

La vista previa representa elementos de escritorio de Windows —accesos directos,
barra de tareas, búsqueda y reloj—, pero son elementos visuales de la demo y no
controles del sistema operativo.

## Estados de la isla

| Estado | Vista compacta | Vista expandida e interacción |
| --- | --- | --- |
| **Inicio** | La hora | Hora y fecha. **No se elige:** la isla la muestra sola cuando no hay ningún medio. Ver [Vista de inicio](#vista-de-inicio). |
| **Inactiva** | Indicador tenue | Muestra que no hay actividades activas. |
| **Llamada entrante** | Indicador de llamada y nombre | Simula una llamada de Alex. Se puede rechazar o responder; responder cambia al estado de llamada. |
| **Temporizador** | Cuenta atrás | Temporizador de cocina que se puede pausar y reanudar. |
| **Grabación** | Punto de grabación y duración | Indicador visual de audio, forma de onda y control de pausa/reanudación. |
| **Música** | Carátula y visualizador | En la app de escritorio muestra la pista que suena en el sistema (SMTC) y sus controles funcionan de verdad. En el navegador muestra una pista de ejemplo. Ver [Música del sistema (SMTC)](#música-del-sistema-smtc). |
| **Transferencia** | Icono y porcentaje | Simula la recepción de `Presentación-final.pdf`, con progreso fijo de ejemplo. |
| **Llamada** | Indicador de llamada y duración | Simula una llamada activa. Permite silenciar, finalizarla y volver a llamar. |
| **Notificación** | Icono y contador de mensajes | Muestra un mensaje de Alex en la isla y como aviso de escritorio. Puede marcarse como leído o cerrarse; estas acciones vuelven a la actividad musical. |

El reloj de la barra de tareas usa la hora local del equipo. Las duraciones y
actividades son estado en memoria de la página: no se sincronizan con servicios,
dispositivos ni APIs multimedia del sistema.

## Animaciones

### Apertura y cierre de la isla flotante

La píldora cambia de tamaño en 0,58 s con la curva `cubic-bezier(.22, 1, .36, 1)`:
entra rápido y frena hasta pararse, **sin rebote**. Un muelle que sobrepasaba el
tamaño final se percibía, a este tamaño, como un temblor al final de cada
transición.

Coordinar eso con la ventana nativa tiene truco. La ventana **no cambia de
tamaño**: mide siempre lo que cabe la isla más grande (540 px con la letra) y
Rust la recorta con `SetWindowRgn` a la forma exacta de la isla, píldora
redondeada más las dos muescas cóncavas de las esquinas. Redimensionarla
obligaba a WebView2 a actualizar su viewport unos fotogramas después y se veía
el fotograma viejo.

- **El recorte sigue a la píldora.** Rust lo interpola en un hilo con la misma
  curva y duración que el CSS (`animate_island`) y se le avisa a la vez que
  arranca la animación, al crecer y al encoger. Si fuera por delante asomaría
  una franja pálida, porque Windows pinta de pálido cualquier píxel transparente
  dentro de la región.
- **Las medidas de muesca y radio replican las del CSS.** Si cambias una, cambia
  su pareja.
- El contenido entra solo con opacidad, sin desplazamiento: cualquier
  `translateY` se percibe como un salto sobre una píldora que ya se está
  redimensionando.

El panel de karaoke usa el mismo mecanismo: la isla se ensancha hasta 540 px y
los controles conservan su ancho mientras tanto, de modo que no se reacomodan
en cada fotograma.

**La isla no respeta `prefers-reduced-motion`, a propósito.** WebView2 lo activa
cuando Windows tiene desactivados los «Efectos de animación» (ahorro de energía,
máquinas virtuales, escritorio remoto), y la isla se quedaba sin animar en esos
equipos. Además el recorte nativo se anima siempre, y el CSS debe ir a la par.
El panel de ajustes sí lo respeta.



### Expansión y contracción

El tamaño y las esquinas de la isla cambian con la clase `island-morph`
(`transition-property: width, height, border-radius, box-shadow`) y las clases de
tamaño de `TheIsland.vue`. Estos son los tamaños, que `IslandApp.vue` replica
para la ventana nativa:

| Forma | Tamaño |
| --- | --- |
| Pestaña mínima | `48 × 8 px` |
| Píldora compacta | `116 × 34 px` (más ancha si el aviso lo pide) |
| Panel expandido, avisos desplegados y notificaciones de otras apps | `330 × 108 px` |
| Panel con letra | `540 × 108 px` |

La forma que recorta Rust se elige por la altura (≤ 12 px pestaña, ≤ 40 píldora,
más es panel), y la pestaña usa una muesca de 5 px en lugar de la de 14.

### AutoAnimate

La directiva `v-auto-animate`, importada de `@formkit/auto-animate/vue`, está
instalada en:

- `.island-content`, para animar los elementos inmediatos que cambian entre
  actividades o controles.
- `.desktop-preview`, para animar la entrada y salida de la notificación de
  escritorio.

AutoAnimate actúa sobre los hijos directos del contenedor: anima su inserción,
eliminación o movimiento. La transición de ancho/alto de la píldora es
independiente y se controla con Tailwind.

El panel de ajustes respeta `prefers-reduced-motion`; la isla no (ver
[Apertura y cierre de la isla flotante](#apertura-y-cierre-de-la-isla-flotante)).

## Componentes y estilo

- `src/components/ui/button/Button.vue` implementa un botón con variantes y usa
  `class-variance-authority` y la utilidad local `cn()`.
- `src/components/ui/toggle-group/index.ts` expone el grupo de selección de
  Reka UI usado para cambiar de actividad.
- `src/components/ui/popover/index.ts` expone las primitivas del selector de
  fondos.
- `components.json` configura el proyecto para **Vite**, **Tailwind CSS 4**,
  TypeScript, alias `@/` e iconos Tabler.
- Google Sans se carga desde Google Fonts; la tipografía alternativa es Segoe
  UI. Para descargarla se necesita conexión a internet. El panel de ajustes usa
  Segoe UI y no carga nada de la red.

**Convención de estilos: Tailwind primero.** El CSS a mano es la excepción y
tiene que tener un motivo.

- La demo web y el panel de ajustes están enteros con utilidades. El panel tiene
  su propia hoja, `settings.css`, porque es otra ventana y no carga `style.css`.
- En la isla, la maquetación de las actividades y los avisos va con utilidades en
  `TheIsland.vue`. En `style.css` quedan las animaciones (`@keyframes`,
  `@property`), los pseudoelementos, los selectores `:has()` y las medidas que
  Rust replica; y en el `<style>` de `IslandApp.vue`, las muescas.
- Un elemento no debe llevar dos utilidades que fijen la misma propiedad: Tailwind
  las emite en un orden que no es el del atributo. Y las clases tienen que
  aparecer **literales** en el archivo, no construidas con plantillas.
- Evita reglas globales sin capa, del estilo `button { font: inherit }`: ganan a
  todas las utilidades y rompen tamaños de texto sin avisar.

Los estados de ejemplo de la isla toman como referencia la interacción de
[Skiper UI · Skiper 2][skiper]. El diseño y la implementación de esta demo son
propios.

## Aplicación de escritorio con Tauri

La configuración en `src-tauri/tauri.conf.json` define:

- Una ventana `main` transparente, sin bordes, siempre visible, sin entrada en
  la barra de tareas y oculta al arrancar; carga `island.html`. Su tamaño real lo
  fija `place_island` en `lib.rs`, no esta configuración.
- `npm run dev` como comando de desarrollo del frontend y
  `http://localhost:1420` como URL de desarrollo.
- `npm run build` como comando previo a compilar el bundle.
- `../dist` como directorio frontend para el bundle.
- Empaquetado activo solo para **NSIS** (instalador por usuario, sin permisos de
  administrador).

La capability predeterminada concede `core:default` a la ventana `main`, que ya
incluye los permisos de eventos que usa la integración multimedia. El builder
Rust de `src-tauri/src/lib.rs` registra los comandos de
[música del sistema](#música-del-sistema-smtc).

**Qué está conectado a Windows y qué sigue siendo simulado.** La isla real lee o
controla la música (SMTC), el volumen maestro (Core Audio), la batería y el
cargador, el Wi-Fi (API WLAN), la salida a internet (Network List Manager) y las
notificaciones de otras aplicaciones. Las actividades de llamada, grabación,
temporizador y transferencia son solo de la demo web y no tocan el sistema.

## Música del sistema (SMTC)

El estado de música se conecta a los **System Media Transport Controls** de
Windows, la misma API que alimenta el panel multimedia del sistema. La isla
refleja lo que realmente está sonando en cualquier reproductor compatible
—Spotify, Edge, Chrome, Firefox, Groove, la mayoría de apps modernas— sin
integrarse con cada uno por separado.

### Qué hace

- Lee título, artista, álbum, carátula, estado de reproducción, posición y
  duración de la sesión multimedia activa.
- Los botones de reproducir/pausar, pista anterior y siguiente controlan el
  reproductor real.
- Al hacer clic en la barra de progreso, salta a esa posición de la pista.
- Muestra y controla la sesión que **está sonando**. La sesión «actual» de
  Windows es la última con la que interactuaste, y no tiene por qué ser la que
  suena: con Spotify en pausa y YouTube Music reproduciendo, la isla sigue a
  YouTube Music.
- Sin ninguna reproducción en curso, muestra «Sin reproducción».

### Cómo está implementado

| Archivo | Responsabilidad |
| --- | --- |
| `src-tauri/src/smtc.rs` | Acceso a SMTC mediante `windows::Media::Control`: lectura de estado, comandos y suscripción a eventos. |
| `src-tauri/src/lib.rs` | Comandos Tauri (`media_state`, `media_play_pause`, `media_next`, `media_previous`, `media_seek`) y emisión del evento `smtc://media-changed`. |
| `src/lib/media.ts` | Composable `useSystemMedia()`: escucha el evento, invoca los comandos y aplica el fallback web. |
| `src/App.vue` | Consume el composable en el estado de música. |

Windows notifica los cambios de pista y de estado mediante eventos, pero no el
avance continuo de la reproducción. Por eso el backend combina ambas vías: los
eventos de SMTC para los cambios inmediatos y un sondeo para la posición, que
solo emite al frontend cuando algo cambió de verdad.

**La posición de SMTC se interpola.** `Position` se queda congelada entre
actualizaciones, que llegan como mucho una vez por segundo: leyéndola tal cual,
la letra de karaoke avanza a tirones y hasta un segundo por detrás. El backend
le suma el tiempo transcurrido desde `LastUpdatedTime` y sondea cada 250 ms,
de modo que la posición avanza de forma continua. La carátula está cacheada por
pista, así que ese sondeo más frecuente no la redecodifica (salvo al empezar la
pista, ver [YouTube Music](#youtube-music-y-otros-reproductores-web)).

La carátula llega como stream de WinRT y se envía al frontend ya codificada en
un data URI, de modo que no hace falta escribir archivos temporales ni
limpiarlos después. Se descartan las imágenes de más de 4 MB. Si el reproductor
no expone carátula, la isla muestra la inicial del título.

El salto de posición usa `TryChangePlaybackPositionAsync`, que no todos los
reproductores admiten —una radio en directo lo rechazará—; en ese caso el
backend devuelve `false` y la reproducción sigue intacta.

La dependencia de Windows está declarada bajo `[target."cfg(windows)"]`, y
`smtc.rs` incluye una implementación vacía para otras plataformas, de modo que
el proyecto sigue compilando fuera de Windows.

### YouTube Music y otros reproductores web

Los reproductores web —YouTube Music, ya sea en el navegador o instalado como
aplicación (PWA)— publican sus datos a su manera, y la isla lo compensa:

- **Título y artista limpios.** YouTube Music informa al artista como
  `Nombre - Topic` o `NombreVEVO`, y los títulos llevan `(Official Video)`,
  `[Lyrics]` o `[HD]`, a veces con el artista repetido delante. Se quita solo lo
  que es adorno: `(Live)`, `(feat. Ana)`, `(Remix)` o `(Remastered 2011)` se
  conservan. La letra se busca con el texto limpio, así que LRCLIB encuentra
  más canciones.
- **Carátula correcta.** Estos reproductores publican el título antes que la
  miniatura, y durante un instante esta sigue siendo la de la canción anterior.
  Por eso la carátula se relee unas veces al empezar cada pista, hasta que se
  asienta, en vez de quedarse con la primera lectura.
- **Control de la sesión que suena**, como se explica arriba.

### Comportamiento en el navegador

SMTC solo existe dentro de la aplicación Tauri. En `npm run dev` abierto en el
navegador no hay acceso a esa API, así que la isla conserva la pista de ejemplo
(«Midnight City») y los controles siguen siendo visuales. La detección es
automática: `src/lib/media.ts` comprueba si la interfaz corre dentro de Tauri e
importa su API solo en ese caso, por lo que la compilación web no incluye ese
código.

## Isla flotante y panel de ajustes

En la aplicación de escritorio la isla no vive dentro de una ventana normal:
es una ventana transparente, sin bordes ni barra de título, anclada al borde
superior de la pantalla y por encima del resto de ventanas. No aparece en la
barra de tareas, así que su punto de entrada es el **icono de la bandeja del
sistema**, con las opciones *Ajustes…*, *Mostrar u ocultar* y *Salir*.

La ventana tiene **tamaño fijo** y Rust recorta la zona visible a la forma de la
isla, así que lo que no dibuja no se ve ni recibe clics. Ver
[Apertura y cierre](#apertura-y-cierre-de-la-isla-flotante).

Con «Siempre visible» activo, la isla **se reafirma por encima de las demás
ventanas** cada vez que cambia de tamaño, que es cuando sale un aviso, pero solo
si de verdad hay otra ventana «siempre visible» por delante (un reproductor
flotante, el administrador de tareas). No roba el foco. Una aplicación a pantalla
completa exclusiva sigue pudiendo taparla.

> **Nota sobre el clic-atraviesa.** En Windows, activar `set_ignore_cursor_events`
> sobre una ventana transparente añade `WS_EX_TRANSPARENT`, que además de dejar
> pasar los clics impide que la ventana se dibuje. Por eso la isla no lo usa:
> compacta mide 116 × 34 px, lo justo para la píldora, y apenas cubre pantalla.

### Panel de ajustes

Se abre desde la bandeja, en su propia ventana, y sigue disponible con la isla
en segundo plano. Cada cambio se guarda y se aplica al instante, sin botón de
confirmación.

| Ajuste | Efecto |
| --- | --- |
| Alineación horizontal | Coloca la isla a la izquierda, centrada o a la derecha. |
| Separación superior | Distancia al borde superior de la pantalla, de 0 a 120 px. |
| Siempre visible | Mantiene o no la isla por encima de otras ventanas. |
| Vista minimizada | Cómo se ve la isla cerrada: **píldora** (la de siempre) o **pestaña mínima**, una leve lengüeta de 48 × 8 px pegada al borde que casi no tapa las pestañas del navegador. Un clic la abre y otro la cierra. Los avisos siguen apareciendo igual. |
| Segundos en la hora | Muestra o no los segundos en la hora de la [vista de inicio](#vista-de-inicio). Activado por defecto. |
| Iniciar con Windows | Registra la app en el arranque del usuario actual. |
| Letras sincronizadas | Busca la letra en LRCLIB. Al desactivarlo, no sale ninguna consulta del equipo. |
| Salto de los botones | Segundos que avanzan o retroceden los botones: 5, 10, 15 o 30. |
| Avisos del sistema | Interruptor general de los avisos. Debajo, uno por tipo: cargador, volumen, batería baja (al 20 % y al 10 %), **Wi-Fi** e **Internet**. Ver [Avisos del sistema](#avisos-del-sistema). |
| Notificaciones de aplicaciones | Muestra en la isla las notificaciones de Teams, Outlook, WhatsApp u otras apps, y la lista de cuáles. **Apagado por defecto.** Con WhatsApp activado, descarga además la foto del remitente. Ver [abajo](#notificaciones-de-otras-aplicaciones). |

Los ajustes se guardan en `%APPDATA%\DynamicIsland\settings.json`. Un campo nuevo
no invalida un archivo existente: los que faltan toman su valor por defecto, y los
que ya no existen se ignoran. El inicio automático se escribe en la clave `Run`
del usuario, que no requiere permisos de administrador.

### Avisos del sistema

Cuando pasa algo en el equipo, la isla **se despliega** unos segundos con un icono
grande, un título y un subtítulo, y luego vuelve a lo que mostraba. Cerrada, crece
al tamaño del panel; abierta, el aviso ocupa la isla entera y, al retirarse, vuelve
el contenido. Un toque sobre el aviso lo descarta, esté la isla abierta o cerrada.

| Aviso | Cuándo | Qué enseña |
| --- | --- | --- |
| **Cargador** | Al enchufarlo | Batería grande que se llena hasta el porcentaje, con el contador animado. |
| **Cargador desconectado** | Al retirarlo | «Cargador desconectado», el porcentaje y una barra. |
| **Volumen** | Al cambiarlo desde el sistema (teclas, mezclador) | «Volumen» con su nivel y una barra, o «Silencio». Con la isla abierta se ignora: su panel ya tiene el deslizador. Mientras sigues pulsando, el panel se actualiza en el sitio en lugar de repetir la animación. |
| **Batería baja** | Al bajar al 20 % y al 10 % sin cargador | «Batería baja» o «Batería crítica», con porcentaje y barra. Se avisa una vez por umbral. |
| **Wi-Fi** | Al conectarse a una red o cambiar de red | «Conectado» con el nombre de la red, y los arcos del icono que se encienden uno a uno. Arrancar ya conectado no avisa. |
| **Internet** | Al perder la salida a internet, y al recuperarla | «Sin internet» / «Internet restablecido». |

Detalles que conviene saber:

- **Internet** se pregunta al Administrador de redes de Windows, que distingue estar
  conectado al router de tener salida a internet, y cubre también el cable. No
  envía tráfico propio. La pérdida exige dos lecturas seguidas (unos 6 s) para
  ignorar los baches al cambiar de red, y Windows tarda unos segundos en notarla,
  así que el aviso no es instantáneo. «Restablecido» se omite si justo se acaba de
  anunciar una red Wi-Fi.
- **Wi-Fi** solo detecta conexiones inalámbricas (API WLAN); el cable no avisa.
- Dos avisos del mismo tipo se **reemplazan** en lugar de encolarse: al enchufar y
  desenchufar seguido interesa el estado final. Uno de otro tipo espera a que el
  visible cumpla su tiempo mínimo.
- Cada tipo tiene su interruptor, además del general, y se consultan en cada vuelta
  de su sondeo: apagarlos surte efecto en caliente, sin reiniciar.
- Ninguno de estos avisos sale del equipo.

### Vista de inicio

Sin nada sonando, la isla no enseña un reproductor vacío con «Sin reproducción» y
una barra a cero: muestra la **hora** (en la píldora cerrada) y la **hora con la
fecha** (abierta), con segundos; se pueden quitar en ajustes. Se activa sola cuando no hay ninguna sesión multimedia, o hay
una vacía y parada. Una pista en pausa cuenta como medio y sigue mostrando el
reproductor, por si quieres reanudarla. En cuanto algo empieza a sonar, la isla
vuelve sola al reproductor.

### Notificaciones de otras aplicaciones

La isla puede mostrar las notificaciones de Windows de otras apps: una píldora
a tamaño de panel (330 × 108 px) con la aplicación, el remitente y el mensaje. Se lee a
través de `UserNotificationListener`, así que Windows debe tener permitido el
acceso a notificaciones (Configuración → Privacidad y seguridad →
Notificaciones).

- **Apagado por defecto**, porque lee contenido privado. Se activa en ajustes.
- **Si no ves ninguna notificación**, mira el panel de ajustes: si Windows no ha
  concedido el acceso, aparece un aviso naranja con un botón **Abrir Windows**
  (Configuración → Privacidad y seguridad → Notificaciones). Además, una app solo
  aparece en la lista cuando envía su primera notificación, y empieza desactivada:
  activa «Mostrar notificaciones en la isla», recibe un mensaje de WhatsApp, abre
  los ajustes y actívalo en la lista.
- **No sale nada del equipo** (salvo la foto de WhatsApp, ver abajo): el texto pasa
  del sistema a la ventana de la isla y no se guarda en disco ni en registros. Lo
  único que se persiste es el nombre de cada aplicación detectada.
- **Tú eliges qué apps**: la isla las va detectando y las lista en ajustes, todas
  desactivadas. Solo se muestran las que actives. Aparecen al llegar una
  notificación o si ya hay alguna en el centro de notificaciones.
- **El avatar** es la foto del remitente si la hay (WhatsApp, ver abajo); si no, el
  **icono de la aplicación**, que da el propio Windows sin red: el logo del paquete
  en las aplicaciones de la Store, y el icono con el que el menú Inicio enseña la
  app en las clásicas (Outlook de escritorio, antivirus…). Los avisos de globo de un
  icono de la bandeja (FortiClient, clientes VPN) se resuelven por el registro de
  iconos de bandeja de Windows. Si no se puede leer, la inicial de su nombre.
- **Varias seguidas se apilan** en lugar de reemplazarse: dos mensajes son dos
  mensajes.
- **El mensaje tiene espacio:** ocupa hasta **tres líneas** del panel. Si es más
  largo, espera un instante y **sube** a ritmo de lectura (una línea cada 1,2 s)
  hasta enseñar el final, en lugar de cortarse con «…». La isla no cambia de
  tamaño. El aviso se queda lo que tarda en llegar al final, entre 5,5 y 20 s; esa
  duración se estima por caracteres, así que un texto con muchas letras anchas puede
  retirarse algo antes.
- **Un clic abre la aplicación.** Al hacer clic en una notificación de otra app, la
  isla la descarta y abre (o trae al frente) esa app, como desde el menú Inicio. No
  abre el chat ni el mensaje concreto: Windows no permite reenviar la activación de
  un aviso desde fuera. El comando recibe solo el id del aviso, no un nombre de
  programa, de modo que solo puede abrir la app de una notificación que ya vio.

#### Foto del remitente (experimental)

Con WhatsApp activado en la lista de aplicaciones, la isla enseña la foto de perfil
de quien te escribe en lugar de la inicial de la aplicación. No hay un interruptor
aparte: activar WhatsApp es el permiso, y el panel de ajustes avisa de ello. Por
ahora solo funciona con WhatsApp (la versión de la Microsoft Store).

Windows no entrega esa imagen a las apps: va dentro del XML de la notificación,
que Windows guarda en su base de datos `wpndatabase.db`. La isla la abre **solo
para lectura**, saca la dirección de la foto y la descarga de `whatsapp.net`, la
misma dirección que Windows ya usó para enseñarla. Por eso:

- **Sale una petición a la red** por cada contacto nuevo (`pps.whatsapp.net`).
  Las fotos ya vistas se recuerdan en memoria. Si no quieres que salga, deja
  WhatsApp desactivado en la lista.
- Solo se descarga de los servidores de una lista fija, sin seguir
  redirecciones, con límite de tamaño y comprobando que sea una imagen.
- Solo se hace para las apps que tú activaste, y solo prospera con avisos que
  traen foto de un servidor de la lista.
- Nada se guarda: la foto vive en memoria y desaparece al cerrar la isla.
- **No siempre aparece.** Hace falta que el contacto tenga foto visible para ti,
  que la descarga llegue a tiempo (límite de 5 s: sin internet o con un cortafuegos
  que bloquee `pps.whatsapp.net`, sale la inicial) y que sea WhatsApp de la
  Microsoft Store, que es donde se verificó. La descarga ocurre antes de mostrar el
  aviso: con una red muy lenta puede retrasarlo hasta ese límite.
- **No es una API documentada de Windows.** Si una actualización cambia el
  formato de esa base, la foto deja de aparecer y la isla vuelve a la inicial,
  sin más efecto.

#### Usar solo la isla, sin el globo de Windows

La isla lee las notificaciones del **centro de notificaciones** de Windows, no el
globo que aparece en pantalla. Por eso se puede quitar el globo y que las
notificaciones sigan llegando a la isla, sin verlas dos veces.

**Por app (recomendado).** En Windows 11, para cada aplicación que quieras ver solo
en la isla (WhatsApp, Teams, Outlook…):

1. Abre **Configuración → Sistema → Notificaciones** y entra en la app.
2. **Desactiva «Mostrar notificaciones de banner».** Es lo que quita el globo.
3. **Deja activado «Mostrar notificaciones en el centro de notificaciones».** Es el
   paso importante: de ahí las lee la isla.
4. Opcional: desactiva «Reproducir un sonido cuando llegue una notificación» si
   tampoco quieres el sonido de Windows.

En Windows 10 es **Sistema → Notificaciones y acciones**, con «Mostrar banners de
notificación» desactivado y «Mostrar notificaciones en el centro de actividades»
activado.

**Con No molestar (global).** En Windows 11, activa **No molestar** desde el panel
de notificaciones (Win+N → icono de la campana). Oculta todos los globos de golpe, y
las notificaciones siguen llegando al centro. Es más rápido, pero afecta a todas las
apps y no solo a las que ves en la isla. En Windows 10 es el *Asistente de
concentración*.

Qué **no** hacer:

- **No apagues la aplicación entera** con su interruptor principal en esa pantalla,
  ni el interruptor general de notificaciones de Windows. Entonces Windows no genera
  la notificación y la isla no ve nada.
- Una app que **no esté activada** en la lista de la isla, con el globo apagado,
  se queda sin ningún aviso visible. Quita el globo solo a las que sí activaste en
  «Notificaciones de aplicaciones».

> **Sin verificar en todos los casos.** Que la isla las siga viendo con el globo
> apagado se desprende de cómo funciona la API (lee el centro de notificaciones),
> pero no se ha probado con todas las combinaciones de apps y de modos de No
> molestar. Prueba primero con **una sola app**: recibe un mensaje y comprueba que
> sale en la isla y no el globo de Windows.

### Letra sincronizada (karaoke)

Solo se ofrece cuando suena un servicio de streaming musical (Spotify, Apple
Music, Deezer…) o una webapp en el navegador; con un reproductor local no se
consulta nada.

Cuando la pista tiene letra disponible aparece un botón de micrófono entre los
controles. Al pulsarlo, la isla **se alarga en horizontal** y muestra un panel
de karaoke: la línea que suena en ese momento resaltada en verde y a mayor
tamaño, y las de alrededor atenuadas, avanzando con la reproducción.

Las letras vienen de [LRCLIB](https://lrclib.net), una API pública y gratuita
que no requiere registro. SMTC no proporciona letras, así que es necesaria una
fuente externa: la búsqueda envía el título y el artista de la pista a
`lrclib.net`. **Se puede desactivar** desde el panel de ajustes, y entonces no
sale ninguna consulta del equipo.

Dos detalles que hicieron falta para que funcione con reproductores reales:

- Apple Music informa el artista como `Miranda! — El Templo del Pop`, con el
  álbum incluido. Se recorta por el separador antes de consultar, o LRCLIB no
  encuentra nada.
- La consulta exacta (`/api/get`) exige que la duración coincida, cosa que
  falla con recopilatorios y ediciones distintas. Si no hay resultado, se
  reintenta con `/api/search`, que es más tolerante.

### Controles de reproducción

La fila inferior reúne, centrados: pista anterior, retroceder 5 s,
reproducir/pausar, avanzar 5 s y pista siguiente. Los iconos de karaoke y
volumen viven arriba, junto al visualizador, para no saturar esa fila.

La barra de progreso es un control deslizante real: se puede **pulsar o
arrastrar** para moverse por la pista, y con el teclado responde a las flechas
izquierda y derecha. El tirador solo aparece al apuntarla, y la barra engorda
un par de píxeles para que sea más fácil de agarrar.

El salto de los botones se elige en los ajustes: 5, 10, 15 o 30 segundos
(10 por defecto). El icono cambia para reflejar el valor elegido.

> **No todos los reproductores admiten avanzar la pista.** SMTC expone
> `IsPlaybackPositionEnabled`, y algunos lo declaran `false`: **Apple Music es
> uno de ellos**. Acepta la llamada y devuelve `true`, pero no mueve nada —
> enviarle un salto grande llega a expulsarlo de la sesión SMTC. En esos casos
> los botones de salto quedan **deshabilitados** y la barra deja de responder
> al arrastre, sin más aviso que el estado visual. En Spotify, Edge y Chrome
> el salto sí funciona.

### Volumen del sistema

El icono de volumen de la isla no es decorativo: controla el volumen maestro de
Windows mediante la API Core Audio (`IAudioEndpointVolume`), el mismo que mueven
las teclas de volumen del teclado. Al pasar el ratón sobre el icono aparece un
deslizador, y pulsarlo silencia o restaura el audio.

## Limitaciones actuales

- La demo corre como página web dentro del navegador o WebView.
- La isla no reproduce audio por sí misma: lee y controla el reproductor que ya
  esté sonando. En el navegador, el estado de música es una pista de ejemplo.
- La llamada, grabación, transferencia y notificación son simulaciones locales;
  no usan micrófono, red ni servicios externos.
- El progreso de transferencia es de muestra, no descarga un archivo.
- Los botones de la barra de tareas simulada no ejecutan acciones de Windows.
- El estado de la interfaz se reinicia al recargar la página.
- La compilación de Rust/Tauri requiere la toolchain de Windows indicada en
  [Requisitos](#requisitos).

### De la aplicación de escritorio

- Solo funciona en Windows 10 y 11 (64 bits).
- El instalador no está firmado: Windows mostrará el aviso de SmartScreen.
- SMTC no dice en qué web suena una pista, así que la isla no distingue YouTube
  Music de un vídeo cualquiera del navegador: ambos cuentan como streaming, y
  si LRCLIB no tiene la letra, simplemente no aparece el botón.
- Las notificaciones de otras aplicaciones dependen de lo que cada app publique
  en Windows: Teams, por ejemplo, solo manda remitente y mensaje si tiene
  activada la vista previa.
- El clic en una notificación abre la aplicación, no el chat o el mensaje
  concreto.
- La foto del remitente solo funciona con WhatsApp de la Microsoft Store, se apoya
  en una base de datos interna de Windows que no está documentada y puede
  dejar de funcionar con una actualización. No aparece si el contacto no tiene
  foto visible o si falla la descarga.
- El aviso de Wi-Fi solo detecta redes inalámbricas, no cable. El de internet
  tarda unos segundos en notar la pérdida, porque depende de la detección de
  Windows.
- Una aplicación a pantalla completa exclusiva puede tapar la isla aunque esté
  «siempre visible».
- Las medidas de la isla, la altura de la ventana nativa y las muescas están
  duplicadas entre el CSS y Rust. Cambiar un tamaño exige tocar las dos.

## Referencia y atribución

La referencia de interacción para los estados de actividad es
[Skiper UI — Skiper 2][skiper]. Skiper UI indica que su versión gratuita se
puede usar y modificar tanto en proyectos personales como comerciales, con
atribución requerida. La demo conserva un enlace de atribución visible.

## Licencia

[MIT](LICENSE). Se puede usar, copiar y modificar, también con fines
comerciales, conservando el aviso de copyright. La atribución a Skiper UI de la
sección anterior se mantiene aparte.

[skiper]: https://skiper-ui.com/v1/skiper2
[tauri-prereq]: https://v2.tauri.app/start/prerequisites/#windows
