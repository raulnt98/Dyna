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
  IconArrowBigUpLine,
  IconBluetoothConnected,
  IconBluetoothOff,
  IconBolt,
  IconDownload,
  IconGauge,
  IconMicrophoneFilled,
  IconVideoFilled,
  IconMoon,
  IconMusic,
  IconNumber123,
  IconSun,
  IconUsb,
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
  <IconBluetoothConnected v-else-if="kind === 'bluetooth'" />
  <IconBluetoothOff v-else-if="kind === 'bluetooth-off'" />
  <IconUsb v-else-if="kind === 'usb'" />
  <IconArrowBigUpLine v-else-if="kind === 'caps'" />
  <IconNumber123 v-else-if="kind === 'num'" />
  <IconMoon v-else-if="kind === 'focus'" />
  <IconSun v-else-if="kind === 'brightness'" />
  <IconMusic v-else-if="kind === 'track'" />
  <IconMicrophoneFilled v-else-if="kind === 'mic'" />
  <IconVideoFilled v-else-if="kind === 'camera'" />
  <IconDownload v-else-if="kind === 'download'" />
  <IconGauge v-else-if="kind === 'speed'" />
  <IconBell v-else />
</template>
