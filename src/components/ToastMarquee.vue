<script setup lang="ts">
/**
 * Mensaje de una notificación: hasta tres líneas, y si pasa de ahí, se desplaza.
 *
 * La isla de una notificación mide lo mismo que el panel (330 × 108) y no puede
 * crecer más: la ventana nativa y su recorte van a tamaño fijo. El mensaje se
 * ajusta a tres líneas; un texto que necesita más espera un instante y sube hasta
 * enseñar el final, en lugar de cortarse con «…». Si cabe, se queda quieto.
 *
 * La distancia se **mide** aquí (no se estima como en los tamaños de la isla): es
 * el contenido ya dibujado, no la ventana. Se vuelve a medir cuando la caja cambia
 * de tamaño, porque el panel nace dentro de una isla que todavía se está
 * ensanchando; el desplazamiento arranca con retraso para que esa medida haya
 * llegado ya a su valor final.
 */
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{ text: string }>();

/**
 * Píxeles por segundo, hacia arriba. Equivale a una línea cada 1,2 s, que es un
 * ritmo de lectura cómodo. En sincronía con lo que supone `toastDuration` en
 * IslandApp.vue, que decide cuánto se queda el aviso: si cambias una, cambia la
 * otra, o el aviso se retira antes de llegar al final.
 */
const SPEED = 14;

const box = ref<HTMLElement | null>(null);
const body = ref<HTMLElement | null>(null);
/** Cuánto sobra de texto por abajo, en px; 0 si cabe. */
const overflow = ref(0);

function measure() {
  if (!box.value || !body.value) return;
  overflow.value = Math.max(0, Math.ceil(body.value.scrollHeight - box.value.clientHeight));
}

let observer: ResizeObserver | undefined;
onMounted(() => {
  measure();
  if (box.value && typeof ResizeObserver !== "undefined") {
    observer = new ResizeObserver(measure);
    observer.observe(box.value);
  }
});
onBeforeUnmount(() => observer?.disconnect());
watch(() => props.text, () => nextTick(measure));
</script>

<template>
  <!-- Tres líneas: 3 × 1,3 em. Con menos texto la caja mide lo que el texto. -->
  <span ref="box" class="block max-h-[calc(1.3em*3)] overflow-hidden text-[12.5px] leading-[1.3] text-[#d4d4db]">
    <span
      ref="body"
      class="block [overflow-wrap:anywhere]"
      :class="overflow > 0 && 'animate-[toast-marquee_var(--dur)_linear_.9s_both]'"
      :style="overflow > 0 ? { '--shift': `-${overflow}px`, '--dur': `${Math.max(1, overflow / SPEED)}s` } : undefined"
    >{{ text }}</span>
  </span>
</template>
