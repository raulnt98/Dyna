<script setup lang="ts">
/**
 * Panel de configuración de la isla.
 *
 * Vive en su propia ventana, que se abre desde el icono de la bandeja del
 * sistema, de modo que sigue siendo accesible con la isla en segundo plano.
 */
import { onMounted, ref, watch } from "vue";

/** Ajustes que se guardan entre sesiones. */
interface Settings {
  /** Posición horizontal de la isla en la pantalla. */
  position: "left" | "center" | "right";
  /** Separación respecto al borde superior, en píxeles. */
  topMargin: number;
  /** Mantener la isla por encima del resto de ventanas. */
  alwaysOnTop: boolean;
  /** Expandir la isla al pasar el ratón, sin hacer clic. */
  expandOnHover: boolean;
  /** Ocultar la isla cuando no hay nada reproduciéndose. */
  hideWhenIdle: boolean;
  /** Arrancar junto con Windows. */
  autostart: boolean;
  /** Avisar de sucesos del sistema, como conectar el cargador. */
  notifications: boolean;
  /** Buscar letras sincronizadas en LRCLIB. */
  lyrics: boolean;
  /** Segundos que saltan los botones de avance y retroceso. */
  skipSeconds: number;
}

const DEFAULTS: Settings = {
  position: "center",
  topMargin: 14,
  alwaysOnTop: true,
  expandOnHover: false,
  hideWhenIdle: false,
  autostart: false,
  notifications: true,
  lyrics: true,
  skipSeconds: 10,
};

const settings = ref<Settings>({ ...DEFAULTS });
const saved = ref(false);
let loading = true;

/** Llama a un comando de Tauri, ignorando el fallo fuera de la app. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T | undefined> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return undefined;
  }
}

onMounted(async () => {
  const stored = await call<Settings>("settings_get");
  if (stored) settings.value = { ...DEFAULTS, ...stored };
  loading = false;
});

// Cada cambio se guarda y se aplica en el momento, sin botón de confirmar.
watch(
  settings,
  (value) => {
    if (loading) return;
    call("settings_set", { settings: value });
    saved.value = true;
    window.setTimeout(() => (saved.value = false), 1400);
  },
  { deep: true },
);

function restoreDefaults() {
  settings.value = { ...DEFAULTS };
}

function closeWindow() {
  call("settings_close");
}
</script>

<template>
  <div class="settings">
    <header class="settings-head">
      <div>
        <h1>Ajustes de la isla</h1>
        <p>Los cambios se aplican al instante.</p>
      </div>
      <button class="close" aria-label="Cerrar ajustes" @click="closeWindow">×</button>
    </header>

    <section class="group">
      <h2>Posición</h2>

      <div class="row">
        <label for="pos">Alineación horizontal</label>
        <select id="pos" v-model="settings.position">
          <option value="left">Izquierda</option>
          <option value="center">Centro</option>
          <option value="right">Derecha</option>
        </select>
      </div>

      <div class="row">
        <label for="margin">Separación superior</label>
        <div class="slider">
          <input
            id="margin"
            v-model.number="settings.topMargin"
            type="range"
            min="0"
            max="120"
            step="2"
          />
          <span class="value">{{ settings.topMargin }} px</span>
        </div>
      </div>
    </section>

    <section class="group">
      <h2>Reproducción</h2>

      <div class="row">
        <label for="skip">Salto de los botones</label>
        <select id="skip" v-model.number="settings.skipSeconds">
          <option :value="5">5 segundos</option>
          <option :value="10">10 segundos</option>
          <option :value="15">15 segundos</option>
          <option :value="30">30 segundos</option>
        </select>
      </div>
      <p class="hint-text">
        No todos los reproductores permiten avanzar la pista: Apple Music,
        por ejemplo, ignora esta función.
      </p>
    </section>

    <section class="group">
      <h2>Comportamiento</h2>

      <label class="check">
        <input v-model="settings.alwaysOnTop" type="checkbox" />
        <span>
          <strong>Siempre visible</strong>
          <small>Mantiene la isla por encima de otras ventanas.</small>
        </span>
      </label>

      <label class="check">
        <input v-model="settings.expandOnHover" type="checkbox" />
        <span>
          <strong>Expandir al pasar el ratón</strong>
          <small>Sin necesidad de hacer clic en la isla.</small>
        </span>
      </label>

      <label class="check">
        <input v-model="settings.hideWhenIdle" type="checkbox" />
        <span>
          <strong>Ocultar sin reproducción</strong>
          <small>La isla desaparece cuando no suena nada.</small>
        </span>
      </label>

      <label class="check">
        <input v-model="settings.autostart" type="checkbox" />
        <span>
          <strong>Iniciar con Windows</strong>
          <small>La isla se abre al encender el equipo.</small>
        </span>
      </label>

      <label class="check">
        <input v-model="settings.notifications" type="checkbox" />
        <span>
          <strong>Avisos del sistema</strong>
          <small>La isla avisa al conectar o desconectar el cargador.</small>
        </span>
      </label>

      <label class="check">
        <input v-model="settings.lyrics" type="checkbox" />
        <span>
          <strong>Letras sincronizadas</strong>
          <small>Busca la letra en lrclib.net enviando título y artista.</small>
        </span>
      </label>
    </section>

    <footer class="settings-foot">
      <button class="ghost" @click="restoreDefaults">Restaurar valores</button>
      <span class="saved" :class="{ visible: saved }">Guardado</span>
    </footer>
  </div>
</template>

<style scoped>
.settings {
  min-height: 100vh;
  padding: 20px 22px 16px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  color: #f4f4f6;
  background: #0d0f16;
  font-family: "Google Sans", "Segoe UI", system-ui, sans-serif;
}

.settings-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.settings-head h1 {
  margin: 0;
  font-size: 16px;
  font-weight: 650;
}

.settings-head p {
  margin: 3px 0 0;
  color: #8e8e99;
  font-size: 11px;
}

.close {
  width: 26px;
  height: 26px;
  flex: 0 0 auto;
  color: #b9b9c4;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.close:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.12);
}

.group {
  display: grid;
  gap: 11px;
  padding: 14px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 13px;
  background: rgba(255, 255, 255, 0.025);
}

.group h2 {
  margin: 0;
  color: #9a9aa6;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  font-size: 12.5px;
}

select {
  padding: 6px 9px;
  color: #f4f4f6;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  background: #15171f;
  font: inherit;
  cursor: pointer;
}

.slider {
  display: flex;
  align-items: center;
  gap: 10px;
}

input[type="range"] {
  width: 128px;
  accent-color: #b7f36b;
}

.value {
  min-width: 46px;
  color: #9a9aa6;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.check {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  cursor: pointer;
}

.check input {
  margin-top: 2px;
  accent-color: #b7f36b;
}

.check strong {
  display: block;
  font-size: 12.5px;
  font-weight: 550;
}

.check small {
  display: block;
  margin-top: 1px;
  color: #8e8e99;
  font-size: 10.5px;
}

.settings-foot {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.ghost {
  padding: 7px 12px;
  color: #c6c6d0;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 9px;
  background: transparent;
  font: inherit;
  font-size: 11.5px;
  cursor: pointer;
  transition: background 0.15s;
}

.ghost:hover {
  background: rgba(255, 255, 255, 0.07);
}

.saved {
  color: #b7f36b;
  font-size: 11px;
  opacity: 0;
  transition: opacity 0.25s;
}

.saved.visible {
  opacity: 1;
}
</style>
