<script setup lang="ts">
/**
 * Línea de texto que se desplaza cuando no cabe.
 *
 * La píldora de una notificación tiene una sola fila para el mensaje y no puede
 * crecer (la ventana nativa y su recorte van a tamaño fijo). En vez de cortar con
 * «…», un texto más ancho que la fila espera un instante y se desliza hasta
 * enseñar el final. Si cabe, se queda quieto.
 *
 * La distancia se **mide** aquí (no se estima como en los tamaños de la isla):
 * es el contenido ya dibujado, no la ventana. Se vuelve a medir cuando la fila
 * cambia de ancho, porque el panel nace dentro de una píldora que todavía se
 * está ensanchando; el desplazamiento arranca con retraso para que esa medida
 * haya llegado ya a su valor final.
 */
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{ text: string }>();

/**
 * Píxeles por segundo. En sincronía con la velocidad que supone `toastDuration`
 * en IslandApp.vue, que decide cuánto se queda el aviso: si cambias una, cambia
 * la otra, o el aviso se retira antes de llegar al final.
 */
const SPEED = 80;

const box = ref<HTMLElement | null>(null);
const line = ref<HTMLElement | null>(null);
/** Cuánto sobra de texto a la derecha, en px; 0 si cabe. */
const overflow = ref(0);

function measure() {
  if (!box.value || !line.value) return;
  overflow.value = Math.max(0, Math.ceil(line.value.scrollWidth - box.value.clientWidth));
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
  <span
    ref="box"
    class="block overflow-hidden text-[11.5px] leading-[1.45] whitespace-nowrap text-[#d4d4db]"
    :class="overflow > 0 && '[mask-image:linear-gradient(90deg,#000_calc(100%-22px),transparent)]'"
  >
    <span
      ref="line"
      class="inline-block"
      :class="overflow > 0 && 'animate-[toast-marquee_var(--dur)_linear_.9s_both]'"
      :style="overflow > 0 ? { '--shift': `-${overflow + 22}px`, '--dur': `${Math.max(1, (overflow + 22) / SPEED)}s` } : undefined"
    >{{ text }}</span>
  </span>
</template>
