/**
 * Canal del audio en tiempo real hacia las barras de la onda.
 *
 * Los niveles llegan unas 60 veces por segundo. Si fueran estado reactivo de Vue (una prop, una
 * `ref`), cada llegada volvería a pintar `IslandApp` y la isla entera 60 veces por segundo para mover
 * ocho barras. Por eso los niveles viven aquí **fuera de la reactividad**, en un objeto que se muta en
 * sitio, y cada `WaveBars` los lee en su propio bucle de animación (ver WaveBars.vue). Lo único
 * reactivo es lo que cambia rara vez: si hay audio en directo y cuántas barras trae.
 */
import { ref } from "vue";

/** Hay niveles en directo (ajuste activo y datos llegando). */
export const audioLive = ref(false);
/** Cuántas barras traen los niveles; el backend manda ocho. */
export const audioBarCount = ref(5);

/** Último bloque de niveles (0–1, de grave a agudo), o null. Se lee en cada fotograma; no es reactivo a propósito. */
export const audioFeed: { levels: number[] | null } = { levels: null };

/** Recibe un bloque de niveles del backend. */
export function pushAudioLevels(levels: number[]) {
  audioFeed.levels = levels;
  if (!audioLive.value) audioLive.value = true;
  if (audioBarCount.value !== levels.length) audioBarCount.value = levels.length;
}

/** Corta el audio en directo: las barras vuelven a animarse solas. */
export function clearAudioLevels() {
  audioFeed.levels = null;
  audioLive.value = false;
}
