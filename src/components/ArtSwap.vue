<script setup lang="ts">
/**
 * Carátula que cambia con un giro, un desenfoque y un destello.
 *
 * Al cambiar de pista la imagen vieja gira, se desenfoca y se desvanece mientras la
 * nueva entra girando en sentido contrario, y un destello blanco breve las une. Es
 * lo que evita que la carátula «salte» de golpe. Las dos capas conviven un instante
 * (modo por defecto de `Transition`), por eso son absolutas y el contenedor, que es
 * quien recorta con su radio, debe tener `overflow-hidden`.
 *
 * No anima la primera vez que se pinta: al abrir la isla la carátula ya está.
 */
import { ref, watch } from "vue";

const props = defineProps<{
  /** Carátula como data URI; vacía si la pista no tiene. */
  src: string;
  /** Letra que se enseña mientras no hay carátula. */
  initial: string;
}>();

/** Cuenta los cambios: cada uno monta un destello nuevo y reinicia su animación. */
const flash = ref(0);
watch(
  () => props.src,
  () => {
    flash.value++;
  },
);
</script>

<template>
  <span class="art-swap">
    <Transition name="art-swap">
      <img
        v-if="src"
        :key="src"
        :src="src"
        alt=""
        class="art-swap-layer block size-full object-cover [image-rendering:auto]"
      />
      <span v-else key="initial" class="art-swap-layer grid size-full place-items-center">{{ initial }}</span>
    </Transition>
    <span v-if="flash" :key="flash" class="art-flash" aria-hidden="true"></span>
  </span>
</template>
