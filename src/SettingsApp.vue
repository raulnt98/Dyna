<script setup lang="ts">
/**
 * Panel de configuración de la isla.
 *
 * Vive en su propia ventana, que se abre desde el icono de la bandeja del
 * sistema, de modo que sigue siendo accesible con la isla en segundo plano.
 */
import { computed, onMounted, ref, watch } from "vue";
import SettingsSelect from "@/components/SettingsSelect.vue";
import SettingsToggle from "@/components/SettingsToggle.vue";

/** Ajustes que se guardan entre sesiones. */
interface Settings {
  /** Posición horizontal de la isla en la pantalla. */
  position: "left" | "center" | "right";
  /** Separación respecto al borde superior, en píxeles. */
  topMargin: number;
  /** Mantener la isla por encima del resto de ventanas. */
  alwaysOnTop: boolean;
  /** Aspecto de la isla cerrada: píldora o pestaña mínima. */
  collapsedStyle: "pill" | "tab";
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
  notifyWifi: boolean;
  notifyInternet: boolean;
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
  collapsedStyle: "pill",
  autostart: false,
  notifications: true,
  notifyCharger: true,
  notifyVolume: true,
  notifyLowBattery: true,
  notifyWifi: true,
  notifyInternet: true,
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

/* Clases que se repiten en las secciones. Cadenas completas y literales: así
   Tailwind las detecta al escanear el archivo. */
const H2 = "mt-0 mb-2 ml-0.5 text-[11.5px] font-semibold tracking-[0.01em] text-muted";
const PANEL = "overflow-hidden rounded-[10px] border border-line bg-panel [&>*+*]:border-t [&>*+*]:border-line";
const ROW = "m-0 flex items-center justify-between gap-4 px-3.5 py-[11px]";
const DESC = "text-[11.5px] leading-[1.4] text-muted";
</script>

<template>
  <div class="box-border flex min-h-screen flex-col gap-[22px] bg-bg px-[18px] pt-[18px] pb-4 font-sans text-[13px] leading-[normal] text-ink">
    <header class="flex items-start justify-between gap-3">
      <div>
        <h1 class="m-0 text-[16px] font-semibold tracking-[-0.01em]">Ajustes</h1>
        <p class="mt-0.5 mb-0 text-[12px] text-muted">Los cambios se aplican al instante.</p>
      </div>
      <div class="flex items-center gap-3">
        <span
          class="text-[11.5px] text-accent transition-opacity duration-250 motion-reduce:duration-[0.01ms]"
          :class="saved ? 'opacity-100' : 'opacity-0'"
          role="status"
        >Guardado</span>
        <button
          class="grid size-7 cursor-pointer place-items-center rounded-[7px] border-0 bg-transparent text-muted transition-colors duration-150 hover:bg-white/8 hover:text-ink"
          aria-label="Cerrar ajustes"
          @click="closeWindow"
        >
          <svg class="size-[11px] fill-none stroke-current stroke-[1.4] [stroke-linecap:round]" viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" /></svg>
        </button>
      </div>
    </header>

    <section>
      <h2 :class="H2">Posición</h2>
      <div :class="PANEL">
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="pos"><span class="font-medium">Alineación horizontal</span></label>
          <SettingsSelect
            id="pos"
            v-model="settings.position"
            :options="[
              { value: 'left', label: 'Izquierda' },
              { value: 'center', label: 'Centro' },
              { value: 'right', label: 'Derecha' },
            ]"
          />
        </div>

        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="margin"><span class="font-medium">Separación superior</span></label>
          <div class="flex items-center gap-3">
            <input
              id="margin"
              v-model.number="settings.topMargin"
              type="range"
              min="0"
              :max="MARGIN_MAX"
              step="2"
              :style="{ '--fill': marginFill }"
              class="m-0 h-[18px] w-[132px] cursor-pointer appearance-none bg-transparent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-sm [&::-webkit-slider-runnable-track]:bg-[linear-gradient(90deg,var(--color-accent)_var(--fill,0%),var(--color-control)_var(--fill,0%))] [&::-webkit-slider-thumb]:-mt-[5.5px] [&::-webkit-slider-thumb]:size-3.5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#f4f4f6] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.5)]"
            />
            <span class="min-w-[42px] text-right text-[12px] text-muted tabular-nums">{{ settings.topMargin }} px</span>
          </div>
        </div>
      </div>
    </section>

    <section>
      <h2 :class="H2">Comportamiento</h2>
      <div :class="PANEL">
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="collapsed">
            <span class="font-medium">Vista minimizada</span>
            <span :class="DESC">Cómo se ve la isla cerrada. La pestaña casi no tapa lo que hay debajo.</span>
          </label>
          <SettingsSelect
            id="collapsed"
            v-model="settings.collapsedStyle"
            :options="[
              { value: 'pill', label: 'Píldora' },
              { value: 'tab', label: 'Pestaña mínima' },
            ]"
          />
        </div>

        <SettingsToggle
          v-model="settings.alwaysOnTop"
          name="Siempre visible"
          desc="Mantiene la isla por encima de otras ventanas."
        />
        <SettingsToggle
          v-model="settings.autostart"
          name="Iniciar con Windows"
          desc="La isla se abre al encender el equipo."
        />
      </div>
    </section>

    <section>
      <h2 :class="H2">Reproducción</h2>
      <div :class="PANEL">
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="skip">
            <span class="font-medium">Salto de los botones</span>
            <span :class="DESC">Apple Music ignora esta función.</span>
          </label>
          <SettingsSelect
            id="skip"
            v-model="settings.skipSeconds"
            :options="[
              { value: 5, label: '5 segundos' },
              { value: 10, label: '10 segundos' },
              { value: 15, label: '15 segundos' },
              { value: 30, label: '30 segundos' },
            ]"
          />
        </div>

        <SettingsToggle
          v-model="settings.lyrics"
          name="Letras sincronizadas"
          desc="Solo con servicios de streaming. Busca la letra en lrclib.net enviando título y artista."
        />
      </div>
    </section>

    <section>
      <h2 :class="H2">Avisos del sistema</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.notifications"
          name="Avisar de sucesos del sistema"
          desc="La isla se ensancha unos segundos y vuelve a su sitio."
        />
        <SettingsToggle
          v-model="settings.notifyCharger"
          sub
          :disabled="!settings.notifications"
          name="Cargador"
          desc="Al conectarlo se abre con la batería cargándose."
        />
        <SettingsToggle
          v-model="settings.notifyVolume"
          sub
          :disabled="!settings.notifications"
          name="Volumen"
          desc="Al subirlo, bajarlo o silenciar. Con la isla abierta no aparece."
        />
        <SettingsToggle
          v-model="settings.notifyLowBattery"
          sub
          :disabled="!settings.notifications"
          name="Batería baja"
          desc="Al llegar al 20 % y al 10 % sin cargador."
        />
        <SettingsToggle
          v-model="settings.notifyWifi"
          sub
          :disabled="!settings.notifications"
          name="Wi-Fi"
          desc="Al conectarse a una red, con el nombre de la red."
        />
        <SettingsToggle
          v-model="settings.notifyInternet"
          sub
          :disabled="!settings.notifications"
          name="Internet"
          desc="Al perder la conexión a internet y al recuperarla."
        />
      </div>
    </section>

    <section>
      <h2 :class="H2">Notificaciones de aplicaciones</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.appNotifications"
          name="Mostrar notificaciones en la isla"
          desc="Lee las notificaciones de Windows para enseñar remitente y mensaje. Nada se guarda. Con WhatsApp activado, se descarga además la foto de perfil del remitente desde los servidores de WhatsApp. Requiere tener activado el acceso a notificaciones en Windows."
        />

        <template v-if="settings.appNotifications">
          <p
            v-if="settings.notificationApps.length === 0"
            class="m-0 px-3.5 py-[11px] text-[11.5px] leading-[1.45] text-muted"
          >
            Aún no se ha detectado ninguna aplicación. Aparecerán aquí cuando envíen una
            notificación; todas empiezan desactivadas.
          </p>
          <!-- Con muchas aplicaciones detectadas la lista haría crecer la
               ventana sin límite: se queda en un alto fijo y se desplaza por
               dentro. -->
          <div
            v-else
            class="max-h-44 overflow-y-auto [scrollbar-color:rgba(255,255,255,0.16)_transparent] [scrollbar-width:thin] [&>*+*]:border-t [&>*+*]:border-line"
          >
            <SettingsToggle
              v-for="entry in settings.notificationApps"
              :key="entry.name"
              v-model="entry.enabled"
              sub
              :name="entry.name"
            />
          </div>
        </template>
      </div>
      <p
        v-if="settings.appNotifications && settings.notificationApps.length > 0"
        class="mt-2 mb-0 ml-0.5 text-[11.5px] text-muted"
      >
        {{ enabledApps }} de {{ settings.notificationApps.length }} activadas.
      </p>
    </section>

    <footer class="mt-auto">
      <button
        class="cursor-pointer rounded-lg border border-line bg-transparent px-[13px] py-[7px] text-[12px] text-[#c6c8d3] transition-colors duration-150 hover:bg-white/6 hover:text-ink"
        @click="restoreDefaults"
      >Restaurar valores</button>
    </footer>
  </div>
</template>
