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
  /** Estilo de la isla, cerrada y abierta: con muescas o flotante redondeada. */
  islandStyle: "notch" | "round";
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
  notifyBluetooth: boolean;
  notifyPrivacy: boolean;
  notifyDownloads: boolean;
  notifyTrack: boolean;
  /** Abrir la isla al pasar el ratón. */
  hoverOpen: boolean;
  notifyUsb: boolean;
  notifyLockKeys: boolean;
  notifyFocus: boolean;
  notifyBrightness: boolean;
  /** Buscar letras sincronizadas en LRCLIB. */
  lyrics: boolean;
  /** Mostrar las notificaciones de otras aplicaciones (Teams, Outlook…). */
  appNotifications: boolean;
  /** Aplicaciones detectadas, cada una con su interruptor. */
  notificationApps: { name: string; enabled: boolean }[];
  /** Segundos que saltan los botones de avance y retroceso. */
  skipSeconds: number;
  /** Segundos en la hora de la vista de inicio. */
  homeSeconds: boolean;
  /** CPU y memoria en la vista de inicio. */
  homeStats: boolean;
  /** La onda sigue el audio real del sistema. */
  audioVisualizer: boolean;
  /** Tiempo en la vista de inicio, y la ciudad que se consulta. */
  weather: boolean;
  weatherCity: string;
  /** Botón de test de velocidad en la vista de inicio. */
  speedtest: boolean;
  /** Pantalla de la isla: 0 es la principal. */
  monitor: number;
}

const DEFAULTS: Settings = {
  position: "center",
  // 0, igual que en Rust: la isla cuelga del borde y su forma lo da por hecho.
  topMargin: 0,
  alwaysOnTop: true,
  collapsedStyle: "pill",
  islandStyle: "notch",
  autostart: false,
  notifications: true,
  notifyCharger: true,
  notifyVolume: true,
  notifyLowBattery: true,
  notifyWifi: true,
  notifyInternet: true,
  notifyBluetooth: true,
  notifyPrivacy: true,
  notifyDownloads: true,
  notifyTrack: true,
  hoverOpen: true,
  notifyUsb: true,
  notifyLockKeys: true,
  notifyFocus: true,
  notifyBrightness: true,
  lyrics: true,
  appNotifications: false,
  notificationApps: [],
  skipSeconds: 10,
  homeSeconds: true,
  homeStats: true,
  audioVisualizer: false,
  weather: false,
  weatherCity: "",
  speedtest: true,
  monitor: 0,
};

const settings = ref<Settings>({ ...DEFAULTS });

/**
 * Pestañas del panel. Con tantos ajustes, una sola lista larga no se podía recorrer: se agrupan por
 * lo que tocan. La última abierta se recuerda (solo comodidad: si el almacenamiento falla, se abre
 * en la primera).
 */
const TABS = [
  { id: "island", label: "Isla" },
  { id: "home", label: "Inicio" },
  { id: "music", label: "Música" },
  { id: "alerts", label: "Avisos" },
  { id: "apps", label: "Apps" },
] as const;
type TabId = (typeof TABS)[number]["id"];
const TAB_KEY = "settings-tab";
function storedTab(): TabId {
  try {
    const value = localStorage.getItem(TAB_KEY);
    return TABS.some((item) => item.id === value) ? (value as TabId) : "island";
  } catch {
    return "island";
  }
}
const tab = ref<TabId>(storedTab());
function selectTab(id: TabId) {
  tab.value = id;
  try {
    localStorage.setItem(TAB_KEY, id);
  } catch {
    // Sin almacenamiento no se recuerda la pestaña; no pasa nada.
  }
  // Cada pestaña empieza arriba: si no, se hereda el scroll de la anterior.
  window.scrollTo({ top: 0 });
}
/** Flechas izquierda y derecha entre pestañas, como en cualquier lista de pestañas. */
function onTabKey(event: KeyboardEvent) {
  const index = TABS.findIndex((item) => item.id === tab.value);
  let next = index;
  if (event.key === "ArrowRight") next = (index + 1) % TABS.length;
  else if (event.key === "ArrowLeft") next = (index - 1 + TABS.length) % TABS.length;
  else return;
  event.preventDefault();
  selectTab(TABS[next].id);
  document.getElementById("tab-" + TABS[next].id)?.focus();
}

/**
 * Permiso de Windows para leer notificaciones (`allowed`, `denied`, `unspecified` o
 * `unavailable`). Sin `allowed` la isla no ve ninguna notificación y no lo dice por
 * su cuenta, así que el panel lo enseña. Fuera de la app de escritorio no hay nada
 * que comprobar y se da por bueno.
 */
const access = ref("allowed");
async function refreshAccess() {
  const status = await call<string>("toasts_access");
  if (status) access.value = status;
}
/** Pantallas que informa Windows, para el selector. Sin la app de escritorio solo hay la principal. */
const monitors = ref<{ value: number; label: string }[]>([{ value: 0, label: "Pantalla principal" }]);
async function loadMonitors() {
  const list = await call<{ index: number; label: string }[]>("monitors_list");
  if (list?.length) monitors.value = list.map((m) => ({ value: m.index, label: m.label }));
}
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
  void refreshAccess();
  void loadMonitors();
  // El usuario cambia el permiso en la configuración de Windows y vuelve aquí:
  // se vuelve a comprobar al recuperar el foco, sin que tenga que reabrir el panel.
  window.addEventListener("focus", refreshAccess);
  // Se enchufa o se quita una pantalla con el panel abierto: se vuelve a leer la lista.
  window.addEventListener("focus", loadMonitors);

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
    <!-- Cabecera y pestañas fijas arriba: con más de treinta ajustes, la lista entera en un solo scroll era
         imposible de recorrer. Los márgenes negativos la llevan de borde a borde de la ventana. -->
    <div class="sticky top-0 z-10 -mx-[18px] -mt-[18px] flex flex-col gap-3 bg-bg px-[18px] pt-[18px]">
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
      <nav class="-mx-1 flex gap-0.5 border-b border-line" role="tablist" aria-label="Secciones de ajustes" @keydown="onTabKey">
        <button
          v-for="item in TABS"
          :id="'tab-' + item.id"
          :key="item.id"
          role="tab"
          :aria-selected="tab === item.id"
          :aria-controls="'panel-' + item.id"
          :tabindex="tab === item.id ? 0 : -1"
          class="relative cursor-pointer border-0 bg-transparent px-3 py-2 text-[12.5px] font-medium transition-colors duration-150 after:absolute after:inset-x-2.5 after:-bottom-px after:h-0.5 after:rounded-full after:transition-colors focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent"
          :class="tab === item.id ? 'text-ink after:bg-accent' : 'text-muted after:bg-transparent hover:text-ink'"
          @click="selectTab(item.id)"
        >
          {{ item.label }}<span v-if="item.id === 'apps' && enabledApps" class="ml-1.5 rounded-full bg-white/10 px-1.5 py-px text-[10px] text-muted">{{ enabledApps }}</span>
        </button>
      </nav>
    </div>

    <div v-show="tab === 'island'" :id="'panel-island'" role="tabpanel" :aria-labelledby="'tab-island'" class="grid gap-[22px]">
    <section>
      <h2 :class="H2">Pantalla</h2>
      <div :class="PANEL">
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="monitor">
            <span class="font-medium">Pantalla</span>
            <span :class="DESC">En qué pantalla cuelga la isla. Si la elegida se desconecta, vuelve a la principal.</span>
          </label>
          <SettingsSelect id="monitor" v-model="settings.monitor" :options="monitors" />
        </div>
      </div>
    </section>

    <section>
      <h2 :class="H2">Aspecto</h2>
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

        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="island-style">
            <span class="font-medium">Estilo de la isla</span>
            <span :class="DESC">Vale para la isla cerrada y para la abierta, con el mismo tamaño. Con muescas, cuelga del borde de la pantalla y se funde con él. Flotante redondeada es una cápsula suelta, unos píxeles por debajo del borde y sin muescas.</span>
          </label>
          <SettingsSelect
            id="island-style"
            v-model="settings.islandStyle"
            :options="[
              { value: 'notch', label: 'Con muescas' },
              { value: 'round', label: 'Flotante redondeada' },
            ]"
          />
        </div>
      </div>
    </section>

    <section>
      <h2 :class="H2">Comportamiento</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.hoverOpen"
          name="Abrir al pasar el ratón"
          desc="La isla se despliega al acercar el ratón y se recoge al alejarlo. El clic sigue funcionando: si haces clic con ella abierta así, se queda abierta."
        />

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
    </div>

    <div v-show="tab === 'home'" :id="'panel-home'" role="tabpanel" :aria-labelledby="'tab-home'" class="grid gap-[22px]">
    <section>
      <h2 :class="H2">Vista de inicio</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.homeSeconds"
          name="Segundos en la hora"
          desc="Muestra los segundos en la hora que enseña la isla cuando no hay nada sonando."
        />

        <SettingsToggle
          v-model="settings.homeStats"
          name="CPU y memoria"
          desc="Muestra el uso del procesador y de la memoria en la vista de inicio."
        />

        <SettingsToggle
          v-model="settings.weather"
          name="Tiempo en la vista de inicio"
          desc="Temperatura y estado del cielo junto a la fecha. Envía la ciudad que escribas aquí a Open-Meteo para consultarlo."
        />

        <div :class="ROW" class="pl-7" :style="{ opacity: settings.weather ? 1 : 0.42 }">
          <label class="grid min-w-0 gap-0.5" for="weather-city"><span class="font-[450]">Ciudad</span></label>
          <input
            id="weather-city"
            v-model.trim="settings.weatherCity"
            :disabled="!settings.weather"
            type="text"
            maxlength="60"
            placeholder="Santiago"
            autocomplete="off"
            class="w-[170px] rounded-[7px] border border-white/7 bg-[#1a1d27] px-3 py-1.5 text-ink outline-none transition-[border-color] duration-150 placeholder:text-[#5c5f6e] hover:border-white/16 focus-visible:border-accent"
          />
        </div>

        <SettingsToggle
          v-model="settings.speedtest"
          name="Test de velocidad"
          desc="Una página de la vista de inicio que mide la velocidad de bajada y la latencia con Cloudflare. Solo mide al pulsar el botón, y descarga unas decenas de megas."
        />
      </div>
      <p class="mt-2 mb-0 ml-0.5 text-[11.5px] leading-[1.45] text-muted">La hora va en el centro. Con la isla abierta, los indicadores de los lados rotan entre las páginas que actives aquí.</p>
    </section>
    </div>

    <div v-show="tab === 'music'" :id="'panel-music'" role="tabpanel" :aria-labelledby="'tab-music'" class="grid gap-[22px]">
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

        <SettingsToggle
          v-model="settings.audioVisualizer"
          name="Visualizador de audio real"
          desc="La onda de la isla sigue el sonido que sale por tus altavoces. Escucha la salida del sistema solo para medir su nivel: no guarda nada ni lo envía."
        />
      </div>
    </section>
    </div>

    <div v-show="tab === 'alerts'" :id="'panel-alerts'" role="tabpanel" :aria-labelledby="'tab-alerts'" class="grid gap-[22px]">
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
        <SettingsToggle
          v-model="settings.notifyBluetooth"
          sub
          :disabled="!settings.notifications"
          name="Bluetooth"
          desc="Al conectar o desconectar un dispositivo emparejado."
        />
        <SettingsToggle
          v-model="settings.notifyTrack"
          sub
          :disabled="!settings.notifications"
          name="Canción"
          desc="Presenta la canción al empezar a sonar o al cambiar de pista, desde cualquier app."
        />
        <SettingsToggle
          v-model="settings.notifyPrivacy"
          sub
          :disabled="!settings.notifications"
          name="Micrófono y cámara"
          desc="Un punto naranja o verde en la isla mientras una aplicación los usa, y un aviso al empezar. Solo lee el nombre de la aplicación."
        />
        <SettingsToggle
          v-model="settings.notifyDownloads"
          sub
          :disabled="!settings.notifications"
          name="Descargas"
          desc="Velocidad de las descargas del navegador y aviso al terminar. Sin porcentaje: el navegador no dice cuánto falta."
        />
        <SettingsToggle
          v-model="settings.notifyUsb"
          sub
          :disabled="!settings.notifications"
          name="USB"
          desc="Al conectar o retirar una memoria USB o una tarjeta SD."
        />
        <SettingsToggle
          v-model="settings.notifyLockKeys"
          sub
          :disabled="!settings.notifications"
          name="Bloq Mayús y Bloq Num"
          desc="Al activar o apagar cualquiera de las dos."
        />
        <SettingsToggle
          v-model="settings.notifyFocus"
          sub
          :disabled="!settings.notifications"
          name="No molestar"
          desc="Al activar o apagar el modo de concentración de Windows."
        />
        <SettingsToggle
          v-model="settings.notifyBrightness"
          sub
          :disabled="!settings.notifications"
          name="Brillo"
          desc="Al cambiar el brillo de la pantalla del portátil."
        />
      </div>
    </section>
    </div>

    <div v-show="tab === 'apps'" :id="'panel-apps'" role="tabpanel" :aria-labelledby="'tab-apps'" class="grid gap-[22px]">
    <section>
      <h2 :class="H2">Notificaciones de aplicaciones</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.appNotifications"
          name="Mostrar notificaciones en la isla"
          desc="Lee las notificaciones de Windows para enseñar remitente y mensaje. Nada se guarda. Con WhatsApp activado, se descarga además la foto de perfil del remitente desde los servidores de WhatsApp. Requiere tener activado el acceso a notificaciones en Windows."
        />

        <!-- Sin permiso de Windows la opción parece activa pero no llega nada. -->
        <div
          v-if="settings.appNotifications && access !== 'allowed'"
          class="flex items-center justify-between gap-3 bg-[rgba(255,138,112,.08)] px-3.5 py-[11px] text-[11.5px] leading-[1.45] text-[#ffb08a]"
          role="alert"
        >
          <span>
            <template v-if="access === 'unavailable'">
              Este equipo no permite leer las notificaciones de otras aplicaciones.
            </template>
            <template v-else>
              Windows no deja que la isla lea tus notificaciones. Actívalo en
              Configuración → Privacidad y seguridad → Notificaciones.
            </template>
          </span>
          <button
            v-if="access !== 'unavailable'"
            class="flex-none cursor-pointer rounded-lg border border-[rgba(255,138,112,.35)] bg-transparent px-[11px] py-[6px] text-[11.5px] text-[#ffb08a] transition-colors duration-150 hover:bg-[rgba(255,138,112,.14)]"
            @click="call('toasts_open_privacy')"
          >Abrir Windows</button>
        </div>

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
    </div>

    <footer class="mt-auto">
      <button
        class="cursor-pointer rounded-lg border border-line bg-transparent px-[13px] py-[7px] text-[12px] text-[#c6c8d3] transition-colors duration-150 hover:bg-white/6 hover:text-ink"
        @click="restoreDefaults"
      >Restaurar valores</button>
    </footer>
  </div>
</template>
