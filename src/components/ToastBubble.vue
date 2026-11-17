<script setup lang="ts">
/**
 * Burbuja de notificación: un círculo con solo el icono de la app, que brota del costado derecho de la isla.
 *
 * Es la alternativa discreta al panel: no toca lo que la isla muestre ni cambia su tamaño. Se coloca respecto a
 * `.island-wrap` (que la contiene), del mismo color que la isla, y **nace desde dentro de ella**: sale deslizándose
 * desde detrás mientras crece, como una gota que la isla suelta. Rust recorta la ventana con el mismo círculo y la misma curva (`island_bubble`, `BUBBLE_SIZE` y
 * `BUBBLE_GAP` en lib.rs): si cambias una medida, cambia la otra.
 *
 * Nunca se re-monta mientras está a la vista: el recorte de la ventana va en 1 y un círculo que volviera a
 * empezar desde 0 dejaría asomar la zona de más (Windows la pinta de pálido). Un aviso nuevo solo cambia el
 * contenido, con un pequeño «pop» en el icono.
 */
import { computed, ref, watch } from "vue";
import NoticeIcon from "@/components/NoticeIcon.vue";
import { trimLogo } from "@/lib/trimLogo";

/** Color del icono de un aviso del sistema, el mismo que lleva en su panel (`NOTICE_TONE` en TheIsland.vue). */
const KIND_COLOR: Record<string, string> = {
  power: "text-green",
  battery: "text-[#ff8a70]",
  internet: "text-[#ff8a70]",
  wifi: "text-[#6eb4ff]",
  bluetooth: "text-[#6eb4ff]",
  "bluetooth-off": "text-[#b4b4bd]",
  caps: "text-[#f4cb82]",
  num: "text-[#f4cb82]",
  focus: "text-[#a99bff]",
  brightness: "text-[#f4cb82]",
};

/**
 * Ajuste óptico por tipo: el dibujo de algunos iconos no está centrado en su caja (el altavoz tiene el cuerpo a la izquierda y las
 * ondas a la derecha, la batería está en la mitad de abajo…) y se ve descentrado dentro del círculo. Valores en px, medidos
 * con los píxeles reales (centro de la caja que envuelve el dibujo frente al centro del círculo; `cdp-center`) y aplicados
 * solo al icono, nunca al tachado, que va siempre en el centro. El icono mide 20 px, par, para que su centro caiga en un
 * cruce de píxeles del círculo de 34: con 19 px caía a medio píxel y todo salía +0,5 px abajo y a la derecha.
 */
const NUDGE: Record<string, string> = {
  battery: "0 -2.1px",
  volume: "1.75px 0", // solo el altavoz con ondas: el tachado del silencio ya viene centrado
  bluetooth: "0.4px 0",
  focus: "0.4px 0",
  wifi: "0 -0.25px",
  caps: "0 -0.25px",
};

const props = defineProps<{
  shown: boolean;
  /** Nombre de la app, o el texto del aviso del sistema: para el lector de pantalla. */
  app: string;
  /** Logo de la app (data URI); sin él se usa la inicial. */
  icon: string;
  /** Si es un aviso del sistema, su tipo: dibuja el icono de ese aviso en lugar del logo de una app. */
  kind?: string;
  muted?: boolean;
  /** El aviso dice que algo se apagó o se retiró (cargador desconectado, Bloq Mayús desactivado…): icono gris tachado. */
  off?: boolean;
  /** Volumen y brillo: nivel 0–100, que se dibuja como un borde que se va llenando alrededor de la burbuja. */
  level?: number;
  /** Cambia con cada aviso, para repetir el «pop» del icono. */
  tick: number;
  /** Sube con cada aviso que repite el contenido de la burbuja (el volumen o el brillo pulsados varias veces): un «respiro» por pulsación. */
  pulse?: number;
}>();
const emit = defineEmits<{ (e: "open"): void; (e: "enter"): void; (e: "leave"): void }>();

const logo = ref("");
watch(
  () => props.icon,
  async (icon) => {
    logo.value = "";
    if (!icon) return;
    const trimmed = await trimLogo(icon);
    if (props.icon === icon) logo.value = trimmed;
  },
  { immediate: true },
);
const initial = computed(() => props.app.trim().charAt(0).toUpperCase() || "!");

/**
 * Un respiro por pulsación: el icono se hincha y vuelve, y un velo de luz recorre el interior del círculo. Va por dentro
 * porque la burbuja no puede crecer: el recorte de la ventana (Rust) vale lo que su tamaño final y lo de fuera se perdería.
 * Se anima con `scale` (la propiedad suelta) para no pisar el `transform` del icono, y con la API de animaciones para que
 * cada pulsación la reinicie aunque la anterior no haya acabado.
 */
const root = ref<HTMLElement | null>(null);
const glow = ref<HTMLElement | null>(null);
watch(
  () => props.pulse,
  () => {
    if (!props.shown || !root.value) return;
    // Misma duración y misma curva que la apertura y el cierre de la burbuja (640 ms, `.toast-bubble`): un respiro lento y
    // poco profundo, no un latido. El pico cae a media animación para que inhalar y exhalar duren lo mismo.
    const timing = { duration: 640, easing: "cubic-bezier(.4, 0, .2, 1)" };
    root.value.querySelector(".toast-bubble-icon")?.animate([{ scale: "1" }, { scale: "1.14", offset: 0.5 }, { scale: "1" }], timing);
    glow.value?.animate([{ opacity: 0 }, { opacity: 0.6, offset: 0.5 }, { opacity: 0 }], timing);
  },
);
</script>

<template>
  <button
    ref="root"
    class="toast-bubble absolute grid cursor-pointer place-items-center overflow-hidden rounded-full border-0 p-0"
    :class="shown ? 'is-shown' : 'pointer-events-none'"
    :aria-label="kind ? app : `Notificación de ${app}`"
    :tabindex="shown ? 0 : -1"
    @click.stop="emit('open')"
    @mouseenter="emit('enter')"
    @mouseleave="emit('leave')"
  >
    <span ref="glow" class="pointer-events-none absolute inset-0 rounded-full opacity-0 shadow-[inset_0_0_0_2px_rgba(255,255,255,.22),inset_0_0_10px_rgba(255,255,255,.14)]" aria-hidden="true"></span>
    <!-- Volumen y brillo: un borde que se va dibujando alrededor del círculo, en el sentido de las agujas del reloj desde arriba, según
         el nivel; con un fundido de la misma duración y curva que el resto de la burbuja (`.toast-bubble-ring`). Vacío con el silencio. -->
    <span
      v-if="level !== undefined && (kind === 'volume' || kind === 'brightness')"
      class="toast-bubble-ring pointer-events-none absolute rounded-full"
      :style="{ '--ring': `${muted ? 0 : Math.min(100, Math.max(0, level))}%`, '--ring-color': kind === 'brightness' ? '#f4cb82' : '#fff' }"
      aria-hidden="true"
    ></span>
    <span v-if="kind" :key="tick" class="toast-bubble-icon relative grid place-items-center [&>svg]:size-5" :class="off || muted ? 'text-[#8d8d98]' : (KIND_COLOR[kind] ?? 'text-white')">
      <NoticeIcon :kind="kind" :muted="muted === true" :style="NUDGE[kind] && !(kind === 'volume' && muted) ? { translate: NUDGE[kind] } : undefined" />
      <!-- Tachado: lo que distingue «apagado» de «encendido» sin depender solo del color. El borde oscuro lo separa del icono. -->
      <i v-if="off" class="absolute top-1/2 left-1/2 h-[2px] w-6 -translate-x-1/2 -translate-y-1/2 -rotate-45 rounded-full bg-current shadow-[0_0_0_1.5px_#050506]" aria-hidden="true"></i>
    </span>
    <img v-else-if="logo" :key="tick" :src="logo" alt="" class="toast-bubble-icon relative size-[22px] object-contain" />
    <span v-else :key="tick" class="toast-bubble-icon relative text-[15px] font-bold text-green" style="translate: -0.4px 0.4px">{{ initial }}</span>
  </button>
</template>
