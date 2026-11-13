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
    class="m-0 flex items-center justify-between gap-4 py-[11px] pr-3.5"
    :class="[sub ? 'pl-7' : 'pl-3.5', disabled ? 'cursor-default opacity-[0.42]' : 'cursor-pointer']"
  >
    <span class="grid min-w-0 gap-0.5">
      <span :class="sub ? 'font-[450]' : 'font-medium'">{{ name }}</span>
      <span v-if="desc" class="text-[11.5px] leading-[1.4] text-muted">{{ desc }}</span>
    </span>
    <input
      v-model="model"
      type="checkbox"
      :disabled="disabled"
      class="relative m-0 h-5 w-[34px] flex-none cursor-pointer appearance-none rounded-full bg-control transition-colors duration-[180ms] checked:bg-accent disabled:cursor-default focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent motion-reduce:duration-[0.01ms] after:absolute after:top-[3px] after:left-[3px] after:size-3.5 after:rounded-full after:bg-[#c9ccd8] after:transition-[translate,background-color] after:duration-[180ms] after:ease-[cubic-bezier(0.22,1,0.36,1)] after:content-[''] checked:after:translate-x-3.5 checked:after:bg-bg motion-reduce:after:duration-[0.01ms]"
    />
  </label>
</template>
