<script setup lang="ts">
/**
 * Icono de salto de N segundos, sólido.
 *
 * Los iconos de salto de Tabler son de trazo fino, huecos, y junto a los de
 * reproducir y pausa rellenos se veían de otro idioma. Este dibuja el arco con un
 * trazo grueso, la punta de la flecha rellena y el número en negrita. El de
 * retroceder es el de avanzar reflejado; el número no se refleja.
 */
defineProps<{
  seconds: number;
  /** `true` para retroceder, `false` para avanzar. */
  back?: boolean;
}>();
</script>

<template>
  <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <g :transform="back ? 'translate(24 0) scale(-1 1)' : undefined">
      <path d="M18.58 11.11A7 7 0 1 1 13.22 6.61" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" />
      <path d="M16.9 7.35 11.9 9.9 12.9 3.4Z" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round" />
    </g>
    <text
      x="12"
      y="16.4"
      text-anchor="middle"
      font-size="8"
      font-weight="800"
      fill="currentColor"
      font-family="Inter, system-ui, sans-serif"
    >{{ seconds }}</text>
  </svg>
</template>
