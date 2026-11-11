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
import { computed } from "vue";
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
  /** Segundos que saltan los botones de avance y retroceso. */
  skipSeconds: number;
  formatTime: (seconds: number) => string;
}>();

/**
 * La isla se dibuja como píldora, en vez de como panel.
 *
 * Un aviso no cierra el panel: abierto se muestra como franja dentro de él,
 * porque el usuario lo abrió a propósito y puede estar usando el volumen o la
 * barra de progreso justo cuando entra el aviso.
 */
const showsPill = computed(() => !props.expanded);

/**
 * Ancho de la píldora.
 *
 * Expandida y sin aviso manda la clase de Tailwind; en píldora el ancho se
 * calcula con la misma función que usa la ventana nativa, para que la isla y
 * su ventana no se desincronicen.
 */
const compactWidth = computed(() => {
  if (!showsPill.value) return undefined;
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
      { 'is-expanded': expanded, 'is-paused': !playing },
      `mode-${mode}`,
      { 'has-lyrics': expanded && lyricsOpen && lyricsAvailable },
      showsPill
        ? 'h-[34px] rounded-[18px]'
        : 'w-[330px] h-[108px] rounded-[24px]',
    ]"
    :style="compactWidth"
    role="button"
    tabindex="0"
    :aria-label="`Isla dinámica, ${expanded ? 'contraer' : 'expandir'}`"
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

    <div v-if="showsPill" class="island-collapsed" aria-hidden="true">
      <!-- Un aviso manda sobre el modo: interrumpe, se muestra unos segundos
           y al retirarse la isla recupera lo que estaba enseñando. -->
      <template v-if="notice">
        <span class="notice-icon" :class="`notice-${notice.kind}`">
          <IconBolt v-if="notice.kind === 'power'" />
          <IconBell v-else />
        </span>
        <span class="collapsed-time notice-label">{{ notice.label }}</span>
      </template>
      <template v-else-if="mode === 'idle'">
        <span class="album-art idle-art"><IconDots /></span>
        <span class="waveform idle-wave"><i></i><i></i><i></i></span>
      </template>
      <template v-else-if="mode === 'ring'">
        <span class="ring-pulse"><IconPhone /></span>
        <span class="collapsed-message">Alex</span>
      </template>
      <template v-else-if="mode === 'record'">
        <span class="record-dot"></span>
        <span class="collapsed-time">{{ formatTime(recordingSeconds) }}</span>
      </template>
      <template v-else-if="mode === 'music'">
        <span class="album-art mini-album">
          <img v-if="trackArtwork" :src="trackArtwork" alt="" />
          <template v-else>{{ trackInitial }}</template>
        </span>
        <span class="waveform"><i></i><i></i><i></i><i></i><i></i></span>
      </template>
      <template v-else-if="mode === 'timer'">
        <IconClock class="collapsed-icon timer-icon" />
        <span class="collapsed-time">{{ timerLabel }}</span>
      </template>
      <template v-else-if="mode === 'call'">
        <span class="call-live-dot"></span>
        <span class="collapsed-time">{{ callLabel }}</span>
      </template>
      <template v-else-if="mode === 'airdrop'">
        <IconArrowDownToArc class="collapsed-icon" />
        <span class="collapsed-time">68%</span>
      </template>
      <template v-else>
        <IconBell class="collapsed-icon" />
        <span class="collapsed-message">1 nuevo</span>
      </template>
    </div>
  
    <!-- El aviso no cierra el panel: entra como franja sobre él y se retira
         sola, dejando intacto lo que el usuario estuviera haciendo.
         Va fuera de `.island-content` a propósito: ahí dentro `v-auto-animate`
         gestionaría su entrada y pelearía con la animación propia. -->
    <div
      v-if="expanded && notice"
      class="notice-strip"
      :class="`notice-${notice.kind}`"
      aria-hidden="true"
    >
      <span class="notice-strip-icon">
        <IconBolt v-if="notice.kind === 'power'" />
        <IconBell v-else />
      </span>
      <span class="notice-strip-label">{{ notice.label }}</span>
    </div>

    <div v-if="!showsPill" v-auto-animate class="island-content">
      <template v-if="mode === 'idle'">
        <div class="activity-top">
          <span class="idle-avatar"><IconDots /></span>
          <span class="media-copy"><strong>Todo tranquilo</strong><small>La isla está lista para ti</small></span>
          <span class="idle-status">EN ESPERA</span>
        </div>
        <div class="idle-footer"><span>Sin actividades activas</span><span class="idle-indicator"></span></div>
      </template>
  
      <template v-else-if="mode === 'ring'">
        <div class="activity-top">
          <span class="contact-avatar ring-avatar">A</span>
          <span class="media-copy"><strong>Alex Morgan</strong><small class="incoming-label">Llamada entrante · móvil</small></span>
          <span class="ring-pulse"><IconPhone /></span>
        </div>
        <div class="incoming-controls">
          <button class="incoming-action decline-call" aria-label="Rechazar llamada" @click="chooseActivity('idle')"><IconPhone /></button>
          <span>Desliza para responder</span>
          <button class="incoming-action accept-call" aria-label="Responder llamada" @click="chooseActivity('call')"><IconPhone /></button>
        </div>
      </template>
  
      <template v-else-if="mode === 'record'">
        <div class="activity-top">
          <span class="recording-avatar"><IconMicrophone /></span>
          <span class="media-copy"><strong>Grabación de audio</strong><small>{{ recording ? "Grabando ahora" : "Grabación en pausa" }}</small></span>
          <span class="recording-clock">{{ formatTime(recordingSeconds) }}</span>
        </div>
        <div class="recording-controls">
          <div class="recording-wave" :class="{ paused: !recording }"><i v-for="bar in 25" :key="bar" :style="{ '--bar': `${20 + ((bar * 31) % 70)}%` }"></i></div>
          <button class="media-control play-control" :aria-label="recording ? 'Pausar grabación' : 'Reanudar grabación'" @click="emit('toggle-recording')">
            <IconPlayerPause v-if="recording" />
            <IconPlayerPlay v-else />
          </button>
        </div>
      </template>
  
      <template v-else-if="mode === 'airdrop'">
        <div class="activity-top">
          <span class="transfer-avatar"><IconArrowDownToArc /></span>
          <span class="media-copy"><strong>Recibiendo archivo</strong><small>MacBook Pro · ahora mismo</small></span>
          <button class="dismiss-button" aria-label="Cerrar transferencia" @click="chooseActivity('idle')"><IconX /></button>
        </div>
        <div class="transfer-info"><span>Presentación-final.pdf</span><strong>68%</strong></div>
        <div class="transfer-progress"><span></span></div>
      </template>
  
      <template v-else-if="mode === 'music'">
        <div class="music-shell">
        <div class="music-layout">
          <div class="music-heading">
            <span class="album-art album-art-large">
              <img v-if="trackArtwork" :src="trackArtwork" alt="" />
              <template v-else>{{ trackInitial }}</template>
            </span>
            <span class="media-copy">
              <strong>{{ trackTitle }}</strong>
              <small>{{ trackSubtitle }}</small>
            </span>
            <!-- Cinco barras: el patrón de alturas (5-10-15-10-5) es simétrico
                 y el CSS las estiliza por posición, así que el número importa. -->
            <span class="waveform heading-wave" :class="{ 'is-idle': !playing }" aria-hidden="true">
              <i></i><i></i><i></i><i></i><i></i>
            </span>

            <!-- Karaoke y volumen viven arriba: la fila de controles ya está
                 ocupada por reproducción, saltos y cambio de pista. -->
            <div class="heading-actions">
              <button
                v-if="lyricsAvailable"
                class="icon-action"
                :class="{ 'is-on': lyricsOpen }"
                :aria-label="lyricsOpen ? 'Ocultar letra' : 'Mostrar letra'"
                :aria-pressed="lyricsOpen"
                @click.stop="emit('toggle-lyrics')"
              >
                <IconMicrophone2 />
              </button>
              <div class="volume" role="presentation" @click.stop>
                <button
                  class="icon-action"
                  :aria-label="muted ? 'Activar sonido' : 'Silenciar'"
                  @click="emit('toggle-mute')"
                >
                  <IconVolumeOff v-if="muted || volume === 0" />
                  <IconVolume2 v-else />
                </button>
                <input
                  class="volume-slider"
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

          <div class="track-line">
            <span class="track-time">{{ positionLabel }}</span>
            <div
              class="track-seek"
              :class="{ 'is-static': !canSeek }"
              role="slider"
              tabindex="0"
              aria-label="Posición de la pista"
              :aria-valuemin="0"
              :aria-valuemax="100"
              :aria-valuenow="Math.round(trackProgress)"
              @pointerdown="canSeek && startScrub($event)"
              @keydown="onSeekKeydown"
            >
              <div class="track-progress">
                <span :style="{ width: `${trackProgress}%` }"></span>
                <i class="track-knob" :style="{ left: `${trackProgress}%` }"></i>
              </div>
            </div>
            <span class="track-time">{{ remainingLabel }}</span>
          </div>

            <div class="control-row">
              <div class="control-buttons">
                <button class="media-control" aria-label="Pista anterior" @click="emit('previous')"><IconPlayerTrackPrev /></button>
                <button
                  class="media-control skip-control"
                  :disabled="!canSeek"
                  :title="canSeek ? `Retroceder ${skipSeconds} segundos` : 'Este reproductor no permite avanzar la pista'"

                  :aria-label="`Retroceder ${skipSeconds} segundos`"
                  @click.stop="emit('skip', -skipSeconds)"
                >
                  <component :is="skipIcons[0]" />
                </button>
                <button class="media-control play-control" :aria-label="playing ? 'Pausar' : 'Reproducir'" @click="emit('toggle-play')">
                  <IconPlayerPause v-if="playing" />
                  <IconPlayerPlay v-else />
                </button>
                <button
                  class="media-control skip-control"
                  :disabled="!canSeek"
                  :title="canSeek ? `Avanzar ${skipSeconds} segundos` : 'Este reproductor no permite avanzar la pista'"
                  :aria-label="`Avanzar ${skipSeconds} segundos`"
                  @click.stop="emit('skip', skipSeconds)"
                >
                  <component :is="skipIcons[1]" />
                </button>
                <button class="media-control" aria-label="Pista siguiente" @click="emit('next')"><IconPlayerTrackNext /></button>
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
          <div
            v-for="line in lyricLines"
            :key="`${line.time}-${line.text}`"
            v-motion
            class="lyric-line"
            :class="{ 'is-active': line.active }"
            :initial="{ opacity: 0, y: 6 }"
            :enter="{
              opacity: line.active ? 1 : 0.4,
              y: 0,
              transition: { type: 'spring', stiffness: 260, damping: 28 },
            }"
          >
            {{ line.text }}
          </div>
        </div>
        </div>
      </template>
  
      <template v-else-if="mode === 'timer'">
        <div class="activity-top">
          <span class="timer-ring"><IconClock /></span>
          <span class="media-copy"><strong>Temporizador</strong><small>{{ timerRunning ? "Cuenta atrás en curso" : "En pausa" }}</small></span>
          <button
            class="media-control play-control"
            :aria-label="timerRunning ? 'Pausar temporizador' : 'Reanudar temporizador'"
            @click="emit('toggle-timer')"
          >
            <IconPlayerPause v-if="timerRunning" />
            <IconPlayerPlay v-else />
          </button>
        </div>
        <div class="timer-summary"><span>COCINA</span><strong>{{ timerLabel }}</strong></div>
      </template>
  
      <template v-else-if="mode === 'call'">
        <div class="activity-top">
          <span class="contact-avatar">A</span>
          <span class="media-copy"><strong>Alex Morgan</strong><small :class="{ 'call-ended': !callActive }">{{ callActive ? `En llamada · ${callLabel}` : "Llamada finalizada" }}</small></span>
          <span class="call-status"><i></i> {{ callActive ? "ACTIVA" : "FINALIZADA" }}</span>
        </div>
        <div v-if="callActive" class="call-controls">
          <button class="media-control" :aria-label="muted ? 'Activar micrófono' : 'Silenciar llamada'" @click="emit('toggle-mute')">
            <IconMicrophone v-if="!muted" />
            <IconX v-else />
          </button>
          <button class="media-control end-call" aria-label="Finalizar llamada" @click="emit('end-call')"><IconPhone /></button>
          <button class="media-control" aria-label="Activar altavoz"><IconVolume2 /></button>
        </div>
        <div v-else class="call-ended-note">
          Llamada finalizada
          <button @click="callActive = true">Volver a llamar</button>
        </div>
      </template>
  
      <template v-else>
        <div class="activity-top">
          <span class="message-avatar"><IconMessage /></span>
          <span class="media-copy"><strong>Alex · Mensajes</strong><small>ahora mismo</small></span>
          <button class="dismiss-button" aria-label="Cerrar notificación" @click="chooseActivity('music')"><IconX /></button>
        </div>
        <p class="message-body">¡Ya llegué! Te espero en la entrada 😊</p>
        <button class="reply-button" @click="chooseActivity('music')">Marcar como leído <span>↗</span></button>
      </template>
    </div>
  </div>
  
</template>
