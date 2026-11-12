# Dynamic Island para Windows

Una isla dinámica para el escritorio de Windows, construida con Vue 3 y Tauri 2.
Cuelga del borde superior de la pantalla, por encima del resto de ventanas, y
muestra lo que suena en el sistema, letras sincronizadas, avisos del equipo y
notificaciones de otras aplicaciones. El repositorio incluye también una demo
web que simula un escritorio Windows con todas las actividades.

> **Estado del proyecto:** la aplicación de escritorio funciona como isla
> flotante real: ventana transparente y sin bordes, anclada al borde superior y
> por encima del resto de ventanas, con la música del sistema vía
> [SMTC](#música-del-sistema-smtc), control de volumen, icono en la bandeja y
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
- **Tailwind CSS 4** mediante `@tailwindcss/vite`, junto con CSS propio para el
  aspecto detallado de la maqueta.
- **Reka UI**, las primitivas accesibles utilizadas por los componentes de
  interfaz locales compatibles con shadcn-vue.
- **shadcn-vue** como convención de componentes: los componentes se mantienen
  como código fuente local en `src/components/ui/`.
- **@tabler/icons-vue** para la iconografía.
- **@formkit/auto-animate** para animar la inserción, eliminación y reordenación
  de elementos internos.
- **Tauri 2 + Rust** como base prevista para el contenedor de escritorio.

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
│   ├── style.css                      # Tailwind y estilos de la isla
│   ├── lib/
│   │   ├── utils.ts                   # Utilidad cn() para clases
│   │   ├── activities.ts              # Actividades de la demo
│   │   ├── media.ts                   # Puente con la música del sistema (SMTC)
│   │   ├── lyrics.ts                  # Letras sincronizadas y línea activa
│   │   ├── clock.ts                   # Reloj local suave para la letra
│   │   ├── notifications.ts           # Cola de avisos de la isla
│   │   ├── power.ts                   # Estado del cargador
│   │   └── systemEvents.ts            # Volumen, batería baja y notificaciones de apps
│   └── components/
│       ├── TheIsland.vue              # La píldora, compartida por demo e isla real
│       ├── SettingsSelect.vue         # Desplegable propio del panel de ajustes
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
        ├── lyrics.rs                  # Letras desde LRCLIB
        ├── toasts.rs                  # Notificaciones de otras aplicaciones
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

La píldora cambia de tamaño con una curva `linear()` que aproxima un muelle
(rigidez ~210, amortiguación ~26), en lugar de un easing plano: entra rápido y
se asienta sin rebote brusco.

Coordinar eso con la ventana nativa tiene truco, porque la ventana cambia de
tamaño de golpe mientras el CSS anima durante medio segundo:

- **Al expandir**, la ventana crece *antes* de que empiece la animación, para
  que la isla tenga sitio donde expandirse sin recortarse.
- **Al contraer**, la ventana espera a que la animación termine; encogerla
  antes cortaría el contenido a media transición.
- **La isla va anclada al borde superior de la ventana**, no centrada. Como la
  ventana se encoge con retraso, centrarla haría que la isla apareciera más
  abajo durante ese tiempo.
- El contenido entra solo con opacidad, sin desplazamiento: cualquier
  `translateY` se percibe como un salto sobre una píldora que ya se está
  redimensionando.

El panel de karaoke usa el mismo mecanismo: al abrirlo la ventana se ensancha
antes de que la píldora crezca, y al cerrarlo espera a que la animación
termine. Los controles conservan su ancho mientras la isla se alarga a su
alrededor, de modo que no se reacomodan en cada fotograma.



### Expansión y contracción

El tamaño y las esquinas de la isla cambian mediante clases de Tailwind en
`src/App.vue`:

```text
transition-[width,height,border-radius]
duration-500
ease-[cubic-bezier(0.16,1,0.3,1)]
```

El estado expandido tiene un tamaño máximo de `356 × 150 px`; el compacto mide
`126 × 40 px`. La clase de ancho expandido limita el ancho disponible en
escritorios pequeños. El CSS de la isla mantiene el centrado horizontal y oculta
el contenido que queda fuera de sus bordes durante la transición.

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

El CSS global también incluye una regla `prefers-reduced-motion` que reduce las
animaciones CSS para quienes hayan solicitado movimiento reducido en su sistema.

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
  UI. Para descargarla se necesita conexión a internet.

Los estados de ejemplo de la isla toman como referencia la interacción de
[Skiper UI · Skiper 2][skiper]. El diseño y la implementación de esta demo son
propios.

## Aplicación de escritorio con Tauri

La configuración en `src-tauri/tauri.conf.json` define:

- Una ventana llamada **Dynamic Island**, inicialmente de `1280 × 900 px`.
- Tamaño mínimo de `360 × 560 px` y centrado inicial.
- `npm run dev` como comando de desarrollo del frontend y
  `http://localhost:1420` como URL de desarrollo.
- `npm run build` como comando previo a compilar el bundle.
- `../dist` como directorio frontend para el bundle.
- Empaquetado activo para los destinos admitidos por Tauri.

La capability predeterminada concede `core:default` a la ventana `main`, que ya
incluye los permisos de eventos que usa la integración multimedia. El builder
Rust de `src-tauri/src/lib.rs` registra los comandos de
[música del sistema](#música-del-sistema-smtc).

**La ventana configurada no es aún una isla nativa de escritorio.** Actualmente
no es transparente, flotante, siempre visible, posicionable sobre otras
ventanas ni conectada al centro de notificaciones, audio, llamadas o
temporizadores de Windows. Esas capacidades requieren configuración de ventana,
permisos y código nativo adicionales.

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
- Si cambias de reproductor, la isla sigue a la nueva sesión activa.
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
pista, así que ese sondeo más frecuente no la redecodifica.

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

La ventana cambia de tamaño al expandirse o contraerse la isla, de modo que
nunca ocupa más espacio del que dibuja.

> **Nota sobre el clic-atraviesa.** En Windows, activar `set_ignore_cursor_events`
> sobre una ventana transparente añade `WS_EX_TRANSPARENT`, que además de dejar
> pasar los clics impide que la ventana se dibuje. Por eso la isla no lo usa:
> compacta mide 104 × 32 px, lo justo para la píldora, y apenas cubre pantalla.

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
| Iniciar con Windows | Registra la app en el arranque del usuario actual. |
| Letras sincronizadas | Busca la letra en LRCLIB. Al desactivarlo, no sale ninguna consulta del equipo. |
| Salto de los botones | Segundos que avanzan o retroceden los botones: 5, 10, 15 o 30. |
| Avisos del sistema | Interruptor general de los avisos. Debajo, uno por tipo: cargador, volumen y batería baja (al 20 % y al 10 %). |
| Notificaciones de aplicaciones | Muestra en la isla las notificaciones de Teams, Outlook u otras apps. **Apagado por defecto.** Ver abajo. |

Los ajustes se guardan en `%APPDATA%\DynamicIsland\settings.json`. El inicio
automático se escribe en la clave `Run` del usuario, que no requiere permisos
de administrador.

### Notificaciones de otras aplicaciones

La isla puede mostrar las notificaciones de Windows de otras apps: una píldora
más alta con el remitente y la primera línea del mensaje. Se lee a través de
`UserNotificationListener`, así que Windows debe tener permitido el acceso a
notificaciones (Configuración → Privacidad y seguridad → Notificaciones).

- **Apagado por defecto**, porque lee contenido privado. Se activa en ajustes.
- **No sale nada del equipo**: el texto pasa del sistema a la ventana de la isla
  y no se guarda en disco ni en registros. Lo único que se persiste es el
  nombre de cada aplicación detectada.
- **Tú eliges qué apps**: la isla las va detectando y las lista en ajustes, todas
  desactivadas. Solo se muestran las que actives. Aparecen al llegar una
  notificación o si ya hay alguna en el centro de notificaciones.

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
