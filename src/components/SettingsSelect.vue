<script setup lang="ts" generic="T extends string | number">
/**
 * Desplegable del panel de ajustes.
 *
 * Sustituye al `<select>` nativo, cuya lista abierta la dibuja Windows y no se
 * puede estilizar (salía con el gris del sistema y un borde verde que no
 * pegaba con el panel). Usa las primitivas de Reka UI, que traen el teclado
 * (flechas, Enter, Esc, escribir para saltar) y el foco accesibles.
 *
 * Los colores van escritos aquí y no como variables del panel: la lista se
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
    <SelectTrigger :id="id" class="ui-select-trigger">
      <SelectValue />
      <svg class="ui-select-chevron" viewBox="0 0 10 6" aria-hidden="true">
        <path d="M1 1l4 4 4-4" />
      </svg>
    </SelectTrigger>

    <SelectPortal>
      <SelectContent class="ui-select-content" position="popper" :side-offset="6" align="end">
        <SelectViewport>
          <SelectItem
            v-for="option in options"
            :key="option.value"
            :value="option.value"
            class="ui-select-item"
          >
            <SelectItemText>{{ option.label }}</SelectItemText>
            <SelectItemIndicator class="ui-select-check">
              <svg viewBox="0 0 12 10" aria-hidden="true"><path d="M1 5.2l3.2 3.2L11 1.6" /></svg>
            </SelectItemIndicator>
          </SelectItem>
        </SelectViewport>
      </SelectContent>
    </SelectPortal>
  </SelectRoot>
</template>

<style>
.ui-select-trigger {
  display: inline-flex;
  flex: none;
  white-space: nowrap;
  align-items: center;
  gap: 10px;
  padding: 6px 10px 6px 12px;
  color: #ececf1;
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 7px;
  background: #1a1d27;
  font: inherit;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.ui-select-trigger:hover {
  border-color: rgba(255, 255, 255, 0.16);
}

.ui-select-trigger[data-state="open"] {
  border-color: rgba(255, 255, 255, 0.24);
  background: #1f2330;
}

.ui-select-trigger:focus-visible {
  outline: 2px solid #b7f36b;
  outline-offset: 2px;
}

.ui-select-chevron {
  width: 10px;
  height: 6px;
  flex: none;
  fill: none;
  stroke: #7f8292;
  stroke-width: 1.4;
  stroke-linecap: round;
  stroke-linejoin: round;
  transition: transform 0.18s;
}

.ui-select-trigger[data-state="open"] .ui-select-chevron {
  transform: rotate(180deg);
}

/* --- Lista --- */
.ui-select-content {
  z-index: 50;
  min-width: var(--reka-select-trigger-width);
  padding: 4px;
  color: #ececf1;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 10px;
  background: #171a24;
  box-shadow: 0 14px 34px rgba(0, 0, 0, 0.55);
  font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  font-size: 13px;
  transform-origin: var(--reka-select-content-transform-origin);
}

.ui-select-content[data-state="open"] {
  animation: ui-select-in 0.14s cubic-bezier(0.22, 1, 0.36, 1);
}

@keyframes ui-select-in {
  from {
    opacity: 0;
    transform: scale(0.97) translateY(-3px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

.ui-select-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 7px 10px;
  border-radius: 7px;
  outline: none;
  cursor: pointer;
  user-select: none;
}

.ui-select-item[data-highlighted] {
  background: rgba(255, 255, 255, 0.07);
}

/* La opción elegida lleva una marca en el verde de acento. */
.ui-select-item[data-state="checked"] {
  font-weight: 550;
}

.ui-select-check {
  display: grid;
  place-items: center;
}

.ui-select-check svg {
  width: 11px;
  height: 9px;
  fill: none;
  stroke: #b7f36b;
  stroke-width: 1.6;
  stroke-linecap: round;
  stroke-linejoin: round;
}

@media (prefers-reduced-motion: reduce) {
  .ui-select-content[data-state="open"] {
    animation: none;
  }

  .ui-select-chevron,
  .ui-select-trigger {
    transition-duration: 0.01ms;
  }
}
</style>
