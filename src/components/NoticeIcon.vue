<script setup lang="ts">
/**
 * Icono de un aviso según su tipo.
 *
 * Lo comparten los tres sitios donde se dibuja un aviso (píldora compacta,
 * franja dentro del panel y panel desplegado), para que añadir un tipo nuevo
 * sea tocar un solo lugar.
 */
import {
  IconBatteryExclamation,
  IconBell,
  IconBolt,
  IconVolume2,
  IconVolumeOff,
  IconWifiOff,
} from "@tabler/icons-vue";
import WifiGlyph from "@/components/WifiGlyph.vue";

defineProps<{
  kind: string;
  /** Solo para el volumen: silencio o nivel cero. */
  muted?: boolean;
}>();
</script>

<template>
  <IconBolt v-if="kind === 'power'" />
  <IconVolumeOff v-else-if="kind === 'volume' && muted" />
  <IconVolume2 v-else-if="kind === 'volume'" />
  <IconBatteryExclamation v-else-if="kind === 'battery'" />
  <WifiGlyph v-else-if="kind === 'wifi'" />
  <IconWifiOff v-else-if="kind === 'internet'" />
  <IconBell v-else />
</template>
