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
  /** Más bajo y de letra más pequeña, para las filas de segundo nivel (el formato de cada app). */
  compact?: boolean;
}>();

const model = defineModel<T>({ required: true });
</script>

<template>
  <SelectRoot v-model="model">
    <SelectTrigger :id="id" :class="compact ? 'gap-1.5 py-[3px] pr-2 pl-2.5 text-[13px]' : 'gap-2.5 py-1.5 pr-2.5 pl-3'" class="group inline-flex flex-none cursor-pointer items-center rounded-[7px] border border-line bg-field whitespace-nowrap text-ink transition-[border-color,background-color] duration-150 hover:border-edge focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent data-[state=open]:border-edge data-[state=open]:bg-white motion-reduce:duration-[0.01ms]">
      <SelectValue />
      <svg class="h-3 w-2.5 flex-none fill-none stroke-[#686a74] stroke-[1.4] [stroke-linecap:round] [stroke-linejoin:round]" viewBox="0 0 10 12" aria-hidden="true">
        <path d="M2 4.6L5 1.8l3 2.8M2 7.4l3 2.8 3-2.8" />
      </svg>
    </SelectTrigger>

    <SelectPortal>
      <SelectContent class="z-50 min-w-(--reka-select-trigger-width) origin-(--reka-select-content-transform-origin) rounded-[10px] border border-line bg-[#fbfbfc] p-1 font-sans text-[14px] text-ink shadow-[0_14px_34px_rgba(30,32,44,0.22)] data-[state=open]:animate-select-in data-[state=closed]:animate-select-out motion-reduce:animate-none" position="popper" :side-offset="6" align="end" :body-lock="false">
        <SelectViewport>
          <SelectItem
            v-for="option in options"
            :key="option.value"
            :value="option.value"
            class="flex cursor-pointer items-center justify-between gap-[18px] rounded-[7px] px-2.5 py-[7px] outline-none select-none data-highlighted:bg-hover data-[state=checked]:font-[550]"
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
