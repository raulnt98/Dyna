<script setup lang="ts">
/**
 * La isla como ventana flotante del escritorio real.
 *
 * A diferencia de la demo, aquí no hay escritorio simulado ni selector de
 * actividades: la ventana es del tamaño de la isla y se mueve y redimensiona
 * desde Rust según el estado.
 */
import { computed, onMounted, ref, watch } from "vue";
import TheIsland from "@/components/TheIsland.vue";
import { useSystemMedia } from "@/lib/media";
import { useSmoothPosition } from "@/lib/clock";
import { useLyrics } from "@/lib/lyrics";
import { noticeWidth, useNotifications } from "@/lib/notifications";
import { useSystemPower } from "@/lib/power";
import { tidyToast, useLowBatteryEvents, useToastEvents, useInternetEvents, useVolumeEvents, useWifiEvents } from "@/lib/systemEvents";
import type { Activity } from "@/lib/activities";

/**
 * Cuánto debe quedarse en pantalla una notificación según su mensaje.
 *
 * El mensaje ocupa hasta tres líneas y, si pasa de ahí, sube hasta enseñar el
 * final (ver ToastMarquee.vue); el aviso tiene que durar lo que tarda en llegar, o
 * se retiraría a medio leer. Se **estima** a partir de los caracteres, como
 * `noticeWidth()`: aquí no hay texto dibujado que medir. A 12,5 px un carácter ocupa
 * unos 6,2 px y la fila mide unos 232, así que caben unos 37 por línea. Cada línea
 * de más mide 16,25 px; el desplazamiento arranca a los 0,9 s, va a 14 px/s (SPEED
 * en ToastMarquee.vue) y se queda un par de segundos en el final.
 */
function toastDuration(body: string): number {
  const lines = Math.ceil(body.length / 37);
  const overflow = Math.max(0, lines - 3) * 16.25;
  if (overflow === 0) return 5500;
  const ms = 900 + (overflow / 14) * 1000 + 2200;
  return Math.min(20000, Math.max(5500, Math.round(ms)));
}

/** Tamaños de la isla, en sincronía con las clases de TheIsland.vue. */
const COMPACT = { width: 116, height: 34 };
/** Pestaña mínima de la vista minimizada alternativa. Su alto (8 px) lo usa Rust para elegir la muesca. */
const TAB = { width: 48, height: 8 };
const EXPANDED = { width: 330, height: 108 };

/** Con la letra abierta la isla se alarga para dejarle sitio al panel. */
const WITH_LYRICS = { width: 540, height: 108 };

const mode = ref<Activity>("music");
const expanded = ref(false);
/** Lado de la pantalla al que se pega la isla, según los ajustes. */
type Alignment = "left" | "center" | "right";
const alignment = ref<Alignment>("center");

/**
 * Alineación de la zona visible dentro de la ventana fija.
 *
 * La ventana ya lleva su margen (WINDOW_PADDING en Rust): centrada, la isla no
 * necesita relleno. Con alineación a un lado la zona visible se pega a ese
 * borde (ver `place_island` en lib.rs), y los 50 px son WINDOW_PADDING +
 * OVERSHOOT_MARGIN. Cada rama lleva su propio `justify-items`: dos utilidades
 * así en el mismo elemento se pisarían según el orden en que Tailwind las emite.
 */
const stageAlignment = computed(() => {
  if (alignment.value === "left") return "justify-items-start pl-[50px]";
  if (alignment.value === "right") return "justify-items-end pr-[50px]";
  return "justify-items-center";
});
/** Aspecto de la isla cerrada, según los ajustes: píldora o pestaña mínima. */
type CollapsedStyle = "pill" | "tab";
const collapsedStyle = ref<CollapsedStyle>("pill");
const media = useSystemMedia();

/** Cola de avisos: interrumpen la píldora compacta y se retiran solos. */
const notifications = useNotifications();
const notice = notifications.current;

/**
 * Aviso al conectar o desconectar el cargador.
 *
 * Solo se anuncia en equipos con batería, y el estado inicial no cuenta:
 * arrancar enchufado no es una novedad.
 */
useSystemPower((state) => {
  notifications.notify({
    kind: "power",
    label: state.plugged ? `${state.percent}% · Cargando` : `${state.percent}%`,
    // Al enchufar, el aviso es grande y se queda algo más: la animación de la
    // batería necesita su segundo largo para verse entera. Al desenchufar se
    // despliega con el formato genérico.
    duration: state.plugged ? 4500 : 3200,
    data: state.plugged
      ? { plugged: true, percent: state.percent, large: true }
      : {
          plugged: false,
          percent: state.percent,
          deployed: true,
          title: "Cargador desconectado",
          subtitle: `${state.percent}% de batería`,
          level: state.percent,
        },
  });
});

/**
 * Aviso de volumen al cambiarlo desde el sistema (teclas, mezclador…).
 *
 * Con la isla abierta se ignora: el panel ya tiene su control deslizante, que
 * muestra el valor, y un aviso encima taparía justo lo que el usuario está
 * moviendo. El silencio se anuncia como texto, no como un 0 %.
 */
useVolumeEvents((change) => {
  if (expanded.value) return;
  notifications.notify({
    kind: "volume",
    label: change.muted || change.level === 0 ? "Silencio" : `Volumen ${change.level}%`,
    duration: 1800,
    data: {
      muted: change.muted || change.level === 0,
      level: change.muted ? 0 : change.level,
      deployed: true,
      title: change.muted || change.level === 0 ? "Silencio" : "Volumen",
      subtitle: change.muted || change.level === 0 ? "Sonido desactivado" : `${change.level}%`,
    },
  });
});

/**
 * Notificación de otra aplicación (Teams, Outlook…).
 *
 * Se apilan en lugar de reemplazarse: dos mensajes seguidos son dos mensajes.
 * Con la isla abierta ocupa la isla entera, como cualquier otro aviso, y al
 * retirarse vuelve el contenido.
 */
useToastEvents((toast) => {
  const { title, body } = tidyToast(toast);
  const text = [title, body].filter(Boolean).join(": ");
  notifications.notify({
    kind: "toast",
    label: `${toast.app} · ${text}`,
    duration: toastDuration(body),
    stack: true,
    data: { tall: true, id: toast.id, app: toast.app, title, body, avatar: toast.avatar ?? null, icon: toast.icon ?? null },
  });
});

/**
 * Aviso al conectarse a una red Wi-Fi.
 *
 * El nombre de la red se recorta: `noticeWidth` estima el ancho a partir del
 * texto y un SSID largo se comería la pantalla.
 */
useWifiEvents((wifi) => {
  if (!wifi.ssid) return;
  const name = wifi.ssid.length > 18 ? `${wifi.ssid.slice(0, 17)}…` : wifi.ssid;
  notifications.notify({
    kind: "wifi",
    label: `Conectado · ${name}`,
    duration: 3600,
    // Aquí el nombre completo: el título y el subtítulo se recortan con
    // puntos suspensivos en el propio panel, no hace falta acortarlo.
    data: { signal: wifi.signal, deployed: true, title: "Conectado", subtitle: wifi.ssid },
  });
});

/**
 * Aviso al perder o recuperar internet.
 *
 * La recuperación usa el tipo `wifi`: es el mismo aviso azul y, si justo se
 * acaba de anunciar otro, lo reemplaza en vez de apilarse.
 */
useInternetEvents((online) => {
  notifications.notify(
    online
      ? {
          kind: "wifi",
          label: "Internet restablecido",
          duration: 3600,
          data: { deployed: true, title: "Internet restablecido", subtitle: "Conexión activa" },
        }
      : {
          kind: "internet",
          label: "Sin internet",
          duration: 4500,
          data: { deployed: true, title: "Sin internet", subtitle: "Revisa tu conexión" },
        },
  );
});

/** Aviso al cruzar el 20 % y el 10 % de batería sin cargador. */
useLowBatteryEvents((state) => {
  notifications.notify({
    kind: "battery",
    label: state.percent <= 10 ? `Batería crítica ${state.percent}%` : `Batería baja ${state.percent}%`,
    duration: 5000,
    data: {
      percent: state.percent,
      deployed: true,
      title: state.percent <= 10 ? "Batería crítica" : "Batería baja",
      subtitle: `${state.percent}% · conecta el cargador`,
      level: state.percent,
    },
  });
});

const formatTime = (seconds: number) =>
  `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;

const playing = computed(() => media.state.value.playing);
const trackTitle = computed(() => media.state.value.title || "Sin reproducción");

/**
 * ¿Hay algún medio con el que el reproductor tenga algo que enseñar?
 *
 * Sin sesión SMTC, o con una sesión vacía y parada, el reproductor solo
 * mostraría «Sin reproducción» y una barra a cero. Una pista en pausa sí
 * cuenta: el usuario puede querer reanudarla.
 */
const hasMedia = computed(() => {
  const { hasSession, title, playing } = media.state.value;
  return hasSession && (title !== "" || playing);
});

/**
 * Lo que se dibuja de verdad: sin medios, el modo «música» cede a la vista de
 * inicio. `mode` no cambia, así que en cuanto algo empieza a sonar la isla
 * vuelve sola al reproductor, sin que nadie tenga que acordarse de volver.
 */
const displayMode = computed<Activity>(() =>
  mode.value === "music" && !hasMedia.value ? "home" : mode.value,
);
const trackSubtitle = computed(() => {
  const { artist, album } = media.state.value;
  return [artist, album].filter(Boolean).join(" · ");
});
const trackInitial = computed(() => trackTitle.value.charAt(0).toUpperCase() || "M");
const trackArtwork = computed(() => media.state.value.artwork);
/** Algunos reproductores no permiten saltar de posición (Apple Music). */
const canSeek = computed(() => media.state.value.canSeek);
const positionLabel = computed(() => formatTime(media.state.value.position));
const durationLabel = computed(() => formatTime(media.state.value.duration));
/** Tiempo que falta, en negativo, como en los reproductores del sistema. */
const remainingLabel = computed(() => {
  const { position, duration } = media.state.value;
  if (duration <= 0) return formatTime(0);
  return `-${formatTime(Math.max(0, duration - position))}`;
});
const trackProgress = computed(() => {
  const { position, duration } = media.state.value;
  if (duration <= 0) return 0;
  return Math.min(100, Math.max(0, (position / duration) * 100));
});

/** Llama a un comando de Tauri, ignorando el fallo fuera de la app. */
async function invokeWindow(command: string, args?: Record<string, unknown>) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke(command, args);
  } catch {
    // Fuera de la app de escritorio no hay ventana que ajustar.
  }
}

/** Igual que invokeWindow, pero devolviendo el resultado del comando. */
async function invokeResult<T>(command: string, args?: Record<string, unknown>) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return undefined;
  }
}

// La ventana sigue al tamaño de la isla para no dejar zona muerta alrededor.
// Compacta ocupa 104x32: lo justo para la píldora, así que apenas tapa nada
// del escritorio y no hace falta dejar pasar los clics. (En Windows, activar
// el clic-atraviesa sobre una ventana transparente la vuelve invisible.)
/** Tamaño que debe tener la ventana según lo que haya abierto. */
const targetSize = computed(() => {
  // Cerrada, los avisos despliegan la isla al tamaño de su panel. Abierta no
  // cambia el tamaño: el panel del aviso ocupa la isla tal como está (con o sin
  // letra) y, al retirarse, vuelve el contenido.
  if (!expanded.value) {
    // El aviso del cargador crece al tamaño del panel (ver `showsCharge`).
    if (notice.value?.data?.large || notice.value?.data?.deployed) return EXPANDED;
    // La notificación de otra app mide lo mismo que el panel: avatar, remitente y mensaje.
    if (notice.value?.data?.tall) return EXPANDED;
    return notice.value
      ? { width: noticeWidth(notice.value.label), height: COMPACT.height }
      : collapsedStyle.value === "tab"
        ? TAB
        : COMPACT;
  }
  return lyricsOpen.value && lyricsAvailable.value ? WITH_LYRICS : EXPANDED;
});

// `immediate` asegura que la ventana adopte el tamaño compacto al arrancar:
// sin él conservaría el declarado en la configuración hasta el primer cambio.
/**
 * Pide a Rust el tamaño de la isla.
 *
 * La ventana nativa tiene tamaño fijo: Rust solo recorta la zona visible
 * (`SetWindowRgn`), así que no hay viewport que actualizar ni fotograma
 * viejo que se cuele. Por eso la isla puede centrarse sin más en la ventana.
 */
function resizeWindow(size: { width: number; height: number }) {
  invokeWindow("island_resize", size);
}

/**
 * Sincroniza el recorte de la ventana con el tamaño de la isla.
 *
 * Rust anima el recorte con la misma curva y duración que el CSS de la
 * píldora (`animate_island`), así que se avisa al mismo tiempo al crecer y al
 * encoger. Antes, al encoger se esperaba 640 ms y el recorte viejo seguía a la
 * vista mientras la píldora ya era pequeña: dos tamaños y un salto final.
 */
watch(
  targetSize,
  (size, previous) => {
    // El tamaño del aviso es un objeto nuevo en cada evaluación, así que el
    // watch salta aunque las medidas no cambien: sin esta comparación por
    // valor, actualizar el texto de un aviso redimensionaría la ventana al
    // mismo tamaño que ya tenía, y ese `SetWindowPos` de más se ve parpadear.
    if (previous && size.width === previous.width && size.height === previous.height) return;

    resizeWindow(size);
  },
  { immediate: true },
);

watch(expanded, (isExpanded) => {
  // El volumen puede haber cambiado desde el sistema mientras estaba cerrada.
  if (isExpanded) refreshVolume();
  // Al contraerse, la letra se cierra para no reabrirse enorme.
  else lyricsOpen.value = false;
});

/** Letra sincronizada de la pista, y si su panel está abierto. */
const lyricsOpen = ref(false);
const trackKey = computed(() => {
  const { title, artist } = media.state.value;
  return title ? `${title} — ${artist}` : "";
});
// Reloj local: la posición del sistema llega a saltos de 250 ms y la letra
// necesita seguirla fotograma a fotograma. Solo corre con el panel abierto.
const trackPosition = useSmoothPosition(() => media.state.value, lyricsOpen);
const lyrics = useLyrics(trackKey, trackPosition);
// Se exponen sueltos: dentro del template un computed anidado en un objeto
// no se desenvuelve solo, y `lyrics.available.value` quedaría indefinido.
const lyricsAvailable = lyrics.available;
const lyricLines = lyrics.visibleLines;

/** Volumen maestro del sistema, de 0 a 100. */
const volume = ref(50);
const muted = ref(false);

/** Relee el volumen real del sistema. */
async function refreshVolume() {
  const result = await invokeResult<[number, boolean]>("volume_get");
  if (result) {
    volume.value = result[0];
    muted.value = result[1];
  }
}

function setVolume(value: number) {
  volume.value = value;
  if (muted.value && value > 0) muted.value = false;
  invokeWindow("volume_set", { value });
}

async function toggleMute() {
  const next = await invokeResult<boolean>("volume_toggle_mute");
  if (next !== undefined) muted.value = next;
}

function toggleIsland() {
  // Con un aviso en pantalla, el primer toque lo descarta, esté la isla como
  // esté: abrirla o cerrarla debajo dejaría el aviso tapado y la cola seguiría
  // corriendo por detrás. En una notificación de otra aplicación, además abre esa
  // aplicación (Rust busca cuál por el id; no abre el chat concreto).
  if (notice.value) {
    const id = notice.value.kind === "toast" ? notice.value.data?.id : undefined;
    if (typeof id === "number") void invokeResult("toast_open", { id });
    notifications.dismiss();
    return;
  }
  expanded.value = !expanded.value;
}

function handleIslandClick(event: MouseEvent) {
  if (event.target instanceof Element && event.target.closest("button")) return;
  toggleIsland();
}

function handleIslandKeydown(event: KeyboardEvent) {
  if (event.target !== event.currentTarget) return;
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    toggleIsland();
  }
}

/** Segundos de salto, leídos de los ajustes del usuario. */
const skipSeconds = ref(10);
/** Segundos en la hora de la vista de inicio. Con `!== undefined`, no por verdad: `false` es un valor válido. */
const homeSeconds = ref(true);

onMounted(async () => {
  const stored = await invokeResult<{
    skipSeconds?: number;
    position?: Alignment;
    collapsedStyle?: CollapsedStyle;
    homeSeconds?: boolean;
  }>("settings_get");
  if (stored?.homeSeconds !== undefined) homeSeconds.value = stored.homeSeconds;
  if (stored?.skipSeconds) skipSeconds.value = stored.skipSeconds;
  if (stored?.position) alignment.value = stored.position;
  if (stored?.collapsedStyle) collapsedStyle.value = stored.collapsedStyle;

  // El primer intento de redimensionar ocurre antes de que la API de Tauri
  // esté cargada, así que se repite aquí ya con el módulo disponible.
  await invokeWindow("island_resize", targetSize.value);
  refreshVolume();

  // Los ajustes pueden cambiar mientras la isla está abierta.
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await listen<{
      skipSeconds?: number;
      position?: Alignment;
      collapsedStyle?: CollapsedStyle;
      homeSeconds?: boolean;
    }>("settings://changed", (event) => {
      if (event.payload?.homeSeconds !== undefined) homeSeconds.value = event.payload.homeSeconds;
      if (event.payload?.skipSeconds) skipSeconds.value = event.payload.skipSeconds;
      if (event.payload?.position) alignment.value = event.payload.position;
      if (event.payload?.collapsedStyle) collapsedStyle.value = event.payload.collapsedStyle;
    });
  } catch {
    // Fuera de Tauri no hay eventos que escuchar.
  }
});


/**
 * Adelanta o retrocede la reproducción los segundos indicados.
 *
 * Algunos reproductores (Apple Music) no admiten cambiar la posición por
 * SMTC: en ese caso los botones aparecen deshabilitados.
 */
function skipBy(seconds: number) {
  const { position, duration, canSeek } = media.state.value;
  // Los botones ya se deshabilitan cuando no se puede: esto solo protege
  // la llamada por teclado.
  if (!canSeek || duration <= 0) return;
  const target = Math.min(duration, Math.max(0, position + seconds));
  media.seek(target);
}


/** Salta a la proporcion de la pista indicada por el arrastre (0 a 1). */
function scrubTo(ratio: number) {
  const { duration, canSeek } = media.state.value;
  if (!canSeek || duration <= 0) return;
  media.seek(ratio * duration);
}
</script>

<template>
  <!-- `island-stage` no tiene estilos propios: es el asa de los selectores de la
       isla de más abajo. La ventana tiene tamaño fijo y Rust recorta la zona
       visible, así que la isla crece hacia el centro sin depender del viewport.
       Sin relleno superior: cuelga del borde de la pantalla y su primer píxel es
       también el primero de la ventana. -->
  <div class="island-stage box-border grid h-screen w-screen items-start pt-0" :class="stageAlignment">
    <div class="island-wrap">
      <!-- Tinte de la carátula sobre cada muesca. El relleno base de las
           muescas es negro, y sin esto quedan como dos parches oscuros a los
           lados de una isla coloreada. -->
      <template v-if="trackArtwork">
        <div
          class="notch-art notch-art-left"
          :style="{ '--art': `url(${trackArtwork})` }"
          aria-hidden="true"
        ></div>
        <div
          class="notch-art notch-art-right"
          :style="{ '--art': `url(${trackArtwork})` }"
          aria-hidden="true"
        ></div>
      </template>
    <TheIsland
      :mode="displayMode"
      :notice="notice"
      :collapsed-style="collapsedStyle"
      :expanded="expanded"
      :playing="playing"
      :recording="false"
      :recording-seconds="0"
      :timer-running="false"
      timer-label="00:00"
      call-label="00:00"
      :call-active="false"
      :muted="muted"
      :volume="volume"
      :track-title="trackTitle"
      :track-subtitle="trackSubtitle"
      :track-initial="trackInitial"
      :track-artwork="trackArtwork"
      :position-label="positionLabel"
      :duration-label="durationLabel"
      :remaining-label="remainingLabel"
      :track-progress="trackProgress"
      :can-seek="canSeek"
      :skip-seconds="skipSeconds"
      :home-seconds="homeSeconds"
      :lyrics-available="lyricsAvailable"
      :lyrics-open="lyricsOpen"
      :lyric-lines="lyricLines"
      :format-time="formatTime"
      @island-click="handleIslandClick"
      @island-keydown="handleIslandKeydown"
      @choose="mode = $event"
      @toggle-play="media.togglePlay()"
      @next="media.next()"
      @previous="media.previous()"
      @scrub="scrubTo"
      @toggle-mute="toggleMute"
      @volume="setVolume"
      @toggle-lyrics="lyricsOpen = !lyricsOpen"
      @skip="skipBy"
    />
    </div>
  </div>
</template>

<style>
/* La ventana es transparente: solo debe verse la isla, nunca un recuadro.
   Se fuerza sobre el fondo global que define style.css para la demo. */
:root,
html,
body,
#app {
  margin: 0;
  min-height: 0;
  overflow: hidden;
  background: transparent !important;
}

/* En la ventana flotante la isla se centra sola, sin posicionamiento absoluto.
   Se anula la animación de entrada de la demo: anima `margin-top` desde -8px
   y deja el contenido descentrado dentro de la píldora. */
.island-stage .island {
  position: relative;
  animation: none;
  margin-top: 0;
  top: auto;
  left: auto;
  transform: none;
  /* Entre el relleno de las muescas (z-index 1) y su tinte (3): la píldora
     tapa el negro que asoma por el costado, y el tinte se dibuja por encima
     de ambos para cubrir la curva entera. */
  z-index: 2;
  /* Las esquinas de arriba quedan rectas: de ellas nacen las muescas. En la
     demo del navegador la isla flota sobre un escritorio simulado y conserva
     sus cuatro esquinas redondeadas, por eso esto vive aquí y no en las
     clases compartidas. Hay que ganar a las utilidades `rounded-[…]`. */
  border-top-left-radius: 0 !important;
  border-top-right-radius: 0 !important;
  /* Sin contorno: las muescas son elementos aparte y no pueden llevarlo, así
     que la línea de la píldora se cortaría en seco al llegar a ellas y
     dejaría dos trazos sueltos. Contra el negro del marco no aporta nada. */
  border: none !important;
}

/* --- Esquinas invertidas --- */
/* La isla cuelga del borde superior y sus esquinas de arriba se abren HACIA
   AFUERA: en vez de morder la píldora, añaden material a cada lado que se
   funde con el canto de la pantalla. `border-radius` no sirve, porque
   siempre curva hacia dentro.

   Cada muesca es un cuadrado pegado al costado, por fuera de la isla, al que
   una máscara le quita el cuarto de círculo que mira hacia ella. Lo que
   queda es el triángulo curvo que une el borde superior con el lateral. */
.island-wrap {
  --notch: 14px;
  /* Tamaño de la píldora, para encuadrar el tinte de las muescas igual que
     ella. Coincide con las clases `w-[…] h-[…]` de TheIsland.vue. */
  --island-width: 116px;
  --island-height: 34px;
  position: relative;
  display: inline-block;
}

.island-wrap::before,
.island-wrap::after {
  content: "";
  position: absolute;
  top: 0;
  width: var(--notch);
  height: var(--notch);
  background: #050506;
  pointer-events: none;
  /* Bajo la píldora: así el solape no tapa su contenido. */
  z-index: 1;
  /* Acompañan el redimensionado de la píldora en vez de saltar al valor
     final: `--notch` cambia de golpe al entrar `.is-expanded`, y sin esta
     transición las muescas parpadean a su tamaño nuevo mientras la isla
     todavía se está moviendo. Misma curva y duración que `.island-morph`. */
  transition: width 0.58s, height 0.58s, left 0.58s, right 0.58s;
  /* Misma curva sin rebote que `.island-morph`. */
  transition-timing-function: cubic-bezier(.22, 1, .36, 1);
}

.island-wrap::before {
  left: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
}

.island-wrap::after {
  right: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
}

/* Tinte de la carátula sobre las muescas, con la misma receta que
   `.art-backdrop` aplica a la píldora: misma imagen, mismo desenfoque, misma
   capa oscura y misma opacidad. Así el color sigue la regla de la isla en vez
   de aproximarse con un tono fijo.

   Va en `z-index: 3`, por encima de `.island-wrap::before/::after`: esos
   pintan el negro base y, al venir después en el árbol con el mismo nivel,
   taparían el tinte —el de la derecha en particular, que es `::after`. */
.notch-art {
  position: absolute;
  top: 0;
  width: var(--notch);
  height: var(--notch);
  z-index: 3;
  overflow: hidden;
  opacity: 0.4;
  pointer-events: none;
  /* En sincronía con la muesca de debajo: si el relleno negro se redimensiona
     animado y el tinte salta, asoma el negro por los bordes a media
     transición. La opacidad también cambia entre estados (0.4 ↔ 0.22). */
  transition: width 0.58s, height 0.58s, left 0.58s, right 0.58s, opacity 0.58s;
  /* Misma curva sin rebote que `.island-morph`. */
  transition-timing-function: cubic-bezier(.22, 1, .36, 1);
}

.notch-art-left {
  left: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
}

.notch-art-right {
  right: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
}

/* La carátula se encuadra como en la píldora —misma escala y mismo centro—
   para que el tinte continúe sin salto al cruzar la juntura. */
.notch-art::before {
  content: "";
  position: absolute;
  width: calc(var(--island-width) * 1.6);
  height: calc(var(--island-height) * 1.6);
  top: calc(var(--island-height) * -0.3);
  background-image: var(--art);
  background-size: cover;
  background-position: center;
  filter: blur(20px) saturate(1.6);
}

.notch-art-left::before {
  left: calc(var(--island-width) * -0.3 + var(--notch));
}

.notch-art-right::before {
  right: calc(var(--island-width) * -0.3 + var(--notch));
}

.notch-art::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(5, 5, 6, 0.35), rgba(5, 5, 6, 0.75));
  background-size: 100% var(--island-height);
}

/* Compacta, la píldora atenúa su tinte; las muescas siguen la misma regla. */
.island-wrap:not(:has(.island.is-expanded)) .notch-art {
  opacity: 0.22;
}

.island-wrap:not(:has(.island.is-expanded)) .notch-art::after {
  background: linear-gradient(180deg, rgba(5, 5, 6, 0.6), rgba(5, 5, 6, 0.8));
}

/* Expandida la isla es mucho mayor: la muesca crece para que la transición
   entre el marco y el costado siga leyéndose igual. */
.island-wrap:has(.island.is-expanded) {
  --notch: 22px;
  --island-width: 330px;
  --island-height: 108px;
}

/* Pestaña mínima: la muesca se reduce a 5 px, porque una de 14 px colgaría por
   debajo de una isla de 8 px de alto. Misma medida que `Shape::for_size` en
   lib.rs; si cambias una, cambia la otra. */
.island-wrap:has(.island.is-tab) {
  --notch: 5px;
  --island-width: 48px;
  --island-height: 8px;
}

/* La sombra grande de la isla sobraría en una pestaña de 8 px: se sustituye
   por un filo claro en el borde inferior, para que se distinga también sobre
   ventanas oscuras. */
.island-stage .island.is-tab {
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35), inset 0 -1px 0 rgba(255, 255, 255, 0.18);
}

/* Con el panel de karaoke la píldora se alarga a 540 px; sin esto el tinte
   de las muescas seguiría encuadrado para 330 y mostraría otro trozo de la
   carátula que la isla. */
.island-wrap:has(.island.is-expanded.has-lyrics) {
  --island-width: 540px;
}

/* Colgando del borde, la sombra solo tiene sentido hacia abajo y a los lados;
   hacia arriba caería fuera de la pantalla. */
.island-stage .island,
.island-stage .island.is-expanded {
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
}
</style>
