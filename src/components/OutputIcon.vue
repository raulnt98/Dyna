<script setup lang="ts">
/**
 * Icono de una salida de audio según su tipo (`kind` de `AudioOutput` en outputs.rs): auriculares, portátil, televisor,
 * monitor o altavoz. Lo comparten el botón del reproductor y las filas del selector. Cualquier tipo desconocido
 * cae en el altavoz.
 */
import { IconDeviceDesktop, IconDeviceLaptop, IconDeviceSpeaker, IconDeviceTv, IconHeadphones } from "@tabler/icons-vue";

defineProps<{ kind?: string }>();
</script>

<template>
  <IconHeadphones v-if="kind === 'headphones'" />
  <IconDeviceLaptop v-else-if="kind === 'laptop'" />
  <IconDeviceTv v-else-if="kind === 'tv'" />
  <IconDeviceDesktop v-else-if="kind === 'monitor'" />
  <IconDeviceSpeaker v-else />
</template>
