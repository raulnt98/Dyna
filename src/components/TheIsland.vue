<script setup lang="ts">
/**
 * La isla en sí: píldora que se expande y muestra la actividad en curso.
 *
 * Se usa en dos sitios: dentro del escritorio simulado de la demo web y como
 * ventana flotante en el escritorio real, por eso vive en su propio componente.
 */
import {
  IconArrowDown,
  IconChevronLeft,
  IconChevronRight,
  IconRefresh,
  IconArrowDownToArc,
  IconCloud,
  IconCloudFog,
  IconCloudRain,
  IconCloudSnow,
  IconCloudStorm,
  IconGauge,
  IconMoon as IconMoonWeather,
  IconSun as IconSunWeather,
  IconBell,
  IconBoltFilled,
  IconClock,
  IconDots,
  IconMessage,
  IconMicrophone,
  IconMicrophone2,
  IconPhone,
  IconPlayerPauseFilled,
  IconPlayerPlayFilled,
  IconPlayerTrackNextFilled,
  IconPlayerTrackPrevFilled,
  IconVolume2,
  IconVolumeOff,
  IconX,
} from "@tabler/icons-vue";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import NoticeIcon from "@/components/NoticeIcon.vue";
import SkipIcon from "@/components/SkipIcon.vue";
import ArtSwap from "@/components/ArtSwap.vue";
import WaveBars from "@/components/WaveBars.vue";
import ToastMarquee from "@/components/ToastMarquee.vue";
import { vAutoAnimate } from "@formkit/auto-animate/vue";
import type { Activity } from "@/lib/activities";
import { noticeWidth } from "@/lib/notifications";

const props = defineProps<{
  mode: Activity;
  expanded: boolean;
  playing: boolean;
  recording: boolean;
  recordingSeconds: number;
  timerRunning: boolean;
  timerLabel: string;
  callLabel: string;
  callActive: boolean;
  muted: boolean;
  volume: number;
  trackTitle: string;
  trackSubtitle: string;
  trackInitial: string;
  trackArtwork: string;
  positionLabel: string;
  durationLabel: string;
  remainingLabel: string;
  trackProgress: number;
  canSeek: boolean;
  lyricsAvailable: boolean;
  lyricsOpen: boolean;
  lyricLines: { time: number; text: string; active: boolean }[];
  /**
   * Aviso que interrumpe lo que la isla muestre, o `null` si no hay ninguno.
   *
   * Solo afecta al estado compacto: un aviso no abre la isla, la ensancha.
   */
  notice?: { kind: string; label: string; data?: Record<string, unknown> } | null;
  /**
   * Aspecto de la isla cerrada: la píldora de siempre o una pestaña mínima
   * pegada al borde, que apenas tapa lo que haya debajo. Un aviso sigue
   * ensanchando la píldora aunque se haya elegido la pestaña.
   */
  collapsedStyle?: "pill" | "tab";
  /** Segundos que saltan los botones de avance y retroceso. */
  skipSeconds: number;
  /** Segundos en la hora de la vista de inicio. Sin valor se muestran, como por defecto. */
  homeSeconds?: boolean;
  /** Uso de CPU y memoria (0–100) para la vista de inicio; sin valor no se dibuja. */
  homeStats?: { cpu: number; memory: number; memoryUsedGb: number; memoryTotalGb: number } | null;
  /** Quién usa el micrófono y la cámara ahora mismo; se enseña como puntos de color. */
  privacy?: { microphone: string | null; camera: string | null } | null;
  /** Hay una descarga del navegador en curso: una línea recorre la píldora. */
  downloading?: boolean;
  /** Tiempo actual para la vista de inicio; sin valor no se dibuja. */
  weather?: { city: string; temp: number; text: string; kind: string; max: number | null; min: number | null } | null;
  /** Mostrar el botón de test de velocidad en la vista de inicio. */
  speedtestEnabled?: boolean;
  /** Estado del test de velocidad, para su página de la vista de inicio. */
  speed?: {
    status: "idle" | "running" | "done" | "error";
    phase: "ping" | "down" | null;
    mbps: number;
    down: number | null;
    ping: number | null;
    reason?: "limit" | "network" | null;
  };
  /** Ofrecer la página de CPU y RAM en la vista de inicio. */
  systemEnabled?: boolean;
  /**
   * La isla abierta es un carrusel de páginas (la ventana real). Con él, el reproductor es una página
   * más, junto a la hora, CPU y RAM y la velocidad. Sin él (la demo web, que enseña una actividad por
   * vez) se enseña lo que diga `mode`, como siempre.
   */
  carousel?: boolean;
  /** Hay algo en reproducción: entonces el reproductor entra en la rueda. */
  hasMedia?: boolean;
  formatTime: (seconds: number) => string;
}>();

/**
 * La isla se dibuja como píldora, en vez de como panel.
 *
 * Un aviso ocupa la isla entera, cerrada o abierta: el panel del aviso sustituye
 * al contenido mientras dura y la isla recupera lo que mostraba al retirarse.
 * Abierta conserva su tamaño (con o sin letra); cerrada crece al del panel.
 */
const showsPill = computed(() => !props.expanded);

/**
 * Hora y fecha de la vista de inicio.
 *
 * Se actualiza cada segundo con un temporizador propio en lugar de depender del
 * padre: la demo web y la ventana nativa montan este componente por separado, y
 * ninguna debería tener que acordarse de pasarle el reloj.
 */
const now = ref(new Date());
let homeTimer: number | undefined;
onMounted(() => {
  homeTimer = window.setInterval(() => (now.value = new Date()), 1000);
});
onUnmounted(() => {
  if (homeTimer !== undefined) window.clearInterval(homeTimer);
});
const homeTime = computed(() =>
  new Intl.DateTimeFormat("es", {
    hour: "2-digit",
    minute: "2-digit",
    // `undefined` omite los segundos; `!== false` para que, sin la propiedad, salgan.
    second: props.homeSeconds !== false ? "2-digit" : undefined,
    hour12: false,
  }).format(now.value),
);
const homeDate = computed(() =>
  new Intl.DateTimeFormat("es", { weekday: "long", day: "numeric", month: "long" }).format(now.value).replace(",", ""),
);

/**
 * Aviso grande: al enchufar el cargador la isla crece al tamaño del panel y
 * enseña la batería llenándose.
 *
 * Reutiliza las medidas y la clase `is-expanded` del panel, de modo que las
 * muescas de las esquinas y el recorte de la ventana nativa (que se basa en la
 * altura) encajan sin código nuevo.
 */
const showsCharge = computed(() => props.notice?.data?.large === true);

/**
 * Aviso desplegado: la isla se abre al tamaño del panel (330 × 108) con un icono
 * grande, un título y un subtítulo, en vez de ensanchar la píldora. Es el formato
 * de los avisos del sistema (Wi-Fi, internet, volumen, batería); el cargador y
 * las notificaciones de apps tienen el suyo. Como los otros, reutiliza
 * `is-expanded` para que las muescas y el recorte nativo encajen.
 */
const showsDeployed = computed(() => props.notice?.data?.deployed === true);
/** Icono y tipo del tiempo (`weather.rs`). */
const WEATHER_ICONS = { sun: IconSunWeather, moon: IconMoonWeather, cloud: IconCloud, rain: IconCloudRain, snow: IconCloudSnow, storm: IconCloudStorm, fog: IconCloudFog } as const;
const WEATHER_TONES = {
  sun: "text-[#f4cb82]",
  moon: "text-[#a99bff]",
  cloud: "text-[#b9c3d6]",
  rain: "text-[#6eb4ff]",
  snow: "text-[#cfe6ff]",
  storm: "text-[#e6b36a]",
  fog: "text-[#a2a8b4]",
} as const;
const weatherTone = computed(() => WEATHER_TONES[(props.weather?.kind ?? "cloud") as keyof typeof WEATHER_TONES] ?? WEATHER_TONES.cloud);
const weatherIcon = computed(() => WEATHER_ICONS[(props.weather?.kind ?? "cloud") as keyof typeof WEATHER_ICONS] ?? IconCloud);

/**
 * Páginas de la vista de inicio: la hora en el centro, y a cada lado un indicador que abre otra
 * (CPU y RAM a la izquierda, velocidad a la derecha). Desde una página lateral, el indicador del
 * lado contrario vuelve a la hora. Cabe una página más a cada lado cuando haga falta.
 * Sin el padre (la demo web) lleva su propio estado; al cerrar la isla se vuelve a la hora.
 */
const homePage = defineModel<"clock" | "system" | "speed" | "music">("homePage", { default: "clock" });
watch(
  () => props.expanded,
  (isOpen) => {
    // Al cerrar se vuelve a la página con la que se abrirá: el reproductor si suena algo, si no la hora.
    if (!isOpen) homePage.value = props.carousel && props.hasMedia ? "music" : "clock";
  },
);
/**
 * Qué enseña la isla abierta. En el carrusel, el reproductor o la página de inicio que toque; fuera de
 * él (la demo web), lo que diga `mode`. La píldora cerrada sigue usando `mode`: enseña la música si hay,
 * y la hora si no, sea cual sea la página.
 */
const expandedMode = computed(() => (props.carousel ? (homePage.value === "music" ? "music" : "home") : props.mode));
/** Las páginas, en el orden del carrusel. Una desactivada en los ajustes no está en la rueda. */
type HomePage = "clock" | "system" | "speed" | "music";
const HOME_PAGE_NAMES: Record<HomePage, string> = { clock: "Hora", system: "CPU y memoria", speed: "Test de velocidad", music: "Reproductor" };
const homePages = computed<HomePage[]>(() => {
  const pages: HomePage[] = [];
  if (props.systemEnabled) pages.push("system");
  // El reproductor va entre CPU y RAM y la hora: desde él, una flecha lleva a la hora y la otra al sistema.
  if (props.carousel && props.hasMedia) pages.push("music");
  pages.push("clock");
  if (props.speedtestEnabled) pages.push("speed");
  return pages;
});
/** Página que queda `delta` pasos de la actual, dando la vuelta por los extremos. */
function neighbour(delta: number): HomePage {
  const pages = homePages.value;
  const index = Math.max(0, pages.indexOf(homePage.value));
  return pages[(index + delta + pages.length) % pages.length];
}
const homePageName = (delta: number) => HOME_PAGE_NAMES[neighbour(delta)];
const rotateHome = (delta: number) => {
  homePage.value = neighbour(delta);
};
// Si se apaga en los ajustes la página que se está viendo, se vuelve a la hora.
watch(homePages, (pages) => {
  if (!pages.includes(homePage.value)) homePage.value = "clock";
});
const formatMbps = (value: number) => (value >= 100 ? value.toFixed(0) : value.toFixed(1).replace(".", ","));
const speedStatusText = computed(() => (props.speed?.phase === "down" ? "Midiendo bajada…" : "Midiendo latencia…"));

/**
 * La canción que se presenta usa la mitad del alto del panel (54 px de 108): es una
 * presentación, no un panel de control, y a media altura tapa menos. El relleno, la
 * carátula y el texto se reajustan a esa medida; con la isla abierta no se usa (ahí
 * el panel ya mide lo de siempre).
 */
const showsTrackBar = computed(() => props.notice?.kind === "track" && showsDeployed.value && !props.expanded);
const deployedTitle = computed(() => String(props.notice?.data?.title ?? props.notice?.label ?? ""));
const deployedSubtitle = computed(() => String(props.notice?.data?.subtitle ?? ""));
/** Nivel de 0 a 100 para la barra (volumen, batería); `null` si el aviso no lleva. */
/** Carátula de la canción que se presenta; sustituye al icono. */
const deployedArt = computed(() => {
  const art = props.notice?.data?.art;
  return typeof art === "string" && art.startsWith("data:image/") ? art : "";
});
const deployedLevel = computed(() => {
  const level = props.notice?.data?.level;
  return typeof level === "number" ? Math.min(100, Math.max(0, level)) : null;
});
/** Aviso con barra de nivel (volumen, batería): la barra ocupa todo el ancho, bajo el icono y el texto. */
const deployedWithBar = computed(() => deployedLevel.value !== null && !showsTrackBar.value);
/**
 * Pestaña mínima: la isla cerrada, sin ningún aviso en curso, con ese estilo
 * elegido. 48 × 8 px, en sincronía con `TAB` de IslandApp.vue y con la forma
 * que Rust recorta para alturas de ese orden.
 */
const showsTab = computed(
  () => !props.expanded && !props.notice && props.collapsedStyle === "tab",
);
const chargePercent = computed(() => Number(props.notice?.data?.percent ?? 0));

/**
 * Notificación de otra aplicación: el panel completo (330 × 108) con el remitente
 * y el mensaje. Igual que el aviso del cargador, reutiliza `is-expanded` para
 * que las muescas y el recorte de la ventana (que se basa en la altura) encajen.
 */
const showsToast = computed(() => props.notice?.data?.tall === true);
/** Algún panel de aviso está a la vista: ocupa la isla entera y oculta su contenido. */
const showsPanel = computed(() => showsCharge.value || showsDeployed.value || showsToast.value);
const toastApp = computed(() => String(props.notice?.data?.app ?? ""));
const toastTitle = computed(() => String(props.notice?.data?.title ?? ""));
const toastBody = computed(() => String(props.notice?.data?.body ?? ""));
/** Logo de la aplicación, que hace de avatar cuando no hay foto del remitente. */
const toastMedia = computed(() => {
  const media = props.notice?.data?.media;
  return media === "sticker" || media === "gif" || media === "photo" || media === "voice" ? media : null;
});
const toastMediaDetail = computed(() => String(props.notice?.data?.mediaDetail ?? ""));
const toastIcon = computed(() => {
  const icon = props.notice?.data?.icon;
  return typeof icon === "string" && icon.startsWith("data:image/") ? icon : "";
});
/** Foto del remitente, si la hay; si no, el logo de la app, y si tampoco, la inicial. */
const toastAvatar = computed(() => {
  const avatar = props.notice?.data?.avatar;
  return typeof avatar === "string" && avatar.startsWith("data:image/") ? avatar : "";
});
/** Inicial de la aplicación, que hace de avatar: Windows no entrega el logo. */
const toastInitial = computed(() => toastApp.value.trim().charAt(0).toUpperCase() || "!");

/**
 * Ancho de la píldora.
 *
 * Expandida y sin aviso manda la clase de Tailwind; en píldora el ancho se
 * calcula con la misma función que usa la ventana nativa, para que la isla y
 * su ventana no se desincronicen.
 */
const compactWidth = computed(() => {
  if (!showsPill.value || showsCharge.value || showsToast.value || showsDeployed.value || showsTab.value) return undefined;
  const width = props.notice ? noticeWidth(props.notice.label) : 116;
  return { width: `${width}px` };
});

/* Clases compartidas. Los botones no mezclan utilidades que se pisen entre sí
   (tamaño, color): el orden en que Tailwind las emite no es el del atributo. */
/* Color de cada aviso. Los tipos sin tono propio (notificaciones) no llevan fondo. */
const NOTICE_TONE: Record<string, string> = {
  power: "bg-green/16 text-green",
  volume: "bg-white/14 text-white",
  battery: "bg-[rgba(255,138,112,.18)] text-[#ff8a70]",
  internet: "bg-[rgba(255,138,112,.18)] text-[#ff8a70]",
  wifi: "bg-[rgba(110,180,255,.16)] text-[#6eb4ff]",
  bluetooth: "bg-[rgba(110,180,255,.16)] text-[#6eb4ff]",
  "bluetooth-off": "bg-white/14 text-[#b4b4bd]",
  usb: "bg-white/14 text-white",
  caps: "bg-[rgba(244,203,130,.16)] text-[#f4cb82]",
  num: "bg-[rgba(244,203,130,.16)] text-[#f4cb82]",
  focus: "bg-[rgba(150,130,255,.18)] text-[#a99bff]",
  brightness: "bg-[rgba(244,203,130,.16)] text-[#f4cb82]",
  track: "bg-white/14 text-white",
  mic: "bg-[rgba(255,159,67,.18)] text-[#ff9f43]",
  camera: "bg-green/16 text-green",
  download: "bg-[rgba(110,180,255,.16)] text-[#6eb4ff]",
  speed: "bg-[rgba(169,155,255,.18)] text-[#a99bff]",
};
const noticeTone = (kind: string) => NOTICE_TONE[kind] ?? "";
const RING_PULSE = "grid size-6 flex-none animate-[ring-pulse_1.2s_ease-in-out_infinite] place-items-center rounded-full bg-[#46bd73] text-white [&>svg]:size-[13px]";
const TOP = "flex min-w-0 items-center justify-start gap-[9px]";
/**
 * Flechas del carrusel: **la misma posición en todas las páginas**, la de la fila de controles del reproductor
 * (pegadas a los extremos del área de contenido y al borde de abajo). Una sola definición para el reproductor y
 * para el resto: si cambia una, cambian las demás y no pueden desalinearse al rotar de página.
 */
const NAV_ARROW = "absolute bottom-0 z-2 grid h-7 w-6 place-items-center text-white/30 transition-colors duration-150 hover:text-white";
const COPY = "grid min-w-0 flex-1 gap-0.5";
const COPY_STRONG = "truncate text-[11px] font-[650]";
const COPY_SMALL = "truncate text-[8px] text-[#a2a2a8]";
const CONTACT_AVATAR = "border-white/15 bg-[linear-gradient(145deg,#f8d7aa,#b17aa2_58%,#5a477f)] text-[16px] font-semibold text-white";
const PLAY = "size-[28px] text-[#f0f0f4] [&>svg]:size-[18px]";
const DISMISS = "grid size-[25px] place-items-center rounded-full text-[#92929c] [&>svg]:size-3.5";
const CONTROL = "size-[24px] text-[#f0f0f4] [&>svg]:size-4";
const SKIP = "size-[24px] text-[#b4b4bd] [&>svg]:size-[18px]";
const CALL_CONTROL = "size-[33px] text-[#f0f0f4] [&>svg]:size-[17px]";
const AVATAR = "grid size-[42px] flex-none place-items-center rounded-[13px] border [&>svg]:size-[19px]";
const MC = "grid flex-none place-items-center rounded-full transition-[color,scale] duration-[160ms] [&>svg]:stroke-2";
const MC_HOVER = "hover:scale-[1.12] hover:text-white";
const ICON_ACTION = "grid size-6 place-items-center rounded-md p-0 transition-[color,background-color] duration-150 [&>svg]:size-4";
const ALBUM = "grid flex-none place-items-center overflow-hidden border border-white/12 font-bold shadow-[inset_0_0_20px_rgba(44,23,60,.2)]";
const ALBUM_ART_BG = "bg-[radial-gradient(circle_at_70%_26%,#f6d29a_0_12%,transparent_13%),linear-gradient(145deg,#f0bc88,#a85c76_55%,#4c397a)] text-white/92";

const emit = defineEmits<{
  (e: "island-click", event: MouseEvent): void;
  (e: "island-keydown", event: KeyboardEvent): void;
  (e: "island-enter"): void;
  (e: "speedtest"): void;
  (e: "island-leave", event: MouseEvent): void;
  (e: "choose", value: Activity): void;
  (e: "toggle-play"): void;
  (e: "next"): void;
  (e: "previous"): void;
  (e: "scrub", ratio: number): void;
  (e: "toggle-timer"): void;
  (e: "toggle-recording"): void;
  (e: "toggle-mute"): void;
  (e: "volume", value: number): void;
  (e: "toggle-lyrics"): void;
  (e: "skip", seconds: number): void;
  (e: "end-call"): void;
}>();

const handleIslandClick = (event: MouseEvent) => emit("island-click", event);
const handleIslandKeydown = (event: KeyboardEvent) => emit("island-keydown", event);
const chooseActivity = (value: Activity) => emit("choose", value);
/**
 * Arrastre sobre la barra de progreso.
 *
 * Se captura el puntero para que el gesto siga funcionando aunque el cursor
 * salga de la barra, como en cualquier control deslizante.
 */
function startScrub(event: PointerEvent) {
  event.stopPropagation();
  const bar = event.currentTarget as HTMLElement;
  bar.setPointerCapture(event.pointerId);

  // La posición se calcula aquí, contra la barra que inició el gesto: durante
  // el arrastre `currentTarget` ya no es fiable en cada evento de movimiento.
  const track = bar.querySelector(".track-progress") ?? bar;
  const emitAt = (moveEvent: PointerEvent) => {
    const { left, width } = track.getBoundingClientRect();
    if (width <= 0) return;
    const ratio = Math.min(1, Math.max(0, (moveEvent.clientX - left) / width));
    emit("scrub", ratio);
  };
  emitAt(event);

  const onMove = (moveEvent: PointerEvent) => emitAt(moveEvent);
  const onUp = (upEvent: PointerEvent) => {
    bar.releasePointerCapture(upEvent.pointerId);
    bar.removeEventListener("pointermove", onMove);
    bar.removeEventListener("pointerup", onUp);
    bar.removeEventListener("pointercancel", onUp);
  };

  bar.addEventListener("pointermove", onMove);
  bar.addEventListener("pointerup", onUp);
  bar.addEventListener("pointercancel", onUp);
}

/** Flechas para mover la posición sin ratón. */
function onSeekKeydown(event: KeyboardEvent) {
  const step = event.key === "ArrowRight" ? 5 : event.key === "ArrowLeft" ? -5 : 0;
  if (step === 0) return;
  event.preventDefault();
  event.stopPropagation();
  emit("skip", step);
}
</script>

<template>
  <div
    :class="[
      'island island-morph',
      {
        'is-expanded': expanded || showsCharge || showsToast || showsDeployed,
        'is-charge': showsCharge,
        'is-toast': showsToast,
        'is-track': showsTrackBar,
        'is-tab': showsTab,
        'is-paused': !playing,
      },
      `mode-${mode}`,
      { 'has-lyrics': expanded && lyricsOpen && lyricsAvailable },
      showsTab
        ? 'w-[48px] h-[8px] rounded-[4px]'
        : showsTrackBar
          ? 'w-[330px] h-[54px] rounded-[18px]'
          : showsPill && !showsCharge && !showsDeployed && !showsToast
            ? 'h-[34px] rounded-[18px]'
            : 'w-[330px] h-[108px] rounded-[18px]',
    ]"
    :style="compactWidth"
    role="button"
    tabindex="0"
    :aria-label="`Isla dinámica, ${expanded ? 'contraer' : 'expandir'}`"
    :aria-expanded="expanded"
    @click="handleIslandClick"
    @keydown="handleIslandKeydown"
    @mouseenter="emit('island-enter')"
    @mouseleave="emit('island-leave', $event)"
  >
    <!-- Fondo tintado por la carátula, compartido por ambas vistas. -->
    <Transition name="art-fade">
      <div
        v-if="mode === 'music' && trackArtwork"
        :key="trackArtwork"
        class="art-backdrop"
        :style="{ '--art': `url(${trackArtwork})` }"
        aria-hidden="true"
      ></div>
    </Transition>

    <!-- Micrófono y cámara en uso: un punto naranja y otro verde, como los de iOS. Van fuera de
         `.island-content` (que se desmonta al abrir y cerrar) para no desaparecer con ella. -->
    <span v-if="privacy && (privacy.microphone || privacy.camera)" class="privacy-dots" aria-hidden="true">
      <i v-if="privacy.microphone" class="privacy-dot is-mic"></i>
      <i v-if="privacy.camera" class="privacy-dot is-camera"></i>
    </span>

    <!-- Descarga en curso: una línea que recorre el borde de abajo. Sin porcentaje: el
         navegador no dice cuánto falta (ver downloads.rs). -->
    <span v-if="downloading && showsPill && !notice" class="download-line" aria-hidden="true"></span>

    <!-- Batería cargando: fuera de `.island-content` por lo mismo que la franja,
         `v-auto-animate` pelearía con su animación. -->
    <Transition name="notice-panel"><div
      v-if="showsCharge"
      class="absolute inset-0 z-1 isolate flex items-center justify-center gap-[26px]"
      :style="{ '--pct': chargePercent }"
      aria-hidden="true"
    >
      <div class="flex items-center">
        <div class="relative grid h-[46px] w-[92px] place-items-center overflow-hidden rounded-[13px] border-2 border-white/70 bg-[#050506]">
          <div class="charge-fill"></div>
          <IconBoltFilled class="relative z-1 size-[24px] animate-[charge-pulse_1.1s_ease-in-out_.4s_infinite] text-white drop-shadow-[0_0_3px_rgba(0,0,0,.55)]" />
        </div>
        <span class="ml-[3px] h-[18px] w-[5px] rounded-[0_3px_3px_0] bg-white/70"></span>
      </div>
      <div class="flex flex-col gap-px">
        <strong class="charge-percent text-[34px] leading-none font-bold tabular-nums"></strong>
        <small class="text-[12px] font-semibold tracking-[.02em] text-green">Cargando</small>
      </div>
    </div></Transition>

    <!-- Aviso del sistema desplegado. La `key` es solo el tipo: un aviso del mismo
         tipo (el volumen, que cambia con cada pulsación) actualiza el texto y la
         barra en el sitio. Si la `key` incluyera el valor, cada pulsación
         remontaría el panel y repetiría la animación de entrada, y tres
         pulsaciones se verían como tres avisos. -->
    <Transition name="notice-panel"><div
      v-if="showsDeployed && notice"
      :key="notice.kind"
      class="absolute inset-0 z-1 isolate flex"
      :class="
        showsTrackBar
          ? 'items-center gap-2.5 pr-3.5 pl-2.5'
          : deployedWithBar
            ? 'flex-col justify-center gap-[11px] px-[26px]'
            : 'items-center gap-[18px] px-[26px]'
      "
      aria-hidden="true"
    >
      <!-- Con barra (volumen, batería): arriba icono y texto, y la barra debajo a todo
           el ancho, en vez de encajonada en la columna del texto. Sin barra, la fila
           desaparece (`contents`) y icono y texto quedan como siempre. -->
      <div :class="deployedWithBar ? 'flex min-w-0 items-center gap-4' : 'contents'">
        <img
          v-if="deployedArt"
          :src="deployedArt"
          alt=""
          class="np-lead flex-none object-cover"
          :class="showsTrackBar ? 'size-[34px] rounded-lg shadow-[0_4px_12px_rgba(0,0,0,.5)]' : deployedWithBar ? 'size-11 rounded-[11px] shadow-[0_6px_18px_rgba(0,0,0,.5)]' : 'size-[52px] rounded-xl shadow-[0_6px_18px_rgba(0,0,0,.5)]'"
        />
        <span
          v-else
          class="np-lead notice-icon grid flex-none place-items-center rounded-full"
          :class="[`notice-${notice.kind}`, noticeTone(notice.kind), showsTrackBar ? 'size-[34px] [&>svg]:size-5' : deployedWithBar ? 'size-11 [&>svg]:size-[22px]' : 'size-[52px] [&>svg]:size-[26px]']"
        >
          <NoticeIcon :kind="notice.kind" :muted="notice.data?.muted === true" />
        </span>
        <span class="np-copy grid min-w-0 flex-1" :class="showsTrackBar ? 'gap-px' : 'gap-[3px]'">
          <strong class="truncate leading-[1.15] font-semibold text-white/96" :class="showsTrackBar ? 'text-[14px]' : 'text-[18px]'">{{ deployedTitle }}</strong>
          <span v-if="deployedSubtitle" class="truncate text-white/60 tabular-nums" :class="showsTrackBar ? 'text-[11px]' : 'text-[12px]'">{{ deployedSubtitle }}</span>
        </span>
        <span v-if="notice.kind === 'track'" class="waveform np-wave" :class="{ 'is-bar': showsTrackBar }" aria-hidden="true"><WaveBars /></span>
      </div>
      <span v-if="deployedWithBar" class="np-bar block h-[5px] w-full flex-none overflow-hidden rounded-full bg-white/15">
        <span class="block h-full rounded-[inherit] bg-white/90 transition-[width] duration-150 ease-out" :style="{ width: `${deployedLevel}%` }"></span>
      </span>
    </div></Transition>

    <!-- Notificación de otra aplicación. Cada una lleva `key` para que el
         contenido se renueve al relevarse por otra aún con la isla alta. -->
    <Transition name="notice-panel"><div v-if="showsToast" :key="`${toastApp}${toastTitle}${toastBody}`" class="absolute inset-0 z-1 isolate flex items-center gap-3.5 px-5" aria-hidden="true">
      <img v-if="toastAvatar" :src="toastAvatar" alt="" class="np-lead size-11 flex-none rounded-full object-cover" />
      <img v-else-if="toastIcon" :src="toastIcon" alt="" class="np-lead size-11 flex-none rounded-full bg-white/12 object-contain p-1.5" />
      <span v-else class="np-lead grid size-11 flex-none place-items-center rounded-full bg-green text-[19px] font-bold text-[#0a0c0a]">{{ toastInitial }}</span>
      <span class="np-copy grid min-w-0 flex-1 gap-1">
        <span class="text-[10px] leading-[1.45] font-semibold tracking-[.06em] text-[#d4d4db] uppercase">{{ toastApp }}</span>
        <!-- Sin cuerpo, el texto único ocupa la fila del mensaje y puede ir a dos
             líneas. Con cuerpo, el mensaje ocupa hasta tres y, si pasa de ahí, sube
             (ToastMarquee): la isla no cambia de tamaño. -->
        <strong
          class="text-[13px] leading-[1.2] text-white/96"
          :class="toastBody
            ? 'truncate font-semibold'
            : 'overflow-hidden font-medium [overflow-wrap:anywhere] [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2]'"
        >{{ toastTitle }}</strong>
        <!-- Un adjunto sin imagen disponible (ver `toastMediaKind`): chip con icono. -->
        <span v-if="toastMedia" class="inline-flex w-fit items-center gap-1.5 rounded-full bg-white/12 py-1 pr-3 pl-2 text-[12px] font-medium text-white/92">
          <svg viewBox="0 0 24 24" class="size-4 flex-none" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <template v-if="toastMedia === 'photo'">
              <rect x="3" y="5" width="18" height="14" rx="3" /><circle cx="9" cy="10.5" r="1.6" /><path d="m4 17 5-4.5 4 3.5 3-2.5 4 3.5" />
            </template>
            <template v-else-if="toastMedia === 'gif'">
              <rect x="3" y="5" width="18" height="14" rx="3" /><path d="M9.5 10.2a2 2 0 1 0 0 3.6V12.2H8.4M12.5 9.5v5M15 14.5v-5h2.5M15 12h2" />
            </template>
            <template v-else-if="toastMedia === 'voice'">
              <rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21" />
            </template>
            <template v-else>
              <path d="M5 4h9l5 5v8a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3Z" /><path d="M14 4v3a2 2 0 0 0 2 2h3" /><path d="M9 14.5c1 1.2 4 1.2 5 0" />
            </template>
          </svg>
          {{ toastMedia === "sticker" ? "Sticker" : toastMedia === "gif" ? "GIF" : toastMedia === "voice" ? "Mensaje de voz" : "Foto" }}<span v-if="toastMediaDetail" class="tabular-nums text-white/60">{{ toastMediaDetail }}</span>
        </span>
        <ToastMarquee v-else-if="toastBody" :text="toastBody" />
      </span>
    </div></Transition>

    <Transition name="collapsed"><div v-if="showsPill && !showsCharge && !showsToast && !showsDeployed && !showsTab" class="island-collapsed" aria-hidden="true">
      <!-- Un aviso manda sobre el modo: interrumpe, se muestra unos segundos
           y al retirarse la isla recupera lo que estaba enseñando. -->
      <template v-if="notice">
        <span
          class="notice-icon grid size-5 flex-none place-items-center rounded-full [&>svg]:size-[13px]"
          :class="[`notice-${notice.kind}`, noticeTone(notice.kind), { 'is-charging': notice.data?.plugged }]"
          :style="{ '--pct': Number(notice.data?.percent ?? 0) }"
        >
          <NoticeIcon :kind="notice.kind" :muted="notice.data?.muted === true" />
        </span>
        <span class="text-[12px] font-semibold whitespace-nowrap text-white/92 tabular-nums">{{ notice.label }}</span>
      </template>
      <template v-else-if="mode === 'home'">
        <span class="text-[12px] font-[650] tabular-nums text-white/90">{{ homeTime }}</span>
      </template>
      <template v-else-if="mode === 'idle'">
        <span :class="[ALBUM, 'size-[30px] rounded-lg bg-[linear-gradient(145deg,#3b4b45,#18221f)] text-[13px] text-[#b7f36b] [&>svg]:size-[17px]']"><IconDots /></span>
        <span class="waveform idle-wave"><i></i><i></i><i></i></span>
      </template>
      <template v-else-if="mode === 'ring'">
        <span :class="RING_PULSE"><IconPhone /></span>
        <span class="text-[10px] font-semibold">Alex</span>
      </template>
      <template v-else-if="mode === 'record'">
        <span class="size-[9px] animate-[record-pulse_1.3s_ease-in-out_infinite] rounded-full bg-[#ff5b67] shadow-[0_0_10px_rgba(255,91,103,.65)]"></span>
        <span class="text-[12px] font-[650] tabular-nums text-[#f4cb82]">{{ formatTime(recordingSeconds) }}</span>
      </template>
      <template v-else-if="mode === 'music'">
        <span :class="[ALBUM, ALBUM_ART_BG, 'size-5 rounded-[7px] text-[9px]']">
          <ArtSwap :src="trackArtwork" :initial="trackInitial" />
        </span>
        <span class="waveform"><WaveBars /></span>
      </template>
      <template v-else-if="mode === 'timer'">
        <IconClock class="size-[15px] text-[#f4cb82]" />
        <span class="text-[12px] font-[650] tabular-nums text-[#f4cb82]">{{ timerLabel }}</span>
      </template>
      <template v-else-if="mode === 'call'">
        <span class="size-2 rounded-full bg-green shadow-[0_0_8px_rgba(183,243,107,.7)]"></span>
        <span class="text-[12px] font-[650] tabular-nums text-[#f4cb82]">{{ callLabel }}</span>
      </template>
      <template v-else-if="mode === 'airdrop'">
        <IconArrowDownToArc class="size-[15px] text-white" />
        <span class="text-[12px] font-[650] tabular-nums text-[#f4cb82]">68%</span>
      </template>
      <template v-else>
        <IconBell class="size-[15px] text-white" />
        <span class="text-[10px] font-semibold">1 nuevo</span>
      </template>
    </div></Transition>
  
    <Transition name="content"><div v-if="!showsPill && !showsPanel" v-auto-animate class="island-content">
      <template v-if="expandedMode === 'home'">
        <div class="relative h-full">
          <!-- Carrusel: cada indicador pasa a la página de su lado, y al llegar al final da la
               vuelta (de la última a la primera). Siempre hay dos mientras haya más de una página. -->
          <template v-if="homePages.length > 1">
            <button
              :class="NAV_ARROW + ' left-0'"
              :aria-label="'Ver ' + homePageName(-1)"
              @click.stop="rotateHome(-1)"
            ><IconChevronLeft class="size-4" /></button>
            <button
              :class="NAV_ARROW + ' right-0'"
              :aria-label="'Ver ' + homePageName(1)"
              @click.stop="rotateHome(1)"
            ><IconChevronRight class="size-4" /></button>
          </template>

          <Transition name="txt-swap" mode="out-in">
        <div :key="homePage" class="flex h-full flex-col items-center justify-center gap-1.5 px-8">
          <template v-if="homePage === 'clock'">
            <!-- Con el tiempo activo, la hora (con la fecha debajo) y, a su lado, el icono del cielo con
                 la temperatura en grande. Sin él, la hora sola con su fecha. Con segundos y tiempo a la
                 vez la hora baja un poco de tamaño para que todo quepa en una fila. -->
            <div class="flex items-stretch justify-center" :class="weather ? 'gap-4' : ''">
              <!-- Hora y fecha. Cada lado tiene dos filas del mismo alto, para que alineen: la hora con la
                   temperatura y la fecha con la línea de debajo del tiempo. -->
              <div class="grid flex-none content-between justify-items-center" :class="weather ? 'gap-1' : 'gap-1.5'">
                <strong class="leading-none font-semibold tracking-[-.02em] tabular-nums" :class="weather && homeSeconds !== false ? 'text-[34px]' : 'text-[40px]'">{{ homeTime }}</strong>
                <span class="max-w-full truncate text-[11px] text-[#a2a2a8]">{{ homeDate }}</span>
              </div>
              <!-- Línea tenue entre la hora y el tiempo. -->
              <span v-if="weather" class="w-px flex-none self-stretch bg-white/12" aria-hidden="true"></span>
              <div v-if="weather" class="grid flex-none content-between justify-items-center gap-1">
                <div class="flex items-center gap-2" :class="homeSeconds !== false ? 'h-[34px]' : 'h-[40px]'">
                  <component :is="weatherIcon" class="size-7" :class="weatherTone" stroke-width="1.8" />
                  <strong class="text-[24px] leading-none font-semibold tabular-nums">{{ weather.temp }}°</strong>
                </div>
                <!-- Una sola línea, a la altura de la fecha: la máxima y la mínima del día. Sin ellas, la descripción.
                     No hay descripción escrita ("Nublado"): con la fecha y el tiempo en una fila no cabe más sin estrechar
                     la fecha, y el icono ya dice cómo está el cielo. -->
                <span class="flex items-center gap-2 text-[11px] text-white/70 tabular-nums">
                  <span v-if="weather.max === null || weather.min === null" class="max-w-[100px] truncate">{{ weather.text }}</span>
                  <template v-if="weather.max !== null && weather.min !== null">
                    <span>↑ {{ weather.max }}°</span><span>↓ {{ weather.min }}°</span>
                  </template>
                </span>
              </div>
            </div>
          </template>

          <!-- Página de la izquierda: CPU y memoria. -->
          <template v-else-if="homePage === 'system'">
            <span class="text-[9px] font-semibold tracking-[.1em] text-white/45 uppercase">Sistema</span>
            <div v-if="homeStats" class="grid w-full gap-2 text-[11px] tabular-nums">
              <div v-for="row in [{ name: 'CPU', value: homeStats.cpu }, { name: 'RAM', value: homeStats.memory }]" :key="row.name" class="flex items-center gap-2.5">
                <span class="w-7 text-[#a2a2a8]">{{ row.name }}</span>
                <span class="block h-1.5 flex-1 overflow-hidden rounded-full bg-white/15">
                  <span class="block h-full rounded-[inherit] bg-white/85 transition-[width] duration-500 ease-out" :style="{ width: row.value + '%' }"></span>
                </span>
                <span class="w-9 text-right text-white/90">{{ row.value }}%</span>
              </div>
              <span class="text-center text-[10px] text-[#8d8d98]">{{ homeStats.memoryUsedGb.toFixed(1).replace('.', ',') }} de {{ homeStats.memoryTotalGb.toFixed(0) }} GB de memoria</span>
            </div>
            <!-- Esqueleto con la MISMA estructura que las filas reales (mismas clases de texto, mismas
                 anchuras de columna), de modo que cada bloque cae donde luego caerá el texto y al llegar los
                 datos nada se mueve. Cada hueco lleva un espacio duro: así la línea de texto mide lo mismo
                 que la real (la altura sale de la línea, no del bloque), y el bloque va centrado encima. -->
            <div v-else class="grid w-full gap-2 text-[11px] tabular-nums" aria-busy="true" aria-label="Midiendo CPU y memoria">
              <div v-for="row in 2" :key="row" class="flex items-center gap-2.5">
                <span class="relative w-7">&nbsp;<span class="skeleton absolute top-1/2 left-0 h-[9px] w-6 -translate-y-1/2"></span></span>
                <span class="skeleton block h-1.5 flex-1 rounded-full"></span>
                <span class="relative w-9 text-right">&nbsp;<span class="skeleton absolute top-1/2 right-0 h-[9px] w-6 -translate-y-1/2"></span></span>
              </div>
              <span class="relative text-center text-[10px]">&nbsp;<span class="skeleton absolute top-1/2 left-1/2 h-[8px] w-36 -translate-x-1/2 -translate-y-1/2"></span></span>
            </div>
          </template>

          <!-- Página de la derecha: test de velocidad. Solo bajada y latencia (ver speedtest.rs). -->
          <template v-else-if="homePage === 'speed'">
            <template v-if="speed?.status === 'running' || speed?.status === 'done'">
              <!-- Una sola cifra, la de bajada, grande y centrada. Mientras mide, va en directo; antes de
                   la primera lectura es un esqueleto del tamaño de la cifra. -->
              <div class="grid justify-items-center gap-2">
                <span class="flex items-center gap-1 text-[9px] font-semibold tracking-[.1em] text-white/45 uppercase">
                  <IconArrowDown class="size-3" stroke-width="2.6" />Bajada
                </span>
                <span v-if="speed.down === null && speed.status === 'running'" class="skeleton h-[38px] w-[130px]" aria-label="Midiendo"></span>
                <strong v-else class="flex items-baseline text-[38px] leading-none font-semibold tabular-nums">
                  {{ speed.down === null ? '—' : formatMbps(speed.down) }}<small class="ml-1.5 text-[13px] font-medium text-white/50">Mbps</small>
                </strong>
              </div>
              <span class="flex items-center gap-3 text-[10px] text-[#8d8d98]">
                <template v-if="speed.status === 'running'">{{ speedStatusText }}</template>
                <template v-else>
                  <span v-if="speed.ping !== null" class="tabular-nums">Latencia {{ Math.round(speed.ping) }} ms</span>
                  <button class="flex items-center gap-1 text-white/55 transition-colors hover:text-white" @click.stop="emit('speedtest')"><IconRefresh class="size-3" />Repetir</button>
                </template>
              </span>
            </template>
            <template v-else>
              <button class="flex items-center gap-2 rounded-full bg-white/10 py-2 pr-4 pl-3 text-[13px] font-medium text-white transition-colors hover:bg-white/16" @click.stop="emit('speedtest')">
                <IconGauge class="size-[18px]" />{{ speed?.status === 'error' ? 'Reintentar' : 'Probar velocidad' }}
              </button>
              <span class="text-[10px] text-[#8d8d98]">{{ speed?.status === 'error' ? (speed.reason === 'limit' ? 'Demasiadas pruebas seguidas: espera unos minutos' : 'No se pudo medir: revisa tu conexión') : 'Mide la bajada y la latencia' }}</span>
            </template>
          </template>
        </div>
          </Transition>
        </div>
      </template>

      <template v-else-if="mode === 'idle'">
        <div :class="TOP">
          <span :class="[AVATAR, 'border-[rgba(183,243,107,.15)] bg-[linear-gradient(145deg,#2c3b35,#171d1b)] text-[#b7f36b]']"><IconDots /></span>
          <span :class="COPY"><strong :class="COPY_STRONG">Todo tranquilo</strong><small :class="COPY_SMALL">La isla está lista para ti</small></span>
          <span class="text-[7px] font-bold tracking-[.08em] text-[#92929c]">EN ESPERA</span>
        </div>
        <div class="flex items-center justify-between text-[9px] text-[#8d8d98]"><span>Sin actividades activas</span><span class="size-1.5 rounded-full bg-[#777782]"></span></div>
      </template>
  
      <template v-else-if="mode === 'ring'">
        <div :class="TOP">
          <span :class="[AVATAR, CONTACT_AVATAR]">A</span>
          <span :class="COPY"><strong :class="COPY_STRONG">Alex Morgan</strong><small class="truncate text-[8px] text-[#9de6b5]">Llamada entrante · móvil</small></span>
          <span :class="RING_PULSE"><IconPhone /></span>
        </div>
        <div class="flex items-center justify-center gap-[19px] text-[9px] text-[#9999a8]">
          <button class="grid size-8 place-items-center rounded-full bg-[#f04e58] text-white rotate-[135deg] [&>svg]:size-4" aria-label="Rechazar llamada" @click="chooseActivity('idle')"><IconPhone /></button>
          <span>Desliza para responder</span>
          <button class="grid size-8 place-items-center rounded-full bg-[#42bb70] text-white rotate-[225deg] [&>svg]:size-4" aria-label="Responder llamada" @click="chooseActivity('call')"><IconPhone /></button>
        </div>
      </template>
  
      <template v-else-if="mode === 'record'">
        <div :class="TOP">
          <span :class="[AVATAR, 'border-[rgba(255,91,103,.18)] bg-[rgba(255,91,103,.1)] text-[#ff727b]']"><IconMicrophone /></span>
          <span :class="COPY"><strong :class="COPY_STRONG">Grabación de audio</strong><small :class="COPY_SMALL">{{ recording ? "Grabando ahora" : "Grabación en pausa" }}</small></span>
          <span class="text-[11px] tabular-nums text-[#ff7881]">{{ formatTime(recordingSeconds) }}</span>
        </div>
        <div class="flex items-center gap-[15px]">
          <div class="recording-wave" :class="{ paused: !recording }"><i v-for="bar in 25" :key="bar" :style="{ '--bar': `${20 + ((bar * 31) % 70)}%` }"></i></div>
          <button :class="[MC, MC_HOVER, PLAY]" :aria-label="recording ? 'Pausar grabación' : 'Reanudar grabación'" @click="emit('toggle-recording')">
            <IconPlayerPauseFilled v-if="recording" />
            <IconPlayerPlayFilled v-else />
          </button>
        </div>
      </template>
  
      <template v-else-if="mode === 'airdrop'">
        <div :class="TOP">
          <span :class="[AVATAR, 'border-[rgba(141,216,255,.18)] bg-[rgba(91,165,204,.12)] text-[#8dd8ff]']"><IconArrowDownToArc /></span>
          <span :class="COPY"><strong :class="COPY_STRONG">Recibiendo archivo</strong><small :class="COPY_SMALL">MacBook Pro · ahora mismo</small></span>
          <button :class="DISMISS" aria-label="Cerrar transferencia" @click="chooseActivity('idle')"><IconX /></button>
        </div>
        <div class="flex justify-between text-[9px] text-[#d6d6de]"><span>Presentación-final.pdf</span><strong class="text-[9px] text-[#8dd8ff]">68%</strong></div>
        <div class="h-1 overflow-hidden rounded-[4px] bg-white/14"><span class="block h-full w-[68%] rounded-[inherit] bg-[linear-gradient(90deg,#7cc9f2,#b6e5ff)]"></span></div>
      </template>
  
      <template v-else-if="expandedMode === 'music'">
        <div class="flex h-full min-w-0 items-stretch gap-3">
        <div class="flex h-full w-[304px] min-w-0 flex-none flex-col justify-between">
          <div class="flex items-center gap-2.5">
            <span :class="[ALBUM, ALBUM_ART_BG, 'size-[34px] rounded-[5px] text-[14px]']">
              <ArtSwap :src="trackArtwork" :initial="trackInitial" />
            </span>
            <span :class="COPY">
              <!-- Cada línea se desenfoca y se desvanece antes de dar paso a la nueva. -->
              <Transition name="txt-swap" mode="out-in"><strong :key="trackTitle" :class="COPY_STRONG">{{ trackTitle }}</strong></Transition>
              <Transition name="txt-swap" mode="out-in"><small :key="trackSubtitle" :class="COPY_SMALL">{{ trackSubtitle }}</small></Transition>
            </span>
            <!-- Acciones de arriba, siempre en el mismo orden y con su sitio reservado:
                 letra, volumen y la onda. La letra no desaparece sin letra: se queda
                 atenuada y sin respuesta, para que nada se mueva al cambiar de pista.
                 No se distingue YouTube Music de un vídeo de YouTube (SMTC no lo dice):
                 donde LRCLIB no tiene letra, el botón queda bloqueado. -->
            <div class="ml-auto flex flex-none items-center gap-0.5">
              <button
                :class="[
                  ICON_ACTION,
                  lyricsAvailable
                    ? lyricsOpen
                      ? 'text-green hover:bg-white/10'
                      : 'text-[#9a9aa4] hover:bg-white/10 hover:text-[#f0f0f4]'
                    : 'cursor-default text-[#9a9aa4] opacity-30',
                ]"
                :disabled="!lyricsAvailable"
                :aria-label="lyricsOpen ? 'Ocultar letra' : 'Mostrar letra'"
                :aria-pressed="lyricsOpen"
                @click.stop="emit('toggle-lyrics')"
              >
                <IconMicrophone2 />
              </button>
              <!-- El deslizador de volumen flota a la izquierda del botón en lugar de
                   empujar a sus vecinos: así el espacio de cada acción no cambia al
                   pasar el ratón. Va dentro del grupo para que el ratón no lo suelte
                   al cruzar de un elemento a otro. -->
              <div class="group/vol relative flex items-center" role="presentation" @click.stop>
                <button
                  :class="[ICON_ACTION, 'text-[#9a9aa4] hover:bg-white/10 hover:text-[#f0f0f4]']"
                  :aria-label="muted ? 'Activar sonido' : 'Silenciar'"
                  @click="emit('toggle-mute')"
                >
                  <IconVolumeOff v-if="muted || volume === 0" />
                  <IconVolume2 v-else />
                </button>
                <div
                  class="pointer-events-none absolute top-1/2 right-full flex -translate-y-1/2 items-center rounded-full bg-[#0c0c0e]/95 py-1 pr-1 pl-3 opacity-0 shadow-[0_2px_10px_rgba(0,0,0,.5)] transition-opacity duration-150 group-focus-within/vol:pointer-events-auto group-focus-within/vol:opacity-100 group-hover/vol:pointer-events-auto group-hover/vol:opacity-100"
                >
                  <input
                    class="w-[64px] cursor-pointer accent-[#f4f4f6]"
                    type="range"
                    min="0"
                    max="100"
                    step="1"
                    :value="muted ? 0 : volume"
                    aria-label="Volumen"
                    @input="emit('volume', Number(($event.target as HTMLInputElement).value))"
                  />
                </div>
              </div>
            </div>

            <!-- Cinco barras: el patrón de alturas (5-10-15-10-5) es simétrico
                 y el CSS las estiliza por posición, así que el número importa. -->
            <span class="waveform heading-wave" :class="{ 'is-idle': !playing }" aria-hidden="true">
              <WaveBars />
            </span>
          </div>

          <div class="flex items-center gap-2">
            <span class="flex-none text-[8.5px] tabular-nums text-[#9999a0]">{{ positionLabel }}</span>
            <div
              class="group/seek min-w-0 flex-1 touch-none py-1 outline-none"
              :class="canSeek ? 'cursor-pointer' : 'cursor-default'"
              role="slider"
              tabindex="0"
              aria-label="Posición de la pista"
              :aria-valuemin="0"
              :aria-valuemax="100"
              :aria-valuenow="Math.round(trackProgress)"
              @pointerdown="canSeek && startScrub($event)"
              @keydown="onSeekKeydown"
            >
              <!-- `track-progress` no tiene estilos: es el asa con que `startScrub` localiza la barra. -->
              <div
                class="track-progress relative h-[3px] rounded-[4px] bg-white/17 transition-[height] duration-150"
                :class="canSeek && 'group-hover/seek:h-[5px] group-focus-visible/seek:h-[5px]'"
              >
                <span class="block h-full rounded-[inherit] bg-[#f4f4f6]" :style="{ width: `${trackProgress}%` }"></span>
                <i
                  class="pointer-events-none absolute top-1/2 -ml-[4.5px] size-[9px] -translate-y-1/2 scale-60 rounded-full bg-white opacity-0 transition-[opacity,scale] duration-150 group-hover/seek:scale-100 group-hover/seek:opacity-100 group-focus-visible/seek:scale-100 group-focus-visible/seek:opacity-100"
                  :class="!canSeek && 'hidden'"
                  :style="{ left: `${trackProgress}%` }"
                ></i>
              </div>
            </div>
            <span class="flex-none text-[8.5px] tabular-nums text-[#9999a0]">{{ remainingLabel }}</span>
          </div>

            <!-- Las flechas del carrusel van aquí, en los extremos de la fila de los controles: los cinco botones
                 solo ocupan el centro, así que sobra sitio sin quitárselo a nada. En el centro de la altura
                 chocarían con los tiempos de la barra de progreso. -->
            <div class="relative flex items-center justify-center">
              <template v-if="carousel && homePages.length > 1">
                <button :class="NAV_ARROW + ' left-0'" :aria-label="'Ver ' + homePageName(-1)" @click.stop="rotateHome(-1)"><IconChevronLeft class="size-4" /></button>
                <button :class="NAV_ARROW + ' right-0'" :aria-label="'Ver ' + homePageName(1)" @click.stop="rotateHome(1)"><IconChevronRight class="size-4" /></button>
              </template>
              <div class="flex items-center justify-center gap-[9px]">
                <button :class="[MC, MC_HOVER, CONTROL]" aria-label="Pista anterior" @click="emit('previous')"><IconPlayerTrackPrevFilled /></button>
                <button
                  :class="[MC, SKIP, canSeek ? 'hover:scale-[1.12] hover:text-white' : 'cursor-default opacity-30']"
                  :disabled="!canSeek"

                  :aria-label="`Retroceder ${skipSeconds} segundos`"
                  @click.stop="emit('skip', -skipSeconds)"
                >
                  <SkipIcon :seconds="skipSeconds" back />
                </button>
                <button :class="[MC, MC_HOVER, PLAY]" :aria-label="playing ? 'Pausar' : 'Reproducir'" @click="emit('toggle-play')">
                  <IconPlayerPauseFilled v-if="playing" />
                  <IconPlayerPlayFilled v-else />
                </button>
                <button
                  :class="[MC, SKIP, canSeek ? 'hover:scale-[1.12] hover:text-white' : 'cursor-default opacity-30']"
                  :disabled="!canSeek"
                  :aria-label="`Avanzar ${skipSeconds} segundos`"
                  @click.stop="emit('skip', skipSeconds)"
                >
                  <SkipIcon :seconds="skipSeconds" />
                </button>
                <button :class="[MC, MC_HOVER, CONTROL]" aria-label="Pista siguiente" @click="emit('next')"><IconPlayerTrackNextFilled /></button>
              </div>
            </div>
        </div>

        <!-- Panel de karaoke: la línea en curso destacada, el resto atenuado. -->
        <div
          v-if="lyricsOpen && lyricsAvailable"
          v-motion
          class="lyrics-panel"
          :initial="{ opacity: 0, x: -16 }"
          :enter="{
            opacity: 1,
            x: 0,
            transition: { type: 'spring', stiffness: 220, damping: 26, mass: 0.7 },
          }"
          :leave="{ opacity: 0, x: -16, transition: { duration: 180 } }"
        >
          <!-- Transiciones CSS y no `v-motion`: este solo aplica su `enter` al
               montar, así que una línea que pasa a ser la activa se quedaba
               atenuada. Con TransitionGroup las líneas además suben una a
               una (FLIP) al avanzar la letra. -->
          <TransitionGroup name="lyric" tag="div" class="lyric-list">
            <div
              v-for="line in lyricLines"
              :key="`${line.time}-${line.text}`"
              class="lyric-line"
              :class="{ 'is-active': line.active }"
            >
              {{ line.text }}
            </div>
          </TransitionGroup>
        </div>
        </div>
      </template>
  
      <template v-else-if="mode === 'timer'">
        <div :class="TOP">
          <span class="grid size-[43px] flex-none place-items-center rounded-full border-2 border-[rgba(244,203,130,.75)] text-[#f4cb82] [&>svg]:size-[19px]"><IconClock /></span>
          <span :class="COPY"><strong :class="COPY_STRONG">Temporizador</strong><small :class="COPY_SMALL">{{ timerRunning ? "Cuenta atrás en curso" : "En pausa" }}</small></span>
          <button
            :class="[MC, MC_HOVER, PLAY]"
            :aria-label="timerRunning ? 'Pausar temporizador' : 'Reanudar temporizador'"
            @click="emit('toggle-timer')"
          >
            <IconPlayerPauseFilled v-if="timerRunning" />
            <IconPlayerPlayFilled v-else />
          </button>
        </div>
        <div class="flex items-center justify-between text-[9px] tracking-[.08em] text-[#9999a0]"><span>COCINA</span><strong class="text-[11px] tracking-normal text-green tabular-nums">{{ timerLabel }}</strong></div>
      </template>
  
      <template v-else-if="mode === 'call'">
        <div :class="TOP">
          <span :class="[AVATAR, CONTACT_AVATAR]">A</span>
          <span :class="COPY"><strong :class="COPY_STRONG">Alex Morgan</strong><small :class="COPY_SMALL">{{ callActive ? `En llamada · ${callLabel}` : "Llamada finalizada" }}</small></span>
          <span class="flex items-center gap-[5px] text-[7px] font-bold tracking-[.07em] whitespace-nowrap text-green"><i class="size-[5px] rounded-full bg-green"></i> {{ callActive ? "ACTIVA" : "FINALIZADA" }}</span>
        </div>
        <div v-if="callActive" class="flex justify-center gap-6">
          <button :class="[MC, MC_HOVER, CALL_CONTROL]" :aria-label="muted ? 'Activar micrófono' : 'Silenciar llamada'" @click="emit('toggle-mute')">
            <IconMicrophone v-if="!muted" />
            <IconX v-else />
          </button>
          <button :class="[MC, 'size-[33px] rotate-[135deg] bg-[#fa453d] text-white hover:bg-[#dc342e] [&>svg]:size-[17px]']" aria-label="Finalizar llamada" @click="emit('end-call')"><IconPhone /></button>
          <button :class="[MC, MC_HOVER, CALL_CONTROL]" aria-label="Activar altavoz"><IconVolume2 /></button>
        </div>
        <div v-else class="grid justify-items-center gap-[7px] self-center text-center text-[9px] text-[#9999a8]">
          Llamada finalizada
          <button class="rounded-[7px] border border-[rgba(183,243,107,.2)] bg-[rgba(183,243,107,.06)] px-[9px] py-[5px] text-[9px] text-green" @click="callActive = true">Volver a llamar</button>
        </div>
      </template>
  
      <template v-else>
        <div :class="TOP">
          <span :class="[AVATAR, 'border-white/15 bg-[linear-gradient(145deg,#ffa784,#ee5970)] text-[16px] font-semibold text-white']"><IconMessage /></span>
          <span :class="COPY"><strong :class="COPY_STRONG">Alex · Mensajes</strong><small :class="COPY_SMALL">ahora mismo</small></span>
          <button :class="DISMISS" aria-label="Cerrar notificación" @click="chooseActivity('music')"><IconX /></button>
        </div>
        <p class="mt-1 ml-[53px] text-[10px] leading-[1.5] text-[#dddde2]">¡Ya llegué! Te espero en la entrada 😊</p>
        <button class="self-end p-0 text-[9px] text-green" @click="chooseActivity('music')">Marcar como leído <span class="ml-[5px]">↗</span></button>
      </template>
    </div></Transition>
  </div>
  
</template>
