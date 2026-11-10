/**
 * Estado eléctrico del equipo, leído del backend.
 *
 * El sondeo de Rust solo avisa cuando cambia el enchufe, así que el estado
 * inicial se pide una vez al arrancar.
 */
import { ref } from "vue";
import { isDesktopApp } from "@/lib/media";

/** Refleja la struct `PowerState` de `power.rs`, que serializa en camelCase. */
export interface PowerState {
  plugged: boolean;
  percent: number;
  hasBattery: boolean;
}

const EMPTY: PowerState = { plugged: false, percent: 0, hasBattery: false };

/**
 * Observa la alimentación y llama a `onPlugChange` cuando cambia el enchufe.
 *
 * El primer estado no dispara el aviso: arrancar con el cargador puesto no es
 * una novedad que merezca interrumpir lo que la isla esté mostrando.
 */
export function useSystemPower(onPlugChange: (state: PowerState) => void) {
  const state = ref<PowerState>({ ...EMPTY });

  if (!isDesktopApp()) return { state };

  void (async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");

      const initial = await invoke<PowerState>("power_state");
      if (initial) state.value = initial;

      await listen<PowerState>("power://changed", (event) => {
        if (!event.payload) return;
        state.value = event.payload;
        if (event.payload.hasBattery) onPlugChange(event.payload);
      });
    } catch {
      // Fuera de la app de escritorio no hay estado eléctrico que leer.
    }
  })();

  return { state };
}
