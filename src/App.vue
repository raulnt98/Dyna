<script setup lang="ts">
import {
  IconApps,
  IconArrowDownToArc,
  IconBrandWindows,
  IconCheck,
  IconClock,
  IconDots,
  IconMaximize,
  IconMessage,
  IconMicrophone,
  IconMinus,
  IconMusic,
  IconSearch,
  IconSettings,
  IconVolume2,
  IconX,
  IconPhone,
} from "@tabler/icons-vue";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { vAutoAnimate } from "@formkit/auto-animate/vue";
import {
  Popover,
  PopoverContent,
  PopoverPortal,
  PopoverTrigger,
} from "@/components/ui/popover";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Button } from "@/components/ui/button";
import TheIsland from "@/components/TheIsland.vue";
import { useSystemMedia } from "@/lib/media";

type Activity = "idle" | "ring" | "timer" | "record" | "music" | "airdrop" | "call" | "message";
type Wallpaper = "aurora" | "dusk" | "blue";

const activities: { id: Activity; title: string; detail: string; icon: typeof IconMusic }[] = [
  { id: "idle", title: "Inactiva", detail: "En espera", icon: IconDots },
  { id: "ring", title: "Llamada entrante", detail: "Alex te está llamando", icon: IconPhone },
  { id: "timer", title: "Temporizador", detail: "Cuenta atrás activa", icon: IconClock },
  { id: "record", title: "Grabación", detail: "Capturando audio", icon: IconMicrophone },
  { id: "music", title: "Música", detail: "Reproducción en curso", icon: IconMusic },
  { id: "airdrop", title: "Transferencia", detail: "Archivo en camino", icon: IconArrowDownToArc },
  { id: "call", title: "Llamada", detail: "En llamada", icon: IconPhone },
  { id: "message", title: "Notificación", detail: "Mensaje nuevo", icon: IconMessage },
];
const isActivity = (value: string): value is Activity =>
  activities.some((activity) => activity.id === value);
const activityCount = String(activities.length).padStart(2, "0");

const wallpapers: { id: Wallpaper; name: string }[] = [
  { id: "aurora", name: "Aurora" },
  { id: "dusk", name: "Atardecer" },
  { id: "blue", name: "Azul" },
];

const mode = ref<Activity>("music");
const expanded = ref(true);
const timerRunning = ref(true);
const callActive = ref(true);
const muted = ref(false);
const recording = ref(true);
const timerSeconds = ref(272);
const callSeconds = ref(138);
const recordingSeconds = ref(42);
const wallpaper = ref<Wallpaper>("aurora");
const currentTime = ref(new Date());
let clockInterval: number | undefined;

const formatTime = (seconds: number) =>
  `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;

const desktopTime = computed(() =>
  new Intl.DateTimeFormat("es", { hour: "2-digit", minute: "2-digit", hour12: false }).format(currentTime.value),
);
const desktopDate = computed(() =>
  new Intl.DateTimeFormat("es", { day: "2-digit", month: "2-digit", year: "numeric" }).format(currentTime.value),
);
const timerLabel = computed(() => formatTime(timerSeconds.value));
const callLabel = computed(() => formatTime(callSeconds.value));

// Música real del sistema (SMTC) dentro de Tauri; pista de ejemplo en la web.
const media = useSystemMedia();
const playing = computed(() => media.state.value.playing);
const trackTitle = computed(() => media.state.value.title || "Sin reproducción");
/** Artista y álbum, omitiendo el separador si falta alguno de los dos. */
const trackSubtitle = computed(() => {
  const { artist, album } = media.state.value;
  return [artist, album].filter(Boolean).join(" · ");
});
/** Inicial que se muestra cuando la pista no tiene carátula. */
const trackInitial = computed(() => trackTitle.value.charAt(0).toUpperCase() || "M");
/** Carátula real de la pista, si el reproductor la expone. */
const trackArtwork = computed(() => media.state.value.artwork);

/**
 * Salta a la posición de la pista correspondiente al punto pulsado.
 *
 * Solo tiene efecto con una duración conocida; si el reproductor no admite
 * búsqueda, el backend devuelve `false` y el estado no cambia.
 */
function scrubTo(ratio: number) {
  const { duration } = media.state.value;
  if (duration <= 0) return;
  media.seek(ratio * duration);
}
const positionLabel = computed(() => formatTime(Math.floor(media.state.value.position)));
const durationLabel = computed(() => formatTime(Math.floor(media.state.value.duration)));
/** Progreso de la pista en porcentaje, acotado por si el reporte es irregular. */
const trackProgress = computed(() => {
  const { position, duration } = media.state.value;
  if (duration <= 0) return 0;
  return Math.min(100, Math.max(0, (position / duration) * 100));
});

function chooseActivity(value: unknown) {
  if (typeof value !== "string" || !isActivity(value)) return;
  mode.value = value;
  expanded.value = true;
  if (mode.value === "call") callActive.value = true;
}

function toggleIsland() {
  expanded.value = !expanded.value;
}

function handleIslandKeydown(event: KeyboardEvent) {
  if (event.target !== event.currentTarget) return;
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    toggleIsland();
  }
}

function handleIslandClick(event: MouseEvent) {
  if (event.target instanceof Element && event.target.closest("button")) return;
  toggleIsland();
}

onMounted(() => {
  clockInterval = window.setInterval(() => {
    currentTime.value = new Date();
    if (mode.value === "timer" && timerRunning.value && timerSeconds.value > 0) timerSeconds.value -= 1;
    if (mode.value === "call" && callActive.value) callSeconds.value += 1;
    if (mode.value === "record" && recording.value) recordingSeconds.value += 1;
  }, 1000);
});

onUnmounted(() => {
  if (clockInterval !== undefined) window.clearInterval(clockInterval);
});

/* Clases de cada fondo, escritas completas: Tailwind solo genera lo que
   encuentra literal en el archivo. Las define `style.css` como utilidades. */
const WALLPAPER: Record<Wallpaper, string> = {
  aurora: "wallpaper-aurora",
  dusk: "wallpaper-dusk",
  blue: "wallpaper-blue",
};
const SWATCH: Record<Wallpaper, string> = {
  aurora: "swatch-aurora",
  dusk: "swatch-dusk",
  blue: "swatch-blue",
};
</script>

<template>
  <main
    class="mx-auto grid min-h-screen w-[min(1190px,calc(100%_-_64px))] grid-cols-[minmax(0,.9fr)_minmax(0,1.1fr)] items-center gap-[clamp(44px,8vw,110px)] py-12 max-[900px]:w-[min(620px,calc(100%_-_36px))] max-[900px]:grid-cols-[minmax(0,1fr)] max-[900px]:gap-[42px] max-[900px]:py-9 max-[500px]:w-[calc(100%_-_28px)]"
  >
    <section class="max-w-[470px] min-w-0 max-[900px]:mx-auto" aria-labelledby="page-title">
      <a
        class="inline-flex items-center gap-2.5 text-[15px] font-[650] tracking-[-.035em] text-[#f6f6f8] no-underline"
        href="#"
        aria-label="Dynamic Island para Windows"
      >
        <span class="grid size-[29px] place-items-center rounded-[9px] bg-green text-[#090b12]">
          <IconBrandWindows class="size-[17px] stroke-2" />
        </span>
        <span>dynamic<span class="font-[450] text-[#9c9ca8]">.island</span></span>
      </a>

      <div
        class="mt-[54px] flex items-center gap-[9px] text-[10px] font-bold tracking-[.15em] text-green uppercase max-[900px]:mt-[38px]"
      >
        <span class="size-[7px] rounded-full bg-green shadow-[0_0_13px_rgba(183,243,107,.7)]"></span>
        Isla dinámica para Windows
      </div>
      <h1
        id="page-title"
        class="mt-[17px] mb-4 text-[clamp(43px,5vw,65px)] leading-[.99] font-semibold tracking-[-.07em] max-[500px]:text-[clamp(42px,12vw,58px)]"
      >
        Lo importante,<br />siempre
        <span class="bg-[linear-gradient(110deg,#c8f89a_5%,#8de4d1_58%,#b9a8ff)] bg-clip-text text-transparent">a mano.</span>
      </h1>
      <p class="max-w-[405px] text-[14px] leading-[1.8] text-[#a4a4b1]">
        Una isla inteligente en tu escritorio. Mira qué está pasando y controla tus actividades sin interrumpir lo que haces.
      </p>

      <div class="mt-[34px] mb-[13px] flex items-center justify-between">
        <h2 class="text-[12px] font-[650]">Prueba una actividad</h2>
        <span class="text-[9px] tracking-[.08em] text-[#71717e]">{{ activityCount }} opciones</span>
      </div>
      <ToggleGroup
        type="single"
        :model-value="mode"
        class="grid grid-cols-2 gap-2.5"
        aria-label="Actividades de la isla dinámica"
        @update:model-value="chooseActivity"
      >
        <ToggleGroupItem
          v-for="activity in activities"
          :key="activity.id"
          :value="activity.id"
          class="group relative flex min-h-[66px] min-w-0 items-center gap-[11px] rounded-[14px] border border-line bg-panel px-3 py-[11px] text-left text-[#f6f6f8] outline-none transition-[border-color,background-color,translate] duration-200 hover:-translate-y-0.5 hover:border-white/20 data-[state=on]:border-green/55 data-[state=on]:bg-green/[.075]"
          :aria-label="`${activity.title}: ${activity.detail}`"
        >
          <span
            class="grid size-8 flex-none place-items-center rounded-[10px] border border-white/[.075] bg-white/5 text-[#d3d3dd] group-data-[state=on]:border-green/16 group-data-[state=on]:bg-green/8 group-data-[state=on]:text-green"
          >
            <component :is="activity.icon" class="size-[17px] stroke-[1.7]" />
          </span>
          <span class="grid min-w-0 gap-1">
            <strong class="truncate text-[11px] font-[650]">{{ activity.title }}</strong>
            <small class="truncate text-[9px] text-muted">{{ activity.detail }}</small>
          </span>
          <IconCheck class="ml-auto size-[13px] flex-none text-green opacity-0 group-data-[state=on]:opacity-100" />
        </ToggleGroupItem>
      </ToggleGroup>

      <div class="mt-[15px] text-[10px] leading-[1.6] text-[#777784]">
        <span class="mr-[5px] text-green">↗</span> Haz clic en la isla para expandirla y ver los controles.
      </div>
      <div class="mt-[37px] flex items-center gap-[11px] text-[8px] font-[650] tracking-[.14em] text-[#5e5e6b]">
        <span class="h-px w-[22px] bg-[#454552]"></span>
        <span>VUE 3 <i class="mx-[3px] text-[#8c8c99] not-italic">·</i> TAURI <i class="mx-[3px] text-[#8c8c99] not-italic">·</i> WINDOWS</span>
      </div>
      <p class="mt-2.5 ml-[33px] text-[9px] text-[#666672]">
        Estados inspirados en
        <a
          class="text-[#9696a3] underline decoration-[rgba(150,150,163,.45)] underline-offset-3 hover:text-[#d3d3dc]"
          href="https://skiper-ui.com/v1/skiper2"
          target="_blank"
          rel="noreferrer"
        >Skiper UI · Skiper 2</a>
      </p>
    </section>

    <section class="min-w-0 max-[900px]:w-full" aria-label="Vista previa del escritorio Windows">
      <div class="mb-5 flex items-end justify-between">
        <div>
          <span class="text-[9px] font-bold tracking-[.15em] text-[#82828f]">VISTA PREVIA INTERACTIVA</span>
          <h2 class="mt-2 text-[20px] leading-[1.18] font-[550] tracking-[-.045em]">
            Tu escritorio,<br /><span class="text-[#858591]">con otra dimensión.</span>
          </h2>
        </div>
        <Popover>
          <PopoverTrigger as-child>
            <Button
              variant="outline"
              size="icon"
              class="size-[35px] rounded-[11px] border-white/10 text-[#c8c8d0] hover:text-white"
              aria-label="Personalizar fondo del escritorio"
            >
              <IconSettings class="size-[17px] stroke-[1.6]" />
            </Button>
          </PopoverTrigger>
          <PopoverPortal>
            <PopoverContent
              class="w-[190px] animate-popover-in rounded-[14px] border border-white/12 bg-[#171820] p-3.5 text-[#f7f7fa] shadow-[0_16px_45px_rgba(0,0,0,.4)] outline-none"
              :side-offset="10"
              align="end"
            >
              <div class="grid gap-1">
                <strong class="text-[11px] font-[650]">Fondo de escritorio</strong>
                <span class="text-[9px] text-[#9494a0]">Elige un ambiente</span>
              </div>
              <div class="mt-3 flex gap-2">
                <button
                  v-for="item in wallpapers"
                  :key="item.id"
                  class="grid h-[34px] w-[42px] place-items-center rounded-lg border border-white/18 text-white"
                  :class="[SWATCH[item.id], wallpaper === item.id && 'outline-2 outline-offset-2 outline-green']"
                  :aria-label="item.name"
                  :aria-pressed="wallpaper === item.id"
                  @click="wallpaper = item.id"
                >
                  <IconCheck v-if="wallpaper === item.id" class="size-3.5" />
                </button>
              </div>
              <p class="mt-[9px] text-[9px] text-[#adadb7]">{{ wallpapers.find((item) => item.id === wallpaper)?.name }}</p>
            </PopoverContent>
          </PopoverPortal>
        </Popover>
      </div>

      <div
        class="rounded-[23px] border border-white/12 bg-[linear-gradient(145deg,#282832,#14151d_55%,#20212a)] p-[7px] shadow-[0_36px_100px_rgba(0,0,0,.5),0_0_0_1px_rgba(255,255,255,.025)] max-[500px]:rounded-[19px] max-[500px]:p-[5px]"
      >
        <div
          v-auto-animate
          class="relative isolate min-h-[410px] overflow-hidden rounded-[17px] border border-white/12 bg-[#263d75] transition-[background] duration-[450ms] ease-in-out before:absolute before:top-[10%] before:-left-[20%] before:-z-1 before:h-4/5 before:w-[68%] before:rotate-[38deg] before:rounded-[48%_52%_62%_38%] before:border before:border-white/12 before:blur-[1px] before:content-[''] after:absolute after:-right-1/4 after:-bottom-[3%] after:-z-1 after:h-4/5 after:w-[68%] after:-rotate-[37deg] after:rounded-[48%_52%_62%_38%] after:border after:border-white/12 after:blur-[1px] after:content-[''] max-[500px]:min-h-[340px] max-[500px]:rounded-[14px]"
          :class="WALLPAPER[wallpaper]"
        >
          <div
            class="pointer-events-none absolute inset-0 -z-1 bg-[linear-gradient(180deg,rgba(14,12,32,.08),transparent_48%,rgba(12,12,28,.25))]"
          ></div>
          <div
            class="absolute top-[18px] left-[17px] grid gap-[18px] max-[500px]:top-3.5 max-[500px]:left-2 max-[500px]:gap-3"
            aria-hidden="true"
          >
            <div class="grid w-[52px] justify-items-center gap-1 text-[8px] text-white/88 [text-shadow:0_1px_5px_rgba(0,0,0,.5)]">
              <span
                class="grid size-[27px] place-items-center rounded-[7px] border border-white/20 bg-white/15 backdrop-blur-[8px] before:h-[15px] before:w-3 before:rounded-[2px_2px_4px_4px] before:border before:border-t-3 before:border-white/85 before:content-['']"
              ></span>
              <span>Papelera</span>
            </div>
            <div class="grid w-[52px] justify-items-center gap-1 text-[8px] text-white/88 [text-shadow:0_1px_5px_rgba(0,0,0,.5)]">
              <span
                class="grid size-[27px] place-items-center rounded-[7px] border border-white/20 bg-[linear-gradient(150deg,#fbd877,#cd8f39)] text-[#ffe091] backdrop-blur-[8px] before:h-2.5 before:w-[15px] before:rounded-[2px] before:bg-[linear-gradient(#ffe69b,#ffd15b)] before:content-['']"
              ></span>
              <span>Proyectos</span>
            </div>
            <div class="grid w-[52px] justify-items-center gap-1 text-[8px] text-white/88 [text-shadow:0_1px_5px_rgba(0,0,0,.5)]">
              <span
                class="grid size-[27px] place-items-center rounded-[7px] border border-white/20 bg-white/15 text-[19px] text-[#9cdcff] backdrop-blur-[8px]"
              >◉</span>
              <span>Navegador</span>
            </div>
          </div>

          <TheIsland
            :mode="mode"
            :expanded="expanded"
            :playing="playing"
            :recording="recording"
            :recording-seconds="recordingSeconds"
            :timer-running="timerRunning"
            :timer-label="timerLabel"
            :call-label="callLabel"
            :call-active="callActive"
            :muted="muted"
            :volume="50"
            :track-title="trackTitle"
            :track-subtitle="trackSubtitle"
            :track-initial="trackInitial"
            :track-artwork="trackArtwork"
            :position-label="positionLabel"
            :duration-label="durationLabel"
            :remaining-label="durationLabel"
            :track-progress="trackProgress"
            :can-seek="true"
            :skip-seconds="10"
            :lyrics-available="false"
            :lyrics-open="false"
            :lyric-lines="[]"
            :format-time="formatTime"
            @island-click="handleIslandClick"
            @island-keydown="handleIslandKeydown"
            @choose="chooseActivity"
            @toggle-play="media.togglePlay()"
            @next="media.next()"
            @previous="media.previous()"
            @scrub="scrubTo"
            @toggle-timer="timerRunning = !timerRunning"
            @toggle-recording="recording = !recording"
            @toggle-mute="muted = !muted"
            @volume="() => {}"
            @toggle-lyrics="() => {}"
            @skip="() => {}"
            @end-call="callActive = false"
          />

          <div
            v-if="mode === 'message'"
            class="absolute right-[15px] bottom-[58px] z-3 flex w-[min(260px,calc(100%_-_30px))] animate-toast-in items-start gap-2.5 rounded-[14px] border border-white/17 bg-[rgba(26,27,42,.66)] p-3 shadow-[0_12px_32px_rgba(12,12,25,.2)] backdrop-blur-[22px] max-[500px]:bottom-[51px]"
            aria-live="polite"
          >
            <span
              class="grid size-7 flex-none place-items-center rounded-lg bg-[linear-gradient(145deg,#ffa784,#ee5970)] text-white"
            ><IconMessage class="size-[15px]" /></span>
            <span class="grid min-w-0 flex-1 gap-1 text-[9px]">
              <strong class="text-[10px]">Alex <small class="font-normal text-[#a2a2a8]">· ahora</small></strong>
              <span class="leading-[1.45] text-[#d4d4db]">¡Ya llegué! Te espero en la entrada 😊</span>
            </span>
            <button class="grid p-0 text-[#a3a3ad]" aria-label="Cerrar notificación" @click="chooseActivity('music')">
              <IconX class="size-[13px]" />
            </button>
          </div>

          <div
            class="absolute inset-x-0 bottom-0 z-2 flex h-[43px] items-center gap-[11px] border-t border-white/17 bg-[rgba(17,18,29,.68)] px-[15px] backdrop-blur-[20px] max-[500px]:h-10 max-[500px]:gap-[5px] max-[500px]:px-[7px]"
          >
            <button class="grid size-[27px] flex-none place-items-center rounded-[7px] text-[#e9e9ef] hover:bg-white/12" aria-label="Inicio de Windows">
              <IconBrandWindows class="size-4" />
            </button>
            <button
              class="flex h-[25px] w-[125px] items-center gap-[7px] rounded-[7px] border border-white/13 bg-white/9 px-[9px] text-[9px] text-white/68 max-[500px]:w-[88px]"
            >
              <IconSearch class="size-[13px]" /><span>Buscar</span>
            </button>
            <div class="flex items-center gap-1">
              <button
                class="relative grid size-[27px] flex-none place-items-center rounded-[7px] bg-white/8 text-[#9ec7ff] after:absolute after:right-[7px] after:bottom-[-2px] after:left-[7px] after:h-0.5 after:rounded-[2px] after:bg-[#83b7ff] after:content-[''] hover:bg-white/12"
                aria-label="Vista de tareas"
              >
                <IconApps class="size-4" />
              </button>
              <button
                class="grid size-[27px] flex-none place-items-center rounded-[7px] text-[18px] text-[#82cefa] hover:bg-white/12 max-[500px]:hidden"
                aria-label="Navegador"
              >◉</button>
            </div>
            <div class="ml-auto flex items-center gap-2.5 text-[#e7e7ec] max-[500px]:gap-[5px]">
              <span class="text-[12px] text-[#d5d5de]">⌃</span>
              <IconVolume2 class="size-3.5" />
              <span class="grid gap-0.5 text-right text-[8px] leading-[1.1]">
                <strong class="font-medium">{{ desktopTime }}</strong>
                <small class="text-[7px] text-white/75">{{ desktopDate }}</small>
              </span>
              <IconMessage class="size-3.5" />
            </div>
          </div>
        </div>
      </div>

      <div class="mx-0.5 mt-3 flex items-center justify-between">
        <span class="flex items-center gap-[7px] text-[8px] font-bold tracking-[.1em] text-[#70707c]">
          <i class="size-[5px] rounded-full bg-green shadow-[0_0_13px_rgba(183,243,107,.7)]"></i> DEMO WEB INTERACTIVA
        </span>
        <span class="flex items-center gap-[11px] text-[#666672]" aria-hidden="true">
          <IconMinus class="size-3" /><IconMaximize class="size-3" /><IconX class="size-3" />
        </span>
      </div>
      <p class="mt-2 text-center text-[9px] text-[#62626f]">Escritorio simulado · Controles de la isla activos</p>
    </section>
  </main>
</template>
