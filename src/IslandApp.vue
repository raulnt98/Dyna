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
import { useLyrics } from "@/lib/lyrics";
import { noticeWidth, useNotifications } from "@/lib/notifications";
import { useSystemPower } from "@/lib/power";
import type { Activity } from "@/lib/activities";

/** Tamaños de la isla, en sincronía con las clases de TheIsland.vue. */
const COMPACT = { width: 116, height: 34 };
const EXPANDED = { width: 330, height: 108 };
/** Con la letra abierta la isla se alarga para dejarle sitio al panel. */
const WITH_LYRICS = { width: 540, height: 108 };

const mode = ref<Activity>("music");
const expanded = ref(false);
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
    duration: 3200,
  });
});

const formatTime = (seconds: number) =>
  `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;

const playing = computed(() => media.state.value.playing);
const trackTitle = computed(() => media.state.value.title || "Sin reproducción");
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
  // Un aviso no abre la isla: la ensancha lo que pida su texto, conservando
  // la altura de la píldora. Con la isla ya abierta no cambia nada, porque
  // encogerla sería arrancarle el panel de las manos al usuario.
  if (!expanded.value) {
    return notice.value
      ? { width: noticeWidth(notice.value.label), height: COMPACT.height }
      : COMPACT;
  }
  return lyricsOpen.value && lyricsAvailable.value ? WITH_LYRICS : EXPANDED;
});

// `immediate` asegura que la ventana adopte el tamaño compacto al arrancar:
// sin él conservaría el declarado en la configuración hasta el primer cambio.
/**
 * Espera antes de encoger la ventana, tras la transición de `.island-morph`.
 *
 * Los .58s del CSS más un margen: igualarlos exactamente hace que el
 * `set_size` compita con el último fotograma de la animación y recorte su
 * cola, que es justo donde el muelle se asienta.
 */
const MORPH_MS = 640;
let shrinkTimer: number | undefined;

/**
 * Sincroniza la ventana con el tamaño de la isla.
 *
 * La ventana cambia de golpe mientras el CSS anima la píldora, así que el
 * orden importa: al crecer se agranda primero, para que la isla tenga sitio
 * donde expandirse; al encoger se espera a que la animación termine, o el
 * contenido se vería recortado a media transición.
 */
watch(
  targetSize,
  (size, previous) => {
    // El tamaño del aviso es un objeto nuevo en cada evaluación, así que el
    // watch salta aunque las medidas no cambien: sin esta comparación por
    // valor, actualizar el texto de un aviso redimensionaría la ventana al
    // mismo tamaño que ya tenía, y ese `SetWindowPos` de más se ve parpadear.
    if (previous && size.width === previous.width && size.height === previous.height) return;

    if (shrinkTimer !== undefined) window.clearTimeout(shrinkTimer);

    const growing = !previous || size.width >= previous.width;
    if (growing) {
      invokeWindow("island_resize", size);
      return;
    }
    shrinkTimer = window.setTimeout(() => invokeWindow("island_resize", size), MORPH_MS);
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
const trackPosition = computed(() => media.state.value.position);
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
  // Con un aviso en pantalla, el primer toque lo descarta: abrir la isla
  // encima dejaría el aviso tapado y la cola seguiría corriendo por detrás.
  if (!expanded.value && notice.value) {
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

onMounted(async () => {
  const stored = await invokeResult<{ skipSeconds?: number }>("settings_get");
  if (stored?.skipSeconds) skipSeconds.value = stored.skipSeconds;

  // El primer intento de redimensionar ocurre antes de que la API de Tauri
  // esté cargada, así que se repite aquí ya con el módulo disponible.
  await invokeWindow("island_resize", targetSize.value);
  refreshVolume();

  // Los ajustes pueden cambiar mientras la isla está abierta.
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await listen<{ skipSeconds?: number }>("settings://changed", (event) => {
      if (event.payload?.skipSeconds) skipSeconds.value = event.payload.skipSeconds;
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
  <div class="island-stage">
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
      :mode="mode"
      :notice="notice"
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

.island-stage {
  width: 100vw;
  height: 100vh;
  display: grid;
  /* La ventana ya lleva el margen (WINDOW_PADDING en Rust): la isla va
     centrada dentro de ella, sin relleno extra que la desplace. */
  /* Anclada arriba, no centrada: la ventana se encoge con retraso al
     contraer la isla, y centrarla la haría desplazarse mientras tanto. */
  place-items: start center;
  /* Sin relleno superior: la isla cuelga del borde de la pantalla. La ventana
     se coloca pegada al borde desde Rust, así que su primer píxel es también
     el primero de la isla. */
  padding-top: 0;
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

/* Con el panel de karaoke la píldora se alarga a 540 px; sin esto el tinte
   de las muescas seguiría encuadrado para 330 y mostraría otro trozo de la
   carátula que la isla. */
.island-wrap:has(.island.is-expanded.has-lyrics) {
  --island-width: 540px;
}

/* Igual que `.island-morph` en style.css: quien pidió menos movimiento no
   debe ver animarse tampoco las muescas. */
@media (prefers-reduced-motion: reduce) {
  .island-wrap::before,
  .island-wrap::after,
  .notch-art {
    transition-duration: 0.01ms;
  }
}

/* Colgando del borde, la sombra solo tiene sentido hacia abajo y a los lados;
   hacia arriba caería fuera de la pantalla. */
.island-stage .island,
.island-stage .island.is-expanded {
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
}
</style>
