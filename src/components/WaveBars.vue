<script setup lang="ts">
/**
 * Barras de la onda de sonido.
 *
 * Sin audio en directo son cinco barras que se animan solas con CSS (style.css). Con él son las que
 * lleguen (ocho) y se mueven aquí, en un bucle de `requestAnimationFrame` a la frecuencia de la pantalla
 * (60, 120, 144 Hz), con **suavizado propio**: sube rápido y baja más despacio, medido en tiempo y no en
 * fotogramas, de modo que se ve igual de fluido aunque los datos lleguen a ráfagas o a otro ritmo.
 *
 * El movimiento es un `transform: scaleY`, que no recalcula el diseño ni repinta nada más: la barra mide
 * siempre lo máximo y se encoge. Los niveles se leen de `audioFeed`, fuera de la reactividad: si pasaran
 * por una prop, cada llegada volvería a pintar la isla entera.
 *
 * Es un fragmento (varios `<i>`) que va dentro del `.waveform` de cada sitio, que fija su tamaño con la
 * variable `--wave-max`.
 */
import { computed, onBeforeUnmount, onMounted, watch } from "vue";
import { audioBarCount, audioFeed, audioLive } from "@/lib/audioFeed";

/** Barras que se dibujan: las del audio en directo, o las cinco de siempre. */
const count = computed(() => (audioLive.value ? audioBarCount.value : 5));

/** Constantes de tiempo del suavizado, en milisegundos: sube rápido, baja despacio. */
const ATTACK_MS = 26;
const RELEASE_MS = 105;
/** Lo mínimo que mide una barra, en píxeles (el «suelo» del silencio). */
const FLOOR_PX = 3;

const els: (HTMLElement | null)[] = [];
const shown: number[] = [];
let frame = 0;
let maxPx = 14;
let last = 0;

function tick(now: number) {
  // Si la ventana estuvo oculta, el primer fotograma trae un salto enorme: se acota.
  const dt = Math.min(64, now - last);
  last = now;
  const target = audioFeed.levels;
  for (let i = 0; i < els.length; i++) {
    const el = els[i];
    if (!el) continue;
    const goal = target?.[i] ?? 0;
    const current = shown[i] ?? 0;
    const tau = goal > current ? ATTACK_MS : RELEASE_MS;
    const next = current + (goal - current) * (1 - Math.exp(-dt / tau));
    shown[i] = next;
    // La barra mide FLOOR_PX en silencio y maxPx + FLOOR_PX con el nivel al máximo.
    const scale = (FLOOR_PX + next * maxPx) / (FLOOR_PX + maxPx);
    el.style.transform = "scaleY(" + scale.toFixed(3) + ")";
  }
  frame = requestAnimationFrame(tick);
}

/** Prepara las barras para el modo en directo, o las devuelve al CSS. */
function sync() {
  cancelAnimationFrame(frame);
  frame = 0;
  const live = audioLive.value;
  for (const el of els) {
    if (!el) continue;
    if (live) {
      // `--wave-max` es el recorrido de la barra en este sitio (style.css).
      const parsed = parseFloat(getComputedStyle(el).getPropertyValue("--wave-max"));
      maxPx = Number.isFinite(parsed) ? parsed : 14;
      el.style.height = maxPx + FLOOR_PX + "px";
      el.style.animation = "none";
      el.style.transformOrigin = "center";
      el.style.willChange = "transform";
    } else {
      el.style.removeProperty("height");
      el.style.removeProperty("animation");
      el.style.removeProperty("transform");
      el.style.removeProperty("transform-origin");
      el.style.removeProperty("will-change");
    }
  }
  if (live) {
    last = performance.now();
    frame = requestAnimationFrame(tick);
  }
}

function setRef(index: number) {
  return (el: unknown) => {
    els[index] = el as HTMLElement | null;
  };
}

// Al activarse o apagarse el directo, y al cambiar el número de barras (de cinco a ocho): se
// espera al pintado para que los elementos ya existan.
watch([audioLive, count], () => queueMicrotask(sync), { flush: "post" });
// Primera vez, tras el montaje: los elementos ya existen.
onMounted(sync);
onBeforeUnmount(() => cancelAnimationFrame(frame));
</script>

<template>
  <i v-for="n in count" :key="n" :ref="setRef(n - 1)"></i>
</template>
