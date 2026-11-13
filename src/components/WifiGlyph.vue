<script setup lang="ts">
/**
 * Icono de Wi-Fi cuyos arcos se encienden de dentro hacia fuera.
 *
 * Se dibuja a mano en lugar de usar `IconWifi` de Tabler porque cada arco
 * necesita su propio elemento para animarse por separado; las reglas viven en
 * style.css, junto al resto de los avisos.
 */
</script>

<template>
  <svg class="wifi-glyph" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path class="wifi-arc wifi-arc-3" d="M3.515 9.515c4.686 -4.687 12.284 -4.687 17 0" />
    <path class="wifi-arc wifi-arc-2" d="M6.343 12.343a8 8 0 0 1 11.314 0" />
    <path class="wifi-arc wifi-arc-1" d="M9.172 15.172a4 4 0 0 1 5.656 0" />
    <path class="wifi-dot" d="M12 18.5l.01 0" />
  </svg>
</template>
