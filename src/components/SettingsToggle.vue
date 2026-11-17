<script setup lang="ts">
/**
 * Fila del panel de ajustes con un interruptor.
 *
 * `sub` sangra la fila: son las opciones que dependen de la de arriba. Con
 * `disabled` se atenúa y deja de responder.
 */
defineProps<{
  name: string;
  desc?: string;
  sub?: boolean;
  disabled?: boolean;
}>();

const model = defineModel<boolean>({ required: true });
</script>

<template>
  <label
    class="m-0 flex items-center justify-between gap-4 py-3 pr-3.5"
    :class="[sub ? 'pl-7' : 'pl-3.5', disabled ? 'cursor-default opacity-[0.42]' : 'cursor-pointer']"
  >
    <span class="grid min-w-0 gap-0.5">
      <span :class="sub ? 'font-[450]' : 'font-medium'">{{ name }}</span>
      <span v-if="desc" class="text-[12.5px] leading-[1.35] text-muted">{{ desc }}</span>
    </span>
    <!-- Control extra junto al interruptor (p. ej. el formato de cada app). Van en un grupo a la derecha: como tercer hijo suelto,
         `justify-between` lo dejaba a mitad del hueco y cambiaba de sitio según el ancho del nombre. Un desplegable dentro del
         `<label>` no activa el interruptor: Reka UI pinta su lista en un portal, fuera de la fila. -->
    <span class="flex flex-none items-center gap-3">
    <slot />
    <input
      v-model="model"
      type="checkbox"
      :disabled="disabled"
      class="relative m-0 h-[22px] w-[38px] flex-none cursor-pointer appearance-none rounded-full bg-control transition-colors duration-[180ms] checked:bg-accent disabled:cursor-default focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent motion-reduce:duration-[0.01ms] after:absolute after:top-[2px] after:left-[2px] after:size-[18px] after:rounded-full after:bg-white after:shadow-[0_1px_2px_rgba(0,0,0,0.28)] after:transition-[translate,background-color] after:duration-[180ms] after:ease-[cubic-bezier(0.22,1,0.36,1)] after:content-[''] checked:after:translate-x-4 checked:after:bg-white motion-reduce:after:duration-[0.01ms]"
    />
    </span>
  </label>
</template>
