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
</script>

<template>
  <main class="app-shell">
    <section class="intro" aria-labelledby="page-title">
      <a class="brand" href="#" aria-label="Dynamic Island para Windows">
        <span class="brand-mark"><IconBrandWindows /></span>
        <span>dynamic<span class="brand-light">.island</span></span>
      </a>

      <div class="eyebrow"><span class="live-dot"></span> Isla dinámica para Windows</div>
      <h1 id="page-title">Lo importante,<br />siempre <span>a mano.</span></h1>
      <p class="description">
        Una isla inteligente en tu escritorio. Mira qué está pasando y controla tus actividades sin interrumpir lo que haces.
      </p>

      <div class="activity-header">
        <h2>Prueba una actividad</h2>
        <span>{{ activityCount }} opciones</span>
      </div>
      <ToggleGroup
        type="single"
        :model-value="mode"
        class="activity-grid"
        aria-label="Actividades de la isla dinámica"
        @update:model-value="chooseActivity"
      >
        <ToggleGroupItem
          v-for="activity in activities"
          :key="activity.id"
          :value="activity.id"
          class="activity-card"
          :aria-label="`${activity.title}: ${activity.detail}`"
        >
          <span class="activity-icon"><component :is="activity.icon" /></span>
          <span class="activity-copy">
            <strong>{{ activity.title }}</strong>
            <small>{{ activity.detail }}</small>
          </span>
          <IconCheck class="activity-check" />
        </ToggleGroupItem>
      </ToggleGroup>

      <div class="hint"><span>↗</span> Haz clic en la isla para expandirla y ver los controles.</div>
      <div class="stack-note">
        <span class="stack-line"></span>
        <span>VUE 3 <i>·</i> TAURI <i>·</i> WINDOWS</span>
      </div>
      <p class="reference-note">
        Estados inspirados en
        <a href="https://skiper-ui.com/v1/skiper2" target="_blank" rel="noreferrer">Skiper UI · Skiper 2</a>
      </p>
    </section>

    <section class="preview-column" aria-label="Vista previa del escritorio Windows">
      <div class="preview-heading">
        <div>
          <span class="preview-kicker">VISTA PREVIA INTERACTIVA</span>
          <h2>Tu escritorio,<br /><span>con otra dimensión.</span></h2>
        </div>
        <Popover>
          <PopoverTrigger as-child>
            <Button variant="outline" size="icon" class="settings-trigger" aria-label="Personalizar fondo del escritorio">
              <IconSettings />
            </Button>
          </PopoverTrigger>
          <PopoverPortal>
            <PopoverContent class="wallpaper-popover" :side-offset="10" align="end">
              <div class="popover-heading">
                <strong>Fondo de escritorio</strong>
                <span>Elige un ambiente</span>
              </div>
              <div class="wallpaper-options">
                <button
                  v-for="item in wallpapers"
                  :key="item.id"
                  class="wallpaper-option"
                  :class="[`wallpaper-${item.id}`, { selected: wallpaper === item.id }]"
                  :aria-label="item.name"
                  :aria-pressed="wallpaper === item.id"
                  @click="wallpaper = item.id"
                >
                  <IconCheck v-if="wallpaper === item.id" />
                </button>
              </div>
              <p class="wallpaper-caption">{{ wallpapers.find((item) => item.id === wallpaper)?.name }}</p>
            </PopoverContent>
          </PopoverPortal>
        </Popover>
      </div>

      <div class="desktop-frame">
        <div v-auto-animate class="desktop-preview" :class="`theme-${wallpaper}`">
          <div class="desktop-glow"></div>
          <div class="desktop-shortcuts" aria-hidden="true">
            <div class="desktop-shortcut">
              <span class="shortcut-icon recycle"></span>
              <span>Papelera</span>
            </div>
            <div class="desktop-shortcut">
              <span class="shortcut-icon folder"></span>
              <span>Proyectos</span>
            </div>
            <div class="desktop-shortcut">
              <span class="shortcut-icon browser">◉</span>
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

          <div v-if="mode === 'message'" class="toast-notification" aria-live="polite">
            <span class="toast-icon"><IconMessage /></span>
            <span class="toast-copy"><strong>Alex <small>· ahora</small></strong><span>¡Ya llegué! Te espero en la entrada 😊</span></span>
            <button aria-label="Cerrar notificación" @click="chooseActivity('music')"><IconX /></button>
          </div>

          <div class="taskbar">
            <button class="taskbar-start" aria-label="Inicio de Windows"><IconBrandWindows /></button>
            <button class="taskbar-search"><IconSearch /><span>Buscar</span></button>
            <div class="taskbar-pinned">
              <button class="taskbar-app active-app" aria-label="Vista de tareas"><IconApps /></button>
              <button class="taskbar-app browser-app" aria-label="Navegador">◉</button>
            </div>
            <div class="taskbar-tray">
              <span class="tray-indicator">⌃</span>
              <IconVolume2 />
              <span class="taskbar-clock"><strong>{{ desktopTime }}</strong><small>{{ desktopDate }}</small></span>
              <IconMessage />
            </div>
          </div>
        </div>
      </div>

      <div class="preview-footer">
        <span><i></i> DEMO WEB INTERACTIVA</span>
        <span class="window-actions" aria-hidden="true"><IconMinus /><IconMaximize /><IconX /></span>
      </div>
      <p class="preview-caption">Escritorio simulado · Controles de la isla activos</p>
    </section>
  </main>
</template>
