<script setup lang="ts">
/**
 * Panel de configuración de la isla.
 *
 * Vive en su propia ventana, que se abre desde el icono de la bandeja del
 * sistema, de modo que sigue siendo accesible con la isla en segundo plano.
 */
import { computed, onMounted, ref, watch } from "vue";

/** Ajustes que se guardan entre sesiones. */
interface Settings {
  /** Posición horizontal de la isla en la pantalla. */
  position: "left" | "center" | "right";
  /** Separación respecto al borde superior, en píxeles. */
  topMargin: number;
  /** Mantener la isla por encima del resto de ventanas. */
  alwaysOnTop: boolean;
  /** Arrancar junto con Windows. */
  autostart: boolean;
  /** Interruptor general de los avisos del sistema. */
  notifications: boolean;
  /** Avisar al conectar o desconectar el cargador. */
  notifyCharger: boolean;
  /** Avisar de los cambios de volumen y del silencio. */
  notifyVolume: boolean;
  /** Avisar al bajar al 20 % y al 10 % de batería. */
  notifyLowBattery: boolean;
  /** Buscar letras sincronizadas en LRCLIB. */
  lyrics: boolean;
  /** Mostrar las notificaciones de otras aplicaciones (Teams, Outlook…). */
  appNotifications: boolean;
  /** Aplicaciones detectadas, cada una con su interruptor. */
  notificationApps: { name: string; enabled: boolean }[];
  /** Segundos que saltan los botones de avance y retroceso. */
  skipSeconds: number;
}

const DEFAULTS: Settings = {
  position: "center",
  // 0, igual que en Rust: la isla cuelga del borde y su forma lo da por hecho.
  topMargin: 0,
  alwaysOnTop: true,
  autostart: false,
  notifications: true,
  notifyCharger: true,
  notifyVolume: true,
  notifyLowBattery: true,
  lyrics: true,
  appNotifications: false,
  notificationApps: [],
  skipSeconds: 10,
};

/** Máximo de la separación superior, que es también el del slider. */
const MARGIN_MAX = 120;

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

  // La isla va descubriendo aplicaciones al ver sus notificaciones: la lista
  // llega por evento y se sustituye entera, porque el backend ya la guardó.
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await listen<Settings["notificationApps"]>("toasts://apps", (event) => {
      if (event.payload) settings.value.notificationApps = event.payload;
    });
  } catch {
    // Fuera de la app de escritorio no hay eventos.
  }
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

const enabledApps = computed(() => settings.value.notificationApps.filter((app) => app.enabled).length);

/** Relleno del slider: la parte recorrida lleva el color de acento. */
const marginFill = computed(() => `${(settings.value.topMargin / MARGIN_MAX) * 100}%`);

function restoreDefaults() {
  // Las aplicaciones detectadas se conservan, pero desactivadas: borrar la
  // lista no tendría sentido, la isla las volvería a descubrir enseguida.
  settings.value = {
    ...DEFAULTS,
    notificationApps: settings.value.notificationApps.map((app) => ({ ...app, enabled: false })),
  };
}

function closeWindow() {
  call("settings_close");
}
</script>

<template>
  <div class="settings">
    <header class="head">
      <div>
        <h1>Ajustes</h1>
        <p>Los cambios se aplican al instante.</p>
      </div>
      <div class="head-side">
        <span class="saved" :class="{ visible: saved }" role="status">Guardado</span>
        <button class="close" aria-label="Cerrar ajustes" @click="closeWindow">
          <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" /></svg>
        </button>
      </div>
    </header>

    <section class="section">
      <h2>Posición</h2>
      <div class="panel">
        <div class="row">
          <label class="text" for="pos"><span class="name">Alineación horizontal</span></label>
          <select id="pos" v-model="settings.position">
            <option value="left">Izquierda</option>
            <option value="center">Centro</option>
            <option value="right">Derecha</option>
          </select>
        </div>

        <div class="row">
          <label class="text" for="margin"><span class="name">Separación superior</span></label>
          <div class="slider">
            <input
              id="margin"
              v-model.number="settings.topMargin"
              type="range"
              min="0"
              :max="MARGIN_MAX"
              step="2"
              :style="{ '--fill': marginFill }"
            />
            <span class="value">{{ settings.topMargin }} px</span>
          </div>
        </div>
      </div>
    </section>

    <section class="section">
      <h2>Comportamiento</h2>
      <div class="panel">
        <label class="row">
          <span class="text">
            <span class="name">Siempre visible</span>
            <span class="desc">Mantiene la isla por encima de otras ventanas.</span>
          </span>
          <input v-model="settings.alwaysOnTop" class="switch" type="checkbox" />
        </label>

        <label class="row">
          <span class="text">
            <span class="name">Iniciar con Windows</span>
            <span class="desc">La isla se abre al encender el equipo.</span>
          </span>
          <input v-model="settings.autostart" class="switch" type="checkbox" />
        </label>
      </div>
    </section>

    <section class="section">
      <h2>Reproducción</h2>
      <div class="panel">
        <div class="row">
          <label class="text" for="skip">
            <span class="name">Salto de los botones</span>
            <span class="desc">Apple Music ignora esta función.</span>
          </label>
          <select id="skip" v-model.number="settings.skipSeconds">
            <option :value="5">5 segundos</option>
            <option :value="10">10 segundos</option>
            <option :value="15">15 segundos</option>
            <option :value="30">30 segundos</option>
          </select>
        </div>

        <label class="row">
          <span class="text">
            <span class="name">Letras sincronizadas</span>
            <span class="desc">
              Solo con servicios de streaming. Busca la letra en lrclib.net enviando título y
              artista.
            </span>
          </span>
          <input v-model="settings.lyrics" class="switch" type="checkbox" />
        </label>
      </div>
    </section>

    <section class="section">
      <h2>Avisos del sistema</h2>
      <div class="panel">
        <label class="row">
          <span class="text">
            <span class="name">Avisar de sucesos del sistema</span>
            <span class="desc">La isla se ensancha unos segundos y vuelve a su sitio.</span>
          </span>
          <input v-model="settings.notifications" class="switch" type="checkbox" />
        </label>

        <label class="row sub" :class="{ off: !settings.notifications }">
          <span class="text">
            <span class="name">Cargador</span>
            <span class="desc">Al conectarlo se abre con la batería cargándose.</span>
          </span>
          <input
            v-model="settings.notifyCharger"
            class="switch"
            type="checkbox"
            :disabled="!settings.notifications"
          />
        </label>

        <label class="row sub" :class="{ off: !settings.notifications }">
          <span class="text">
            <span class="name">Volumen</span>
            <span class="desc">Al subirlo, bajarlo o silenciar. Con la isla abierta no aparece.</span>
          </span>
          <input
            v-model="settings.notifyVolume"
            class="switch"
            type="checkbox"
            :disabled="!settings.notifications"
          />
        </label>

        <label class="row sub" :class="{ off: !settings.notifications }">
          <span class="text">
            <span class="name">Batería baja</span>
            <span class="desc">Al llegar al 20 % y al 10 % sin cargador.</span>
          </span>
          <input
            v-model="settings.notifyLowBattery"
            class="switch"
            type="checkbox"
            :disabled="!settings.notifications"
          />
        </label>
      </div>
    </section>

    <section class="section">
      <h2>Notificaciones de aplicaciones</h2>
      <div class="panel">
        <label class="row">
          <span class="text">
            <span class="name">Mostrar notificaciones en la isla</span>
            <span class="desc">
              Lee las notificaciones de Windows para enseñar remitente y mensaje. Nada sale del
              equipo ni se guarda. Requiere tener activado el acceso a notificaciones en Windows.
            </span>
          </span>
          <input v-model="settings.appNotifications" class="switch" type="checkbox" />
        </label>

        <template v-if="settings.appNotifications">
          <p v-if="settings.notificationApps.length === 0" class="row empty">
            Aún no se ha detectado ninguna aplicación. Aparecerán aquí cuando envíen una
            notificación; todas empiezan desactivadas.
          </p>
          <div v-else class="app-list">
            <label v-for="entry in settings.notificationApps" :key="entry.name" class="row sub">
              <span class="text"><span class="name">{{ entry.name }}</span></span>
              <input v-model="entry.enabled" class="switch" type="checkbox" />
            </label>
          </div>
        </template>
      </div>
      <p v-if="settings.appNotifications && settings.notificationApps.length > 0" class="note">
        {{ enabledApps }} de {{ settings.notificationApps.length }} activadas.
      </p>
    </section>

    <footer class="foot">
      <button class="ghost" @click="restoreDefaults">Restaurar valores</button>
    </footer>
  </div>
</template>

<style scoped>
/* Paleta del proyecto: el fondo azul-negro y el verde de la isla. El verde
   solo aparece en lo que esta activo, para que destaque sin competir. */
.settings {
  --bg: #0d0f16;
  --panel: rgba(255, 255, 255, 0.028);
  --line: rgba(255, 255, 255, 0.07);
  --control: #1a1d27;
  --text: #ececf1;
  --muted: #7f8292;
  --accent: #b7f36b;

  min-height: 100vh;
  box-sizing: border-box;
  padding: 18px 18px 16px;
  display: flex;
  flex-direction: column;
  gap: 22px;
  color: var(--text);
  background: var(--bg);
  font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  font-size: 13px;
}

/* --- Cabecera --- */
.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.head h1 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.head p {
  margin: 2px 0 0;
  color: var(--muted);
  font-size: 12px;
}

.head-side {
  display: flex;
  align-items: center;
  gap: 12px;
}

.saved {
  color: var(--accent);
  font-size: 11.5px;
  opacity: 0;
  transition: opacity 0.25s;
}

.saved.visible {
  opacity: 1;
}

.close {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  color: var(--muted);
  border: 0;
  border-radius: 7px;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.close svg {
  width: 11px;
  height: 11px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
  stroke-linecap: round;
}

.close:hover {
  color: var(--text);
  background: rgba(255, 255, 255, 0.08);
}

/* --- Secciones: un titulo discreto y un panel de filas con lineas finas --- */
.section h2 {
  margin: 0 0 8px 2px;
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 600;
  letter-spacing: 0.01em;
}

.panel {
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--panel);
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin: 0;
  padding: 11px 14px;
}

.row + .row,
.row + .app-list,
.app-list .row + .row {
  border-top: 1px solid var(--line);
}

label.row {
  cursor: pointer;
}

.text {
  min-width: 0;
  display: grid;
  gap: 2px;
}

.name {
  font-weight: 500;
}

.desc {
  color: var(--muted);
  font-size: 11.5px;
  line-height: 1.4;
}

/* Opciones que dependen de la de arriba: sangradas y algo mas discretas. */
.row.sub {
  padding-left: 28px;
}

.row.sub .name {
  font-weight: 450;
}

.row.off {
  opacity: 0.42;
  cursor: default;
}

.row.empty {
  display: block;
  color: var(--muted);
  font-size: 11.5px;
  line-height: 1.45;
  border-top: 1px solid var(--line);
}

/* Con muchas aplicaciones detectadas la lista haria crecer la ventana sin
   limite: se queda en un alto fijo y se desplaza por dentro. */
.app-list {
  max-height: 176px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.16) transparent;
}

.note {
  margin: 8px 0 0 2px;
  color: var(--muted);
  font-size: 11.5px;
}

/* --- Controles --- */
.switch {
  position: relative;
  flex: none;
  width: 34px;
  height: 20px;
  margin: 0;
  border-radius: 999px;
  background: #2b2e3a;
  cursor: pointer;
  appearance: none;
  transition: background 0.18s;
}

.switch::after {
  content: "";
  position: absolute;
  top: 3px;
  left: 3px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #c9ccd8;
  transition: transform 0.18s cubic-bezier(0.22, 1, 0.36, 1), background 0.18s;
}

.switch:checked {
  background: var(--accent);
}

.switch:checked::after {
  background: var(--bg);
  transform: translateX(14px);
}

.switch:disabled {
  cursor: default;
}

.switch:focus-visible,
select:focus-visible,
input[type="range"]:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

select {
  padding: 6px 28px 6px 10px;
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--control)
    url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6' fill='none' stroke='%237f8292' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M1 1l4 4 4-4'/%3E%3C/svg%3E")
    no-repeat right 10px center;
  font: inherit;
  cursor: pointer;
  appearance: none;
  transition: border-color 0.15s;
}

select:hover {
  border-color: rgba(255, 255, 255, 0.16);
}

.slider {
  display: flex;
  align-items: center;
  gap: 12px;
}

.value {
  min-width: 42px;
  color: var(--muted);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

input[type="range"] {
  width: 132px;
  height: 18px;
  margin: 0;
  background: transparent;
  cursor: pointer;
  appearance: none;
}

input[type="range"]::-webkit-slider-runnable-track {
  height: 3px;
  border-radius: 2px;
  background: linear-gradient(90deg, var(--accent) var(--fill, 0%), #2b2e3a var(--fill, 0%));
}

input[type="range"]::-webkit-slider-thumb {
  width: 14px;
  height: 14px;
  margin-top: -5.5px;
  border-radius: 50%;
  background: #f4f4f6;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  appearance: none;
}

/* --- Pie --- */
.foot {
  margin-top: auto;
}

.ghost {
  padding: 7px 13px;
  color: #c6c8d3;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.ghost:hover {
  color: var(--text);
  background: rgba(255, 255, 255, 0.06);
}

@media (prefers-reduced-motion: reduce) {
  .switch,
  .switch::after,
  .saved,
  select {
    transition-duration: 0.01ms;
  }
}
</style>

<!-- Global, no `scoped`: esta ventana no importa style.css, que es quien pone
     `margin: 0` al body. Sin esto el navegador deja 8 px de margen sobre
     fondo blanco y se ve un marco blanco alrededor del panel. -->
<style>
html,
body {
  margin: 0;
  background: #0d0f16;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.16) transparent;
}
</style>
