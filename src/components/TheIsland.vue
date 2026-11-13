<script setup lang="ts">
/**
 * La isla en sí: píldora que se expande y muestra la actividad en curso.
 *
 * Se usa en dos sitios: dentro del escritorio simulado de la demo web y como
 * ventana flotante en el escritorio real, por eso vive en su propio componente.
 */
import {
  IconArrowDownToArc,
  IconBell,
  IconBolt,
  IconClock,
  IconDots,
  IconMessage,
  IconMicrophone,
  IconMicrophone2,
  IconPhone,
  IconPlayerPause,
  IconPlayerPlay,
  IconPlayerTrackNext,
  IconPlayerTrackPrev,
  IconRewindBackward5,
  IconRewindBackward10,
  IconRewindBackward15,
  IconRewindBackward30,
  IconRewindForward5,
  IconRewindForward10,
  IconRewindForward15,
  IconRewindForward30,
  IconVolume2,
  IconVolumeOff,
  IconX,
} from "@tabler/icons-vue";
import { computed, onMounted, onUnmounted, ref } from "vue";
import NoticeIcon from "@/components/NoticeIcon.vue";
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
  new Intl.DateTimeFormat("es", { weekday: "long", day: "numeric", month: "long" }).format(now.value),
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
const deployedTitle = computed(() => String(props.notice?.data?.title ?? props.notice?.label ?? ""));
const deployedSubtitle = computed(() => String(props.notice?.data?.subtitle ?? ""));
/** Nivel de 0 a 100 para la barra (volumen, batería); `null` si el aviso no lleva. */
const deployedLevel = computed(() => {
  const level = props.notice?.data?.level;
  return typeof level === "number" ? Math.min(100, Math.max(0, level)) : null;
});
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

/** Iconos de salto disponibles, por segundos. */
const SKIP_ICONS = {
  5: [IconRewindBackward5, IconRewindForward5],
  10: [IconRewindBackward10, IconRewindForward10],
  15: [IconRewindBackward15, IconRewindForward15],
  30: [IconRewindBackward30, IconRewindForward30],
} as const;

/** El icono refleja los segundos configurados; 10 s si el valor no encaja. */
const skipIcons = computed(() => {
  const key = props.skipSeconds as keyof typeof SKIP_ICONS;
  return SKIP_ICONS[key] ?? SKIP_ICONS[10];
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
};
const noticeTone = (kind: string) => NOTICE_TONE[kind] ?? "";
const RING_PULSE = "grid size-6 flex-none animate-[ring-pulse_1.2s_ease-in-out_infinite] place-items-center rounded-full bg-[#46bd73] text-white [&>svg]:size-[13px]";
const TOP = "flex min-w-0 items-center justify-start gap-[9px]";
const COPY = "grid min-w-0 flex-1 gap-0.5";
const COPY_STRONG = "truncate text-[11px] font-[650]";
const COPY_SMALL = "truncate text-[8px] text-[#a2a2a8]";
const CONTACT_AVATAR = "border-white/15 bg-[linear-gradient(145deg,#f8d7aa,#b17aa2_58%,#5a477f)] text-[16px] font-semibold text-white";
const PLAY = "size-[30px] text-[#f0f0f4] [&>svg]:size-5";
const DISMISS = "grid size-[25px] place-items-center rounded-full text-[#92929c] [&>svg]:size-3.5";
const CONTROL = "size-[26px] text-[#f0f0f4] [&>svg]:size-[17px]";
const SKIP = "size-[22px] text-[#b4b4bd] [&>svg]:size-[15px]";
const CALL_CONTROL = "size-[33px] text-[#f0f0f4] [&>svg]:size-[17px]";
const AVATAR = "grid size-[42px] flex-none place-items-center rounded-[13px] border [&>svg]:size-[19px]";
const MC = "grid flex-none place-items-center rounded-full transition-[color,scale] duration-[160ms] [&>svg]:stroke-2";
const MC_HOVER = "hover:scale-[1.12] hover:text-white";
const ICON_ACTION = "grid size-5 place-items-center rounded-md p-0 transition-[color,background-color] duration-150 [&>svg]:size-3.5";
const ALBUM = "grid flex-none place-items-center overflow-hidden border border-white/12 font-bold shadow-[inset_0_0_20px_rgba(44,23,60,.2)]";
const ALBUM_ART_BG = "bg-[radial-gradient(circle_at_70%_26%,#f6d29a_0_12%,transparent_13%),linear-gradient(145deg,#f0bc88,#a85c76_55%,#4c397a)] text-white/92";
const ALBUM_IMG = "block size-full object-cover [image-rendering:auto]";

const emit = defineEmits<{
  (e: "island-click", event: MouseEvent): void;
  (e: "island-keydown", event: KeyboardEvent): void;
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
        'is-tab': showsTab,
        'is-paused': !playing,
      },
      `mode-${mode}`,
      { 'has-lyrics': expanded && lyricsOpen && lyricsAvailable },
      showsTab
        ? 'w-[48px] h-[8px] rounded-[4px]'
        : showsPill && !showsCharge && !showsDeployed && !showsToast
          ? 'h-[34px] rounded-[18px]'
          : 'w-[330px] h-[108px] rounded-[24px]',
    ]"
    :style="compactWidth"
    role="button"
    tabindex="0"
    :aria-label="`Isla dinámica, ${expanded ? 'contraer' : 'expandir'}`"
    :title="showsTab ? 'Abrir la isla' : undefined"
    :aria-expanded="expanded"
    @click="handleIslandClick"
    @keydown="handleIslandKeydown"
  >
    <!-- Fondo tintado por la carátula, compartido por ambas vistas. -->
    <div
      v-if="mode === 'music' && trackArtwork"
      class="art-backdrop"
      :style="{ '--art': `url(${trackArtwork})` }"
      aria-hidden="true"
    ></div>

    <!-- Batería cargando: fuera de `.island-content` por lo mismo que la franja,
         `v-auto-animate` pelearía con su animación. -->
    <div
      v-if="showsCharge"
      class="absolute inset-0 z-1 isolate flex animate-[island-fade-in_.3s_ease-out_.16s_both] items-center justify-center gap-[26px]"
      :style="{ '--pct': chargePercent }"
      aria-hidden="true"
    >
      <div class="flex items-center">
        <div class="relative grid h-[46px] w-[92px] place-items-center overflow-hidden rounded-[13px] border-2 border-white/70">
          <div class="charge-fill"></div>
          <IconBolt class="relative z-1 size-[22px] animate-[charge-pulse_1.1s_ease-in-out_.4s_infinite] text-[#0a0c0a]" />
        </div>
        <span class="ml-[3px] h-[18px] w-[5px] rounded-[0_3px_3px_0] bg-white/70"></span>
      </div>
      <div class="flex flex-col gap-px">
        <strong class="charge-percent text-[34px] leading-none font-bold tabular-nums"></strong>
        <small class="text-[12px] font-semibold tracking-[.02em] text-green">Cargando</small>
      </div>
    </div>

    <!-- Aviso del sistema desplegado. La `key` es solo el tipo: un aviso del mismo
         tipo (el volumen, que cambia con cada pulsación) actualiza el texto y la
         barra en el sitio. Si la `key` incluyera el valor, cada pulsación
         remontaría el panel y repetiría la animación de entrada, y tres
         pulsaciones se verían como tres avisos. -->
    <div
      v-if="showsDeployed && notice"
      :key="notice.kind"
      class="absolute inset-0 z-1 isolate flex animate-[island-fade-in_.3s_ease-out_.16s_both] items-center gap-4 px-6"
      aria-hidden="true"
    >
      <span
        class="notice-icon grid size-14 flex-none place-items-center rounded-full [&>svg]:size-7"
        :class="[`notice-${notice.kind}`, noticeTone(notice.kind)]"
      >
        <NoticeIcon :kind="notice.kind" :muted="notice.data?.muted === true" />
      </span>
      <span class="grid min-w-0 flex-1 gap-1">
        <strong class="truncate text-[18px] leading-[1.15] font-semibold text-white/96">{{ deployedTitle }}</strong>
        <span v-if="deployedSubtitle" class="truncate text-[12px] text-white/60 tabular-nums">{{ deployedSubtitle }}</span>
        <span v-if="deployedLevel !== null" class="mt-1 block h-1.5 overflow-hidden rounded-full bg-white/15">
          <span class="block h-full rounded-[inherit] bg-white/90 transition-[width] duration-150 ease-out" :style="{ width: `${deployedLevel}%` }"></span>
        </span>
      </span>
    </div>

    <!-- Notificación de otra aplicación. Cada una lleva `key` para que el
         contenido se renueve al relevarse por otra aún con la isla alta. -->
    <div v-if="showsToast" :key="`${toastApp}${toastTitle}${toastBody}`" class="absolute inset-0 z-1 isolate flex animate-[island-fade-in_.3s_ease-out_.14s_both] items-center gap-3.5 px-5" aria-hidden="true">
      <img v-if="toastAvatar" :src="toastAvatar" alt="" class="size-11 flex-none rounded-full object-cover" />
      <img v-else-if="toastIcon" :src="toastIcon" alt="" class="size-11 flex-none rounded-full bg-white/12 object-contain p-1.5" />
      <span v-else class="grid size-11 flex-none place-items-center rounded-full bg-green text-[19px] font-bold text-[#0a0c0a]">{{ toastInitial }}</span>
      <span class="grid min-w-0 flex-1 gap-1">
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
        <ToastMarquee v-if="toastBody" :text="toastBody" />
      </span>
    </div>

    <div v-if="showsPill && !showsCharge && !showsToast && !showsDeployed && !showsTab" class="island-collapsed" aria-hidden="true">
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
          <img v-if="trackArtwork" :src="trackArtwork" alt="" :class="ALBUM_IMG" />
          <template v-else>{{ trackInitial }}</template>
        </span>
        <span class="waveform"><i></i><i></i><i></i><i></i><i></i></span>
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
    </div>
  
    <div v-if="!showsPill && !showsPanel" v-auto-animate class="island-content">
      <template v-if="mode === 'home'">
        <div class="flex h-full flex-col items-center justify-center gap-1.5">
          <strong class="text-[40px] leading-none font-semibold tracking-[-.02em] tabular-nums">{{ homeTime }}</strong>
          <span class="truncate text-[11px] text-[#a2a2a8] first-letter:uppercase">{{ homeDate }}</span>
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
            <IconPlayerPause v-if="recording" />
            <IconPlayerPlay v-else />
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
  
      <template v-else-if="mode === 'music'">
        <div class="flex h-full min-w-0 items-stretch gap-3">
        <div class="flex h-full w-[300px] min-w-0 flex-none flex-col justify-between gap-1.5">
          <div class="flex items-center gap-[9px]">
            <span :class="[ALBUM, ALBUM_ART_BG, 'size-[38px] rounded-[9px] text-[15px]']">
              <img v-if="trackArtwork" :src="trackArtwork" alt="" :class="ALBUM_IMG" />
              <template v-else>{{ trackInitial }}</template>
            </span>
            <span :class="COPY">
              <strong :class="COPY_STRONG">{{ trackTitle }}</strong>
              <small :class="COPY_SMALL">{{ trackSubtitle }}</small>
            </span>
            <!-- Cinco barras: el patrón de alturas (5-10-15-10-5) es simétrico
                 y el CSS las estiliza por posición, así que el número importa. -->
            <span class="waveform heading-wave" :class="{ 'is-idle': !playing }" aria-hidden="true">
              <i></i><i></i><i></i><i></i><i></i>
            </span>

            <!-- Karaoke y volumen viven arriba: la fila de controles ya está
                 ocupada por reproducción, saltos y cambio de pista. -->
            <div class="flex flex-none items-center gap-1">
              <button
                v-if="lyricsAvailable"
                :class="[ICON_ACTION, lyricsOpen ? 'text-green hover:bg-white/10' : 'text-[#9a9aa4] hover:bg-white/10 hover:text-[#f0f0f4]']"
                :aria-label="lyricsOpen ? 'Ocultar letra' : 'Mostrar letra'"
                :aria-pressed="lyricsOpen"
                @click.stop="emit('toggle-lyrics')"
              >
                <IconMicrophone2 />
              </button>
              <div class="group/vol flex items-center gap-[5px]" role="presentation" @click.stop>
                <button
                  :class="[ICON_ACTION, 'text-[#9a9aa4] hover:bg-white/10 hover:text-[#f0f0f4]']"
                  :aria-label="muted ? 'Activar sonido' : 'Silenciar'"
                  @click="emit('toggle-mute')"
                >
                  <IconVolumeOff v-if="muted || volume === 0" />
                  <IconVolume2 v-else />
                </button>
                <input
                  class="w-0 cursor-pointer accent-[#f4f4f6] opacity-0 [transition:width_.22s_cubic-bezier(.16,1,.3,1),opacity_.18s] group-hover/vol:w-[52px] group-hover/vol:opacity-100 focus-visible:w-[52px] focus-visible:opacity-100"
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

          <div class="flex items-center gap-2">
            <span class="flex-none text-[8.5px] tabular-nums text-[#9999a0]">{{ positionLabel }}</span>
            <div
              class="group/seek min-w-0 flex-1 touch-none py-[5px] outline-none"
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

            <div class="flex items-center justify-center">
              <div class="flex items-center justify-center gap-[9px]">
                <button :class="[MC, MC_HOVER, CONTROL]" aria-label="Pista anterior" @click="emit('previous')"><IconPlayerTrackPrev /></button>
                <button
                  :class="[MC, SKIP, canSeek ? 'hover:scale-[1.12] hover:text-white' : 'cursor-default opacity-30']"
                  :disabled="!canSeek"
                  :title="canSeek ? `Retroceder ${skipSeconds} segundos` : 'Este reproductor no permite avanzar la pista'"

                  :aria-label="`Retroceder ${skipSeconds} segundos`"
                  @click.stop="emit('skip', -skipSeconds)"
                >
                  <component :is="skipIcons[0]" />
                </button>
                <button :class="[MC, MC_HOVER, PLAY]" :aria-label="playing ? 'Pausar' : 'Reproducir'" @click="emit('toggle-play')">
                  <IconPlayerPause v-if="playing" />
                  <IconPlayerPlay v-else />
                </button>
                <button
                  :class="[MC, SKIP, canSeek ? 'hover:scale-[1.12] hover:text-white' : 'cursor-default opacity-30']"
                  :disabled="!canSeek"
                  :title="canSeek ? `Avanzar ${skipSeconds} segundos` : 'Este reproductor no permite avanzar la pista'"
                  :aria-label="`Avanzar ${skipSeconds} segundos`"
                  @click.stop="emit('skip', skipSeconds)"
                >
                  <component :is="skipIcons[1]" />
                </button>
                <button :class="[MC, MC_HOVER, CONTROL]" aria-label="Pista siguiente" @click="emit('next')"><IconPlayerTrackNext /></button>
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
            <IconPlayerPause v-if="timerRunning" />
            <IconPlayerPlay v-else />
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
    </div>
  </div>
  
</template>
