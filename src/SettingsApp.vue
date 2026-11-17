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
  /** Material de cristal líquido (solo con el estilo flotante). */
  glass: boolean;
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
  notificationStyle: "panel" | "bubble";
  /** Aplicaciones detectadas, cada una con su interruptor. */
  notificationApps: { name: string; enabled: boolean; style?: "panel" | "bubble" | null }[];
  /** Segundos que saltan los botones de avance y retroceso. */
  skipSeconds: number;
  /** Segundos en la hora de la vista de inicio. */
  homeSeconds: boolean;
  /** CPU y memoria en la vista de inicio. */
  homeStats: boolean;
  /** Página de baterías de dispositivos. */
  homeBatteries: boolean;
  /** La onda sigue el audio real del sistema. */
  audioVisualizer: boolean;
  /** Tiempo en la vista de inicio, y la ciudad que se consulta. */
  weather: boolean;
  weatherCity: string;
  /** Coordenadas de la ciudad elegida o de la ubicación actual; sin ellas se busca por nombre. */
  weatherLat: number | null;
  weatherLon: number | null;
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
  glass: false,
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
  notificationStyle: "panel",
  notificationApps: [],
  skipSeconds: 10,
  homeSeconds: true,
  homeStats: true,
  homeBatteries: true,
  audioVisualizer: false,
  weather: false,
  weatherCity: "",
  weatherLat: null,
  weatherLon: null,
  speedtest: true,
  monitor: 0,
};

const settings = ref<Settings>({ ...DEFAULTS });

/**
 * Pestañas del panel. Con tantos ajustes, una sola lista larga no se podía recorrer: se agrupan por
 * lo que tocan. La última abierta se recuerda (solo comodidad: si el almacenamiento falla, se abre
 * en la primera).
 */
/** Formato de las notificaciones de cada app. `default` (en pantalla, «General») no es un valor guardado: es `null`. */
const APP_STYLES = [
  { value: "default", label: "General" },
  { value: "panel", label: "Completa" },
  { value: "bubble", label: "Discreta" },
];

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
/** Posición de la pestaña activa, para el indicador deslizante. */
const tabIndex = computed(() => TABS.findIndex((item) => item.id === tab.value));
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

/** Formato de la hora de la vista de inicio: `homeSeconds` sigue siendo un booleano en el archivo de ajustes. */
const timeFormat = computed<string>({
  get: () => (settings.value.homeSeconds ? "hms" : "hm"),
  set: (value) => {
    settings.value.homeSeconds = value === "hms";
  },
});

/** El material se guarda como booleano (`glass`) y se elige como texto. */
const material = computed<string>({
  get: () => (settings.value.glass ? "glass" : "solid"),
  set: (value) => {
    settings.value.glass = value === "glass";
  },
});

interface Place {
  name: string;
  detail: string;
  lat: number;
  lon: number;
}
const places = ref<Place[]>([]);
const locating = ref(false);
const locateError = ref("");
let searchTimer: number | undefined;

/** Escribir en el campo de ciudad: descarta las coordenadas anteriores y busca coincidencias al dejar de teclear. */
function onCityInput() {
  settings.value.weatherLat = null;
  settings.value.weatherLon = null;
  locateError.value = "";
  window.clearTimeout(searchTimer);
  const query = settings.value.weatherCity.trim();
  if (query.length < 2) {
    places.value = [];
    return;
  }
  searchTimer = window.setTimeout(async () => {
    const found = await call<Place[]>("weather_search", { query });
    if (settings.value.weatherCity.trim() === query) places.value = found ?? [];
  }, 350);
}

function pickPlace(place: Place) {
  settings.value.weatherCity = place.detail ? `${place.name}, ${place.detail}` : place.name;
  settings.value.weatherLat = place.lat;
  settings.value.weatherLon = place.lon;
  places.value = [];
}

async function useMyLocation() {
  locating.value = true;
  locateError.value = "";
  places.value = [];
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    pickPlace(await invoke<Place>("weather_locate_here"));
  } catch (error) {
    locateError.value =
      error === "denied"
        ? "Windows bloquea la ubicación. Actívala en Configuración → Privacidad → Ubicación."
        : "No se pudo obtener la ubicación.";
  } finally {
    locating.value = false;
  }
}

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

/** La ventana no lleva barra de título nativa: se arrastra desde la cabecera, salvo desde los botones. */
function startDrag(event: MouseEvent) {
  if ((event.target as HTMLElement).closest("button")) return;
  call("settings_drag");
}

/* Clases que se repiten en las secciones. Cadenas completas y literales: así
   Tailwind las detecta al escanear el archivo. */
const H2 = "mt-0 mb-1.5 ml-3.5 text-[11.5px] font-medium tracking-[0.05em] text-muted uppercase";
const PANEL = "overflow-hidden rounded-[11px] border border-line bg-panel shadow-[0_1px_2px_rgba(30,32,44,0.04)] [&>*]:relative [&>*+*]:before:absolute [&>*+*]:before:top-0 [&>*+*]:before:right-0 [&>*+*]:before:left-3.5 [&>*+*]:before:h-px [&>*+*]:before:bg-line [&>*+*]:before:content-['']";
const ROW = "m-0 flex items-center justify-between gap-4 px-3.5 py-3";
const DESC = "text-[12.5px] leading-[1.35] text-muted";
</script>

<template>
  <div class="box-border flex min-h-screen flex-col gap-[22px] px-[18px] pt-[18px] pb-4 font-sans select-none text-[14px] leading-[normal] text-ink">
    <!-- Cabecera y pestañas fijas arriba: con más de treinta ajustes, la lista entera en un solo scroll era
         imposible de recorrer. Los márgenes negativos la llevan de borde a borde de la ventana. -->
    <div class="sticky top-0 z-10 -mx-[18px] -mt-[18px] flex flex-col gap-3.5 bg-bg px-[18px] pt-[16px] pb-3">
     <header class="flex cursor-default items-center justify-between gap-3 select-none" @mousedown.left="startDrag">
        <div class="flex items-center gap-2.5">
          <img src="/logo-96.png" alt="" class="size-7 flex-none" draggable="false" />
          <h1 class="m-0 text-[16px] font-semibold tracking-[-0.005em]">Dyna</h1>
        </div>
        <div class="flex items-center gap-3">
          <span
            class="text-[11.5px] text-accent transition-opacity duration-250 motion-reduce:duration-[0.01ms]"
            :class="saved ? 'opacity-100' : 'opacity-0'"
            role="status"
          >Guardado</span>
          <button
            class="grid size-7 cursor-pointer place-items-center rounded-[7px] border-0 bg-transparent text-muted transition-colors duration-150 hover:bg-hover hover:text-ink"
            aria-label="Cerrar ajustes"
            @click="closeWindow"
          >
            <svg class="size-[11px] fill-none stroke-current stroke-[1.4] [stroke-linecap:round]" viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" /></svg>
          </button>
        </div>
      </header>
      <nav class="relative flex rounded-[9px] bg-line p-0.5" role="tablist" aria-label="Secciones de ajustes" @keydown="onTabKey">
        <!-- El indicador mide lo que una pestaña (sin huecos entre ellas: con ellos el paso no coincidiría) y se desliza con la curva
             de la isla. Va por debajo de los botones, que son transparentes. -->
        <span
          class="pointer-events-none absolute top-0.5 bottom-0.5 left-0.5 rounded-[7px] bg-white shadow-[0_1px_2px_rgba(0,0,0,0.12),0_0_0_0.5px_rgba(0,0,0,0.05)] transition-transform duration-[460ms] ease-[cubic-bezier(0.32,0.72,0,1)] motion-reduce:duration-[0.01ms]"
          :style="{ width: `calc((100% - 4px) / ${TABS.length})`, transform: `translateX(${tabIndex * 100}%)` }"
          aria-hidden="true"
        ></span>
        <button
          v-for="item in TABS"
          :id="'tab-' + item.id"
          :key="item.id"
          role="tab"
          :aria-selected="tab === item.id"
          :aria-controls="'panel-' + item.id"
          :tabindex="tab === item.id ? 0 : -1"
          class="relative flex-1 cursor-pointer rounded-[7px] border-0 bg-transparent px-2 py-[5px] text-[13px] font-medium transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent"
          :class="tab === item.id ? 'text-ink' : 'text-muted hover:text-ink'"
          @click="selectTab(item.id)"
        >
          {{ item.label }}<span v-if="item.id === 'apps' && enabledApps" class="ml-1.5 rounded-full bg-line px-1.5 py-px text-[10px] text-muted">{{ enabledApps }}</span>
        </button>
      </nav>
    </div>

    <div v-show="tab === 'island'" :id="'panel-island'" role="tabpanel" :aria-labelledby="'tab-island'" class="tab-panel grid gap-[22px]">
    <section>
      <h2 :class="H2">Pantalla</h2>
      <div :class="PANEL">
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="monitor">
            <span class="font-medium">Pantalla</span>
            <span :class="DESC">Si se desconecta, vuelve a la principal.</span>
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
            <span :class="DESC">La pestaña casi no tapa nada.</span>
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
            <span :class="DESC">Con muescas, pegada al borde. Flotante, una cápsula suelta.</span>
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

        <div :class="ROW" :style="{ opacity: settings.islandStyle === 'round' ? 1 : 0.42 }">
          <label class="grid min-w-0 gap-0.5" for="island-material">
            <span class="font-medium">Material</span>
            <span :class="DESC">Desenfoca lo de detrás. Solo con el estilo flotante; la isla no sale en capturas.</span>
          </label>
          <SettingsSelect
            id="island-material"
            v-model="material"
            :options="[
              { value: 'solid', label: 'Sólido' },
              { value: 'glass', label: 'Cristal líquido (beta)' },
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
          desc="Se abre al acercar el ratón y se recoge al alejarlo."
        />

        <SettingsToggle
          v-model="settings.alwaysOnTop"
          name="Siempre visible"
          desc="Se mantiene sobre otras ventanas."
        />

        <SettingsToggle
          v-model="settings.autostart"
          name="Iniciar con Windows"
          desc="Arranca con Windows."
        />
      </div>
    </section>
    </div>

    <div v-show="tab === 'home'" :id="'panel-home'" role="tabpanel" :aria-labelledby="'tab-home'" class="tab-panel grid gap-[22px]">
    <section>
      <h2 :class="H2">Vista de inicio</h2>
      <div :class="PANEL">
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="time-format">
            <span class="font-medium">Formato de la hora</span>
            <span :class="DESC">Formato de la hora en la vista de inicio.</span>
          </label>
          <SettingsSelect
            id="time-format"
            v-model="timeFormat"
            :options="[
              { value: 'hms', label: 'hh:mm:ss' },
              { value: 'hm', label: 'hh:mm' },
            ]"
          />
        </div>

        <SettingsToggle
          v-model="settings.homeStats"
          name="CPU y memoria"
          desc="Procesador y memoria en la vista de inicio."
        />

        <SettingsToggle
          v-model="settings.homeBatteries"
          name="Baterías de dispositivos"
          desc="Carga del portátil y de los dispositivos Bluetooth que la informen."
        />

        <SettingsToggle
          v-model="settings.weather"
          name="Tiempo en la vista de inicio"
          desc="Temperatura y cielo junto a la fecha. Envía la ciudad a Open-Meteo."
        />

        <div class="grid gap-2 px-3.5 py-[11px] pl-7" :style="{ opacity: settings.weather ? 1 : 0.42 }">
          <div class="flex items-center justify-between gap-4">
            <label class="grid min-w-0 gap-0.5" for="weather-city"><span class="font-[450]">Ciudad</span></label>
            <input
              id="weather-city"
              v-model.trim="settings.weatherCity"
              :disabled="!settings.weather"
              type="text"
              maxlength="60"
              placeholder="Busca una ciudad"
              autocomplete="off"
              class="w-[200px] rounded-[7px] border border-line bg-field select-text px-3 py-1.5 text-ink outline-none transition-[border-color] duration-150 placeholder:text-muted hover:border-edge focus-visible:border-accent"
              @input="onCityInput"
            />
          </div>
          <ul v-if="places.length" class="m-0 grid list-none gap-px overflow-hidden rounded-[8px] border border-line bg-line p-0">
            <li v-for="place in places" :key="`${place.lat},${place.lon}`" class="bg-panel">
              <button
                type="button"
                class="grid w-full cursor-pointer gap-0.5 border-0 bg-transparent px-3 py-2 text-left text-ink transition-colors hover:bg-hover"
                @click="pickPlace(place)"
              >
                <span class="font-[450]">{{ place.name }}</span>
                <span :class="DESC">{{ place.detail }}</span>
              </button>
            </li>
          </ul>
          <div class="flex items-center justify-between gap-4">
            <span :class="DESC">{{ settings.weatherLat !== null ? "Ciudad fijada." : "Elige una ciudad o usa tu ubicación." }}</span>
            <button
              type="button"
              :disabled="!settings.weather || locating"
              class="shrink-0 cursor-pointer rounded-[7px] border border-line bg-field px-3 py-1.5 text-ink transition-[border-color] duration-150 hover:border-edge disabled:cursor-default disabled:opacity-60"
              @click="useMyLocation"
            >{{ locating ? "Buscando…" : "Usar mi ubicación" }}</button>
          </div>
          <p v-if="locateError" class="m-0 text-[11.5px] leading-[1.4] text-[#b4540f]">{{ locateError }}</p>
        </div>

        <SettingsToggle
          v-model="settings.speedtest"
          name="Test de velocidad"
          desc="Velocidad y latencia con Cloudflare. Solo mide al pulsar; descarga unos megas."
        />
      </div>
      <p class="mt-2 mb-0 ml-0.5 text-[11.5px] leading-[1.45] text-muted">La hora va en el centro; los lados rotan entre las páginas activas.</p>
    </section>
    </div>

    <div v-show="tab === 'music'" :id="'panel-music'" role="tabpanel" :aria-labelledby="'tab-music'" class="tab-panel grid gap-[22px]">
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
          desc="Busca la letra en lrclib.net (envía título y artista)."
        />

        <SettingsToggle
          v-model="settings.audioVisualizer"
          name="Visualizador de audio real"
          desc="La onda sigue el sonido del sistema. Solo mide su nivel: no guarda ni envía nada."
        />
      </div>
    </section>
    </div>

    <div v-show="tab === 'alerts'" :id="'panel-alerts'" role="tabpanel" :aria-labelledby="'tab-alerts'" class="tab-panel grid gap-[22px]">
    <section>
      <h2 :class="H2">Avisos del sistema</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.notifications"
          name="Avisar de sucesos del sistema"
          desc="La isla se ensancha unos segundos."
        />
        <div :class="ROW">
          <label class="grid min-w-0 gap-0.5" for="notification-style">
            <span class="font-medium">Formato de los avisos</span>
            <span :class="DESC">Completa despliega la isla con el texto; Discreta es un círculo con el icono. Cada app puede tener el suyo.</span>
          </label>
          <SettingsSelect
            id="notification-style"
            v-model="settings.notificationStyle"
            :options="[
              { value: 'panel', label: 'Completa' },
              { value: 'bubble', label: 'Discreta' },
            ]"
          />
        </div>
        <SettingsToggle
          v-model="settings.notifyCharger"
          sub
          :disabled="!settings.notifications"
          name="Cargador"
          desc="Al conectarlo o desconectarlo."
        />
        <SettingsToggle
          v-model="settings.notifyVolume"
          sub
          :disabled="!settings.notifications"
          name="Volumen"
          desc="Al cambiarlo. No aparece con la isla abierta."
        />
        <SettingsToggle
          v-model="settings.notifyLowBattery"
          sub
          :disabled="!settings.notifications"
          name="Batería baja"
          desc="Al llegar al 20 % y al 10 %."
        />
        <SettingsToggle
          v-model="settings.notifyWifi"
          sub
          :disabled="!settings.notifications"
          name="Wi-Fi"
          desc="Al conectarse a una red."
        />
        <SettingsToggle
          v-model="settings.notifyInternet"
          sub
          :disabled="!settings.notifications"
          name="Internet"
          desc="Al perder o recuperar la conexión."
        />
        <SettingsToggle
          v-model="settings.notifyBluetooth"
          sub
          :disabled="!settings.notifications"
          name="Bluetooth"
          desc="Al conectar o desconectar un dispositivo."
        />
        <SettingsToggle
          v-model="settings.notifyTrack"
          sub
          :disabled="!settings.notifications"
          name="Canción"
          desc="Al empezar a sonar o cambiar de pista."
        />
        <SettingsToggle
          v-model="settings.notifyPrivacy"
          sub
          :disabled="!settings.notifications"
          name="Micrófono y cámara"
          desc="Punto de color mientras una app los usa. Solo lee su nombre."
        />
        <SettingsToggle
          v-model="settings.notifyDownloads"
          sub
          :disabled="!settings.notifications"
          name="Descargas"
          desc="Velocidad y aviso al terminar. Sin porcentaje."
        />
        <SettingsToggle
          v-model="settings.notifyUsb"
          sub
          :disabled="!settings.notifications"
          name="USB"
          desc="Al conectar o retirar una unidad."
        />
        <SettingsToggle
          v-model="settings.notifyLockKeys"
          sub
          :disabled="!settings.notifications"
          name="Bloq Mayús y Bloq Num"
          desc="Al activar o apagar."
        />
        <SettingsToggle
          v-model="settings.notifyFocus"
          sub
          :disabled="!settings.notifications"
          name="No molestar"
          desc="Al activar o apagar el modo."
        />
        <SettingsToggle
          v-model="settings.notifyBrightness"
          sub
          :disabled="!settings.notifications"
          name="Brillo"
          desc="Al cambiar el brillo del portátil."
        />
      </div>
    </section>
    </div>

    <div v-show="tab === 'apps'" :id="'panel-apps'" role="tabpanel" :aria-labelledby="'tab-apps'" class="tab-panel grid gap-[22px]">
    <section>
      <h2 :class="H2">Notificaciones de aplicaciones</h2>
      <div :class="PANEL">
        <SettingsToggle
          v-model="settings.appNotifications"
          name="Mostrar notificaciones en la isla"
          desc="Muestra remitente y mensaje; no se guarda nada. Con WhatsApp, descarga la foto del remitente. Requiere el acceso a notificaciones de Windows."
        />

        <!-- Sin permiso de Windows la opción parece activa pero no llega nada. -->
        <div
          v-if="settings.appNotifications && access !== 'allowed'"
          class="flex items-center justify-between gap-3 bg-[rgba(214,96,60,.1)] px-3.5 py-[11px] text-[11.5px] leading-[1.45] text-[#a8431f]"
          role="alert"
        >
          <span>
            <template v-if="access === 'unavailable'">
              Este equipo no permite leer notificaciones de otras apps.
            </template>
            <template v-else>
              Windows bloquea el acceso. Actívalo en Configuración → Privacidad → Notificaciones.
            </template>
          </span>
          <button
            v-if="access !== 'unavailable'"
            class="flex-none cursor-pointer rounded-lg border border-[rgba(214,96,60,.4)] bg-transparent px-[11px] py-[6px] text-[11.5px] text-[#a8431f] transition-colors duration-150 hover:bg-[rgba(214,96,60,.14)]"
            @click="call('toasts_open_privacy')"
          >Abrir Windows</button>
        </div>

        <template v-if="settings.appNotifications">
          <p
            v-if="settings.notificationApps.length === 0"
            class="m-0 px-3.5 py-[11px] text-[11.5px] leading-[1.45] text-muted"
          >
            Aún no se detectó ninguna app. Aparecerán al enviar una notificación, desactivadas.
          </p>
          <!-- Sin altura máxima: la ventana ya se desplaza, y un recuadro fijo con espacio libre debajo obligaba a desplazar dos veces. -->
          <div
            v-else
            class="[&>*]:relative [&>*+*]:before:absolute [&>*+*]:before:top-0 [&>*+*]:before:right-0 [&>*+*]:before:left-7 [&>*+*]:before:h-px [&>*+*]:before:bg-line [&>*+*]:before:content-['']"
          >
            <SettingsToggle
              v-for="entry in settings.notificationApps"
              :key="entry.name"
              v-model="entry.enabled"
              sub
              :name="entry.name"
            >
              <!-- Solo con la app activada. «General» sigue al formato de la pestaña Avisos. -->
              <Transition name="soft">
                <span v-if="entry.enabled" class="flex">
                  <SettingsSelect
                    :model-value="entry.style ?? 'default'"
                    :options="APP_STYLES"
                    compact
                    @update:model-value="entry.style = $event === 'default' ? null : ($event as 'panel' | 'bubble')"
                  />
                </span>
              </Transition>
            </SettingsToggle>
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
        class="cursor-pointer rounded-lg border border-line bg-transparent px-[13px] py-[7px] text-[12px] text-muted transition-colors duration-150 hover:bg-hover hover:text-ink"
        @click="restoreDefaults"
      >Restaurar valores</button>
    </footer>
  </div>
</template>
