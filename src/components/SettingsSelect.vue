<script setup lang="ts" generic="T extends string | number">
/**
 * Desplegable del panel de ajustes.
 *
 * Sustituye al `<select>` nativo, cuya lista abierta la dibuja Windows y no se
 * puede estilizar (salía con el gris del sistema y un borde verde que no
 * pegaba con el panel). Usa las primitivas de Reka UI, que traen el teclado
 * (flechas, Enter, Esc, escribir para saltar) y el foco accesibles.
 *
 * Los colores son literales y no variables del panel: la lista se
 * dibuja en un portal, fuera de `.settings`, y allí las variables no existen.
 */
import {
  SelectContent,
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  SelectPortal,
  SelectRoot,
  SelectTrigger,
  SelectValue,
  SelectViewport,
} from "reka-ui";

defineProps<{
  options: { value: T; label: string }[];
  id?: string;
}>();

const model = defineModel<T>({ required: true });
</script>

<template>
  <SelectRoot v-model="model">
    <SelectTrigger :id="id" class="group inline-flex flex-none cursor-pointer items-center gap-2.5 rounded-[7px] border border-white/7 bg-[#1a1d27] py-1.5 pr-2.5 pl-3 whitespace-nowrap text-ink transition-[border-color,background-color] duration-150 hover:border-white/16 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent data-[state=open]:border-white/24 data-[state=open]:bg-[#1f2330] motion-reduce:duration-[0.01ms]">
      <SelectValue />
      <svg class="h-1.5 w-2.5 flex-none fill-none stroke-[#7f8292] stroke-[1.4] transition-transform duration-[180ms] [stroke-linecap:round] [stroke-linejoin:round] group-data-[state=open]:rotate-180 motion-reduce:duration-[0.01ms]" viewBox="0 0 10 6" aria-hidden="true">
        <path d="M1 1l4 4 4-4" />
      </svg>
    </SelectTrigger>

    <SelectPortal>
      <SelectContent class="z-50 min-w-(--reka-select-trigger-width) origin-(--reka-select-content-transform-origin) rounded-[10px] border border-white/10 bg-[#171a24] p-1 font-sans text-[13px] text-ink shadow-[0_14px_34px_rgba(0,0,0,0.55)] data-[state=open]:animate-select-in motion-reduce:animate-none" position="popper" :side-offset="6" align="end">
        <SelectViewport>
          <SelectItem
            v-for="option in options"
            :key="option.value"
            :value="option.value"
            class="flex cursor-pointer items-center justify-between gap-[18px] rounded-[7px] px-2.5 py-[7px] outline-none select-none data-highlighted:bg-white/7 data-[state=checked]:font-[550]"
          >
            <SelectItemText>{{ option.label }}</SelectItemText>
            <SelectItemIndicator class="grid place-items-center">
              <svg class="h-[9px] w-[11px] fill-none stroke-accent stroke-[1.6] [stroke-linecap:round] [stroke-linejoin:round]" viewBox="0 0 12 10" aria-hidden="true"><path d="M1 5.2l3.2 3.2L11 1.6" /></svg>
            </SelectItemIndicator>
          </SelectItem>
        </SelectViewport>
      </SelectContent>
    </SelectPortal>
  </SelectRoot>
</template>
