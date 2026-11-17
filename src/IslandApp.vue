<script setup lang="ts">
/**
 * La isla como ventana flotante del escritorio real.
 *
 * A diferencia de la demo, aquí no hay escritorio simulado ni selector de
 * actividades: la ventana es del tamaño de la isla y se mueve y redimensiona
 * desde Rust según el estado.
 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import TheIsland from "@/components/TheIsland.vue";
import { useSystemMedia } from "@/lib/media";
import { accentFromArtwork } from "@/lib/artColor";
import { clearAudioLevels, pushAudioLevels } from "@/lib/audioFeed";
import { useSmoothPosition } from "@/lib/clock";
import { useLyrics } from "@/lib/lyrics";
import { noticeWidth, useNotifications } from "@/lib/notifications";
import { useSystemPower } from "@/lib/power";
import { tidyToast, toastMediaKind, useAudioLevels, useBluetoothEvents, useBrightnessEvents, useDownloadEvents, usePrivacyEvents, useSpeedtestEvents, useFocusEvents, useLockKeyEvents, useUsbEvents, useLowBatteryEvents, useToastEvents, useInternetEvents, useVolumeEvents, useWifiEvents } from "@/lib/systemEvents";
import type { Activity } from "@/lib/activities";

/**
 * Cuánto debe quedarse en pantalla una notificación según su mensaje.
 *
 * El mensaje ocupa hasta tres líneas y, si pasa de ahí, sube hasta enseñar el
 * final (ver ToastMarquee.vue); el aviso tiene que durar lo que tarda en llegar, o
 * se retiraría a medio leer. Se **estima** a partir de los caracteres, como
 * `noticeWidth()`: aquí no hay texto dibujado que medir. A 12,5 px un carácter ocupa
 * unos 6,2 px y la fila mide unos 232, así que caben unos 37 por línea. Cada línea
 * de más mide 16,25 px; el desplazamiento arranca a los 0,9 s, va a 14 px/s (SPEED
 * en ToastMarquee.vue) y se queda un par de segundos en el final.
 */
function toastDuration(body: string): number {
  const lines = Math.ceil(body.length / 37);
  const overflow = Math.max(0, lines - 3) * 16.25;
  if (overflow === 0) return 5500;
  const ms = 900 + (overflow / 14) * 1000 + 2200;
  return Math.min(20000, Math.max(5500, Math.round(ms)));
}

/** Tamaños de la isla, en sincronía con las clases de TheIsland.vue. */
const COMPACT = { width: 116, height: 34 };
/** Pestaña mínima de la vista minimizada alternativa. Su alto (8 px) lo usa Rust para elegir la muesca. */
const TAB = { width: 48, height: 8 };
const EXPANDED = { width: 330, height: 108 };

/** Con la letra abierta la isla se alarga para dejarle sitio al panel. */
const WITH_LYRICS = { width: 540, height: 108 };

/** La canción presentada usa la mitad del alto del panel: ver `showsTrackBar` en TheIsland.vue. */
const TRACK = { width: 330, height: 54 };

const mode = ref<Activity>("music");
const expanded = ref(false);
/** Lado de la pantalla al que se pega la isla, según los ajustes. */
type Alignment = "left" | "center" | "right";
const alignment = ref<Alignment>("center");

/**
 * Alineación de la zona visible dentro de la ventana fija.
 *
 * La ventana ya lleva su margen (WINDOW_PADDING en Rust): centrada, la isla no
 * necesita relleno. Con alineación a un lado la zona visible se pega a ese
 * borde (ver `place_island` en lib.rs), y los 50 px son WINDOW_PADDING +
 * OVERSHOOT_MARGIN. Cada rama lleva su propio `justify-items`: dos utilidades
 * así en el mismo elemento se pisarían según el orden en que Tailwind las emite.
 */
const stageAlignment = computed(() => {
  if (alignment.value === "left") return "justify-items-start pl-[50px]";
  if (alignment.value === "right") return "justify-items-end pr-[50px]";
  return "justify-items-center";
});
/** Aspecto de la isla cerrada, según los ajustes: píldora o pestaña mínima. */
type CollapsedStyle = "pill" | "tab";
/** Estilo de la isla, cerrada y abierta: con muescas (colgada del borde) o flotante redondeada. */
type IslandStyle = "notch" | "round";
const collapsedStyle = ref<CollapsedStyle>("pill");
const islandStyle = ref<IslandStyle>("notch");
/**
 * Color del anillo que suaviza el borde del estilo flotante: el del escritorio alrededor de la isla (lo muestrea
 * Rust, ver `sample_backdrop`). Hasta la primera lectura, un gris oscuro neutro.
 */
const ring = ref("rgb(38, 41, 43)");
const media = useSystemMedia();

/** Cola de avisos: interrumpen la píldora compacta y se retiran solos. */
const notifications = useNotifications();
const notice = notifications.current;

/**
 * Aviso al conectar o desconectar el cargador.
 *
 * Solo se anuncia en equipos con batería, y el estado inicial no cuenta:
 * arrancar enchufado no es una novedad.
 */
useSystemPower((state) => {
  notifications.notify({
    kind: "power",
    label: state.plugged ? `${state.percent}% · Cargando` : `${state.percent}%`,
    // Al enchufar, el aviso es grande y se queda algo más: la animación de la
    // batería necesita su segundo largo para verse entera. Al desenchufar se
    // despliega con el formato genérico.
    duration: state.plugged ? 4500 : 3200,
    data: state.plugged
      ? { plugged: true, percent: state.percent, large: true }
      : {
          plugged: false,
          percent: state.percent,
          deployed: true,
          title: "Cargador desconectado",
          subtitle: `${state.percent}% de batería`,
          level: state.percent,
        },
  });
});

/**
 * Aviso de volumen al cambiarlo desde el sistema (teclas, mezclador…).
 *
 * Con la isla abierta se ignora: el panel ya tiene su control deslizante, que
 * muestra el valor, y un aviso encima taparía justo lo que el usuario está
 * moviendo. El silencio se anuncia como texto, no como un 0 %.
 */
useVolumeEvents((change) => {
  if (expanded.value) return;
  notifications.notify({
    kind: "volume",
    label: change.muted || change.level === 0 ? "Silencio" : `Volumen ${change.level}%`,
    duration: 1800,
    data: {
      muted: change.muted || change.level === 0,
      level: change.muted ? 0 : change.level,
      deployed: true,
      title: change.muted || change.level === 0 ? "Silencio" : "Volumen",
      subtitle: change.muted || change.level === 0 ? "Sonido desactivado" : `${change.level}%`,
    },
  });
});

/**
 * Notificación de otra aplicación (Teams, Outlook…).
 *
 * Se apilan en lugar de reemplazarse: dos mensajes seguidos son dos mensajes.
 * Con la isla abierta ocupa la isla entera, como cualquier otro aviso, y al
 * retirarse vuelve el contenido.
 */
useToastEvents((toast) => {
  const { title, body } = tidyToast(toast);
  const text = [title, body].filter(Boolean).join(": ");
  notifications.notify({
    kind: "toast",
    label: `${toast.app} · ${text}`,
    duration: toastDuration(body),
    stack: true,
    data: { tall: true, id: toast.id, app: toast.app, title, body, media: toastMediaKind(body)?.kind ?? null, mediaDetail: toastMediaKind(body)?.detail ?? "", avatar: toast.avatar ?? null, icon: toast.icon ?? null },
  });
});

/**
 * Aviso al conectarse a una red Wi-Fi.
 *
 * El nombre de la red se recorta: `noticeWidth` estima el ancho a partir del
 * texto y un SSID largo se comería la pantalla.
 */
useWifiEvents((wifi) => {
  if (!wifi.ssid) return;
  const name = wifi.ssid.length > 18 ? `${wifi.ssid.slice(0, 17)}…` : wifi.ssid;
  notifications.notify({
    kind: "wifi",
    label: `Conectado · ${name}`,
    duration: 3600,
    // Aquí el nombre completo: el título y el subtítulo se recortan con
    // puntos suspensivos en el propio panel, no hace falta acortarlo.
    data: { signal: wifi.signal, deployed: true, title: "Conectado", subtitle: wifi.ssid },
  });
});

/**
 * Aviso al perder o recuperar internet.
 *
 * La recuperación usa el tipo `wifi`: es el mismo aviso azul y, si justo se
 * acaba de anunciar otro, lo reemplaza en vez de apilarse.
 */
useInternetEvents((online) => {
  notifications.notify(
    online
      ? {
          kind: "wifi",
          label: "Internet restablecido",
          duration: 3600,
          data: { deployed: true, title: "Internet restablecido", subtitle: "Conexión activa" },
        }
      : {
          kind: "internet",
          label: "Sin internet",
          duration: 4500,
          data: { deployed: true, title: "Sin internet", subtitle: "Revisa tu conexión" },
        },
  );
});

/**
 * Aviso al conectar o desconectar un dispositivo Bluetooth.
 *
 * Cada sentido lleva su propio tipo para que conectar y desconectar seguidos
 * no se reemplacen entre sí: son dos hechos distintos. El nombre va completo:
 * el panel desplegado lo trunca con puntos suspensivos.
 */
useBluetoothEvents((change) => {
  notifications.notify({
    kind: change.connected ? "bluetooth" : "bluetooth-off",
    label: `${change.connected ? "Conectado" : "Desconectado"} · ${change.name}`,
    duration: 3600,
    data: {
      deployed: true,
      title: change.connected ? "Conectado" : "Desconectado",
      subtitle: change.name,
    },
  });
});

/**
 * Micrófono y cámara en uso: un punto de color en la isla mientras dure y un aviso al empezar.
 * El backend manda el estado completo cada vez que cambia; el aviso solo salta al pasar de
 * «nadie» a «alguien», no cuando cambia la aplicación que ya lo usaba.
 */
const privacy = ref<{ microphone: string | null; camera: string | null } | null>(null);
let previousPrivacy = { microphone: null as string | null, camera: null as string | null };
usePrivacyEvents((state) => {
  privacy.value = state;
  if (state.microphone && !previousPrivacy.microphone) {
    notifications.notify({
      kind: "mic",
      label: `Micrófono en uso · ${state.microphone}`,
      duration: 3200,
      data: { deployed: true, title: "Micrófono en uso", subtitle: state.microphone },
    });
  }
  if (state.camera && !previousPrivacy.camera) {
    notifications.notify({
      kind: "camera",
      label: `Cámara en uso · ${state.camera}`,
      duration: 3200,
      data: { deployed: true, title: "Cámara en uso", subtitle: state.camera },
    });
  }
  previousPrivacy = state;
});

/**
 * Niveles del audio real para la onda (ajuste `audioVisualizer`). Van a `audioFeed`, fuera de la
 * reactividad (ver audioFeed.ts): llegan unas 60 veces por segundo y cada una repintaría la isla entera.
 */
const audioOn = ref(false);
useAudioLevels((levels) => {
  if (audioOn.value) pushAudioLevels(levels);
});
watch(audioOn, (on) => {
  if (!on) clearAudioLevels();
});

/** Velocidad legible: «3,2 MB/s» o «840 KB/s». */
function formatBytesPerSecond(bytes: number): string {
  const mb = bytes / 1_000_000;
  if (mb >= 1) return `${mb.toLocaleString("es", { maximumFractionDigits: 1 })} MB/s`;
  return `${Math.round(bytes / 1000)} KB/s`;
}

/**
 * Descargas del navegador: una línea recorre la píldora mientras dura, un aviso al empezar
 * (con la velocidad) y otro al terminar (con el nombre). Sin porcentaje: ver downloads.rs.
 */
const downloading = ref(false);
useDownloadEvents(
  (progress) => {
    const started = progress.active && !downloading.value;
    downloading.value = progress.active;
    if (started) {
      notifications.notify({
        kind: "download",
        label: "Descargando",
        duration: 2800,
        data: { deployed: true, title: "Descargando", subtitle: progress.speed > 0 ? formatBytesPerSecond(progress.speed) : "En curso" },
      });
    }
  },
  (done) => {
    const name = done.name.length > 30 ? `${done.name.slice(0, 29)}…` : done.name;
    notifications.notify({
      kind: "download",
      label: "Descarga completada",
      duration: 4200,
      data: { deployed: true, title: "Descarga completada", subtitle: name },
    });
  },
);

/** El tiempo de la vista de inicio (ajustes `weather` y `weatherCity`; ver weather.rs). */
const weatherOn = ref(false);
const weatherCity = ref("");
const weather = ref<{ city: string; temp: number; text: string; kind: string; max: number | null; min: number | null } | null>(null);
async function refreshWeather() {
  if (!weatherOn.value || !weatherCity.value.trim()) {
    weather.value = null;
    return;
  }
  const result = await invokeResult<{ city: string; temp: number; text: string; kind: string; max: number | null; min: number | null } | null>("weather_current");
  // Si la consulta falla (sin red) se conserva el último dato en vez de borrarlo.
  if (result) weather.value = result;
}
// La ciudad se teclea en los ajustes y cada letra llega como un cambio: se espera a que pare.
let weatherDebounce: number | undefined;
watch([weatherOn, weatherCity], () => {
  weather.value = null;
  if (weatherDebounce !== undefined) window.clearTimeout(weatherDebounce);
  weatherDebounce = window.setTimeout(() => void refreshWeather(), 900);
});
let weatherTimer: number | undefined;
onMounted(() => {
  weatherTimer = window.setInterval(() => void refreshWeather(), 10 * 60 * 1000);
});
onUnmounted(() => {
  if (weatherTimer !== undefined) window.clearInterval(weatherTimer);
});

/**
 * Test de velocidad (página de velocidad de la vista de inicio; ver speedtest.rs).
 *
 * El estado vive aquí y no en la página: así el resultado sigue ahí si se cambia de página o
 * se cierra la isla y se vuelve a abrir. Mientras mide, la página enseña la fase y la velocidad
 * del momento; al terminar, la bajada y la subida. Si en ese momento no se está mirando esa
 * página, un aviso avisa del resultado (el test dura unos 12 s y nadie se queda mirando).
 */
const speedtestOn = ref(true);
type SpeedState = {
  status: "idle" | "running" | "done" | "error";
  phase: "ping" | "down" | null;
  mbps: number;
  down: number | null;
  ping: number | null;
  /** Por qué falló, si falló: el mensaje es distinto si el servicio limita las pruebas. */
  reason: "limit" | "network" | null;
};
const speed = ref<SpeedState>({ status: "idle", phase: null, mbps: 0, down: null, ping: null, reason: null });
/** Página de la vista de inicio: la hora en el centro; CPU y RAM a un lado, la velocidad al otro. */
const homePage = ref<"clock" | "system" | "speed" | "music">("clock");
/** Megabits legibles, con coma decimal: «538», «79,2». */
const formatMbps = (value: number) => (value >= 100 ? value.toFixed(0) : value.toFixed(1).replace(".", ","));
/**
 * Aplica el resultado del test, una sola vez. Llega por dos caminos (el evento `speedtest://done`
 * y el valor que devuelve el comando) y no se sabe cuál llega antes; el que llega segundo ve que la
 * prueba ya no está «midiendo» y no hace nada, así el aviso no sale duplicado ni la página parpadea.
 */
function applySpeedResult(result: { down: number | null; ping: number | null; error?: "limit" | "network" | null } | null | undefined) {
  if (speed.value.status !== "running") return;
  if (!result || result.down === null) {
    speed.value = { status: "error", phase: null, mbps: 0, down: null, ping: null, reason: result?.error ?? "network" };
  } else {
    speed.value = { status: "done", phase: null, mbps: 0, down: result.down, ping: result.ping, reason: null };
  }
  // Con su página a la vista no hace falta aviso: el resultado ya está delante.
  if (expanded.value && homePage.value === "speed") return;
  if (!result || result.down === null) {
    const limited = result?.error === "limit";
    notifications.notify({
      kind: "speed",
      label: "No se pudo medir",
      duration: 5000,
      data: { deployed: true, title: "No se pudo medir", subtitle: limited ? "Demasiadas pruebas seguidas" : "Comprueba tu conexión" },
    });
    return;
  }
  notifications.notify({
    kind: "speed",
    label: `↓ ${formatMbps(result.down)} Mbps`,
    duration: 9000,
    data: { deployed: true, title: `↓ ${formatMbps(result.down)} Mbps`, subtitle: result.ping === null ? "Bajada" : `Latencia ${Math.round(result.ping)} ms` },
  });
}
async function startSpeedtest() {
  if (speed.value.status === "running" || !speedtestOn.value) return;
  // Las dos cifras empiezan vacías y se llenan en orden: bajada, luego subida.
  speed.value = { status: "running", phase: "ping", mbps: 0, down: null, ping: null, reason: null };
  const result = await invokeResult<{ down: number | null; ping: number | null; error?: "limit" | "network" | null } | null>("speedtest_run");
  applySpeedResult(result);
}
useSpeedtestEvents(
  (progress) => {
    if (speed.value.status !== "running") return;
    // La cifra de la fase en curso va en directo; la de la fase anterior se queda como estaba
    // hasta que llegue el resultado final, que es el promedio de toda la fase.
    speed.value = {
      ...speed.value,
      phase: progress.phase,
      mbps: progress.mbps,
      down: progress.phase === "down" ? progress.mbps : speed.value.down,
    };
  },
  (result) => applySpeedResult(result),
);

/** Unidad extraíble conectada o retirada. Un mismo tipo: al enchufar y sacar seguido interesa el estado final. */
useUsbEvents((change) => {
  const subtitle = `${change.name || "Unidad extraíble"} (${change.letter}:)`;
  notifications.notify({
    kind: "usb",
    label: `${change.connected ? "Unidad conectada" : "Unidad retirada"} · ${subtitle}`,
    duration: 3600,
    data: { deployed: true, title: change.connected ? "Unidad conectada" : "Unidad retirada", subtitle },
  });
});

/** Bloq Mayús y Bloq Num. Cada tecla con su tipo, para que no se reemplacen entre sí. */
useLockKeyEvents(({ key, on }) => {
  const caps = key === "caps";
  const title = `${caps ? "Bloq Mayús" : "Bloq Num"} ${on ? "activado" : "desactivado"}`;
  notifications.notify({
    kind: key,
    label: title,
    duration: 1800,
    data: {
      deployed: true,
      title,
      subtitle: caps
        ? on ? "Las letras salen en mayúsculas" : "Las letras salen en minúsculas"
        : on ? "Teclado numérico activo" : "Teclado numérico apagado",
    },
  });
});

/** Modo No molestar. */
useFocusEvents((on) => {
  notifications.notify({
    kind: "focus",
    label: on ? "No molestar activado" : "No molestar desactivado",
    duration: 2800,
    data: {
      deployed: true,
      title: on ? "No molestar" : "No molestar desactivado",
      subtitle: on ? "Avisos silenciados" : "Los avisos vuelven",
    },
  });
});

/**
 * Brillo de la pantalla. A diferencia del volumen, se anuncia con la isla abierta:
 * su panel no tiene un control de brillo que ya lo muestre. Mientras se sigue
 * pulsando la tecla, el mismo tipo actualiza el aviso en el sitio.
 */
useBrightnessEvents((level) => {
  notifications.notify({
    kind: "brightness",
    label: `Brillo ${level}%`,
    duration: 1800,
    data: { level, deployed: true, title: "Brillo", subtitle: `${level}%` },
  });
});

/** Aviso al cruzar el 20 % y el 10 % de batería sin cargador. */
useLowBatteryEvents((state) => {
  notifications.notify({
    kind: "battery",
    label: state.percent <= 10 ? `Batería crítica ${state.percent}%` : `Batería baja ${state.percent}%`,
    duration: 5000,
    data: {
      percent: state.percent,
      deployed: true,
      title: state.percent <= 10 ? "Batería crítica" : "Batería baja",
      subtitle: `${state.percent}% · conecta el cargador`,
      level: state.percent,
    },
  });
});

const formatTime = (seconds: number) =>
  `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;

const playing = computed(() => media.state.value.playing);
const trackTitle = computed(() => media.state.value.title || "Sin reproducción");

/**
 * ¿Hay algún medio con el que el reproductor tenga algo que enseñar?
 *
 * Sin sesión SMTC, o con una sesión vacía y parada, el reproductor solo
 * mostraría «Sin reproducción» y una barra a cero. Una pista en pausa sí
 * cuenta: el usuario puede querer reanudarla.
 */
const mediaPresent = computed(() => {
  const { hasSession, title, playing } = media.state.value;
  return hasSession && (title !== "" || playing);
});
/**
 * `mediaPresent` con un margen al perderse. Al cambiar de pista, según el reproductor y el equipo, SMTC pasa un
 * instante sin sesión o con una sesión vacía y parada; sin margen, la isla creía que ya no había nada, saltaba a
 * la hora y no volvía hasta pasados los 15 s de inactividad. Ganar medios es inmediato; perderlos tarda
 * `MEDIA_LOSS_GRACE_MS`.
 */
const MEDIA_LOSS_GRACE_MS = 3000;
const hasMedia = ref(mediaPresent.value);
let mediaLossTimer: number | undefined;
watch(mediaPresent, (present) => {
  if (mediaLossTimer !== undefined) window.clearTimeout(mediaLossTimer);
  mediaLossTimer = undefined;
  if (present) hasMedia.value = true;
  else mediaLossTimer = window.setTimeout(() => (hasMedia.value = false), MEDIA_LOSS_GRACE_MS);
});

/**
 * Lo que se dibuja de verdad: sin medios, el modo «música» cede a la vista de
 * inicio. `mode` no cambia, así que en cuanto algo empieza a sonar la isla
 * vuelve sola al reproductor, sin que nadie tenga que acordarse de volver.
 */
const displayMode = computed<Activity>(() =>
  mode.value === "music" && !hasMedia.value ? "home" : mode.value,
);

/**
 * El reproductor es la página principal mientras suena algo.
 *
 * - La isla se abre en él (y al cerrar se prepara para abrirse en él); si no suena nada, en la hora.
 * - Si se rota a otra página (la hora, CPU y RAM, la velocidad) y se deja sin tocar, **vuelve sola al
 *   reproductor** pasados unos segundos: «en primer plano» mientras se usa. Cada movimiento del puntero
 *   por la isla reinicia la cuenta, y no corre mientras mide un test de velocidad, para no
 *   arrebatarle la pantalla a una medición en curso.
 */
const IDLE_BACK_MS = 15_000;
const defaultPage = () => (hasMedia.value ? "music" : "clock");
watch(expanded, () => {
  homePage.value = defaultPage();
});
watch(hasMedia, (has) => {
  // Si lo que sonaba se para mientras se mira el reproductor, no queda una página vacía; y si algo
  // empieza a sonar con la isla cerrada, la próxima vez se abre en él.
  if (!has && homePage.value === "music") homePage.value = "clock";
  if (has && !expanded.value) homePage.value = "music";
});
// Con la letra abierta la isla se alarga: al salir del reproductor se cierra, o la página siguiente
// se quedaría en 540 px de ancho.
watch(homePage, (page) => {
  if (page !== "music") lyricsOpen.value = false;
});
let backTimer: number | undefined;
function armBackToPlayer() {
  if (backTimer !== undefined) window.clearTimeout(backTimer);
  backTimer = undefined;
  if (!expanded.value || !hasMedia.value || homePage.value === "music" || speed.value.status === "running") return;
  backTimer = window.setTimeout(() => {
    if (expanded.value && hasMedia.value && speed.value.status !== "running") homePage.value = "music";
  }, IDLE_BACK_MS);
}
watch([homePage, expanded, hasMedia, () => speed.value.status], armBackToPlayer);
onMounted(() => window.addEventListener("pointermove", armBackToPlayer, { passive: true }));
onUnmounted(() => {
  window.removeEventListener("pointermove", armBackToPlayer);
  if (backTimer !== undefined) window.clearTimeout(backTimer);
});
const trackSubtitle = computed(() => {
  const { artist, album } = media.state.value;
  return [artist, album].filter(Boolean).join(" · ");
});
const trackInitial = computed(() => trackTitle.value.charAt(0).toUpperCase() || "M");
const trackArtwork = computed(() => media.state.value.artwork);

/**
 * Tono de la carátula, para la onda de sonido. Vacío (verde de siempre) hasta
 * calcularlo o si la pista no tiene portada. Se descarta el resultado si, mientras
 * se calculaba, ya cambió la pista: pintaría el color de la anterior.
 */
const artAccent = ref("");
watch(
  trackArtwork,
  async (art) => {
    if (!art) {
      artAccent.value = "";
      return;
    }
    const accent = await accentFromArtwork(art);
    if (trackArtwork.value === art) artAccent.value = accent ?? "";
  },
  { immediate: true },
);
/** Algunos reproductores no permiten saltar de posición (Apple Music). */
const canSeek = computed(() => media.state.value.canSeek);
const positionLabel = computed(() => formatTime(media.state.value.position));
const durationLabel = computed(() => formatTime(media.state.value.duration));
/** Tiempo que falta, en negativo, como en los reproductores del sistema. */
const remainingLabel = computed(() => {
  const { position, duration } = media.state.value;
  if (duration <= 0) return formatTime(0);
  return `-${formatTime(Math.max(0, duration - position))}`;
});
const trackProgress = computed(() => {
  const { position, duration } = media.state.value;
  if (duration <= 0) return 0;
  return Math.min(100, Math.max(0, (position / duration) * 100));
});

/** Llama a un comando de Tauri, ignorando el fallo fuera de la app. */
async function invokeWindow(command: string, args?: Record<string, unknown>) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke(command, args);
  } catch {
    // Fuera de la app de escritorio no hay ventana que ajustar.
  }
}

/** Igual que invokeWindow, pero devolviendo el resultado del comando. */
async function invokeResult<T>(command: string, args?: Record<string, unknown>) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return undefined;
  }
}

// La ventana sigue al tamaño de la isla para no dejar zona muerta alrededor.
// Compacta ocupa 104x32: lo justo para la píldora, así que apenas tapa nada
// del escritorio y no hace falta dejar pasar los clics. (En Windows, activar
// el clic-atraviesa sobre una ventana transparente la vuelve invisible.)
/** Tamaño que debe tener la ventana según lo que haya abierto. */
const targetSize = computed(() => {
  // Cerrada, los avisos despliegan la isla al tamaño de su panel. Abierta no
  // cambia el tamaño: el panel del aviso ocupa la isla tal como está (con o sin
  // letra) y, al retirarse, vuelve el contenido.
  if (!expanded.value) {
    // La canción que se presenta mide la mitad del panel; va antes que `deployed`.
    if (notice.value?.kind === "track") return TRACK;
    // El aviso del cargador crece al tamaño del panel (ver `showsCharge`).
    if (notice.value?.data?.large || notice.value?.data?.deployed) return EXPANDED;
    // La notificación de otra app mide lo mismo que el panel: avatar, remitente y mensaje.
    if (notice.value?.data?.tall) return EXPANDED;
    return notice.value
      ? { width: noticeWidth(notice.value.label), height: COMPACT.height }
      : collapsedStyle.value === "tab"
        ? TAB
        : COMPACT;
  }
  return lyricsOpen.value && lyricsAvailable.value ? WITH_LYRICS : EXPANDED;
});

// `immediate` asegura que la ventana adopte el tamaño compacto al arrancar:
// sin él conservaría el declarado en la configuración hasta el primer cambio.
/**
 * Pide a Rust el tamaño de la isla.
 *
 * La ventana nativa tiene tamaño fijo: Rust solo recorta la zona visible
 * (`SetWindowRgn`), así que no hay viewport que actualizar ni fotograma
 * viejo que se cuele. Por eso la isla puede centrarse sin más en la ventana.
 */
function resizeWindow(size: { width: number; height: number }, durationMs?: number) {
  invokeWindow("island_resize", { ...size, durationMs });
}

/**
 * Duración de la transición de tamaño.
 *
 * Cuando es el usuario quien abre o cierra la isla, 0,58 s: un gesto propio se
 * acompaña con calma. Un aviso del sistema llega sin que nadie lo pida y no debe
 * hacer esperar: se abre en 0,40 s y se retira en 0,34 s, algo más rápido, porque
 * lo que sale no necesita lucirse. La misma curva sin rebote en todos.
 *
 * Va en la variable CSS `--morph` y se pasa a Rust a la vez: el recorte nativo
 * debe durar lo mismo que la píldora.
 */
const MORPH_USER_MS = 580;
const MORPH_NOTICE_IN_MS = 400;
const MORPH_NOTICE_OUT_MS = 460;
const morphMs = ref(MORPH_USER_MS);
let hadNotice = false;

/**
 * Sincroniza el recorte de la ventana con el tamaño de la isla.
 *
 * Rust anima el recorte con la misma curva y duración que el CSS de la
 * píldora (`animate_island`), así que se avisa al mismo tiempo al crecer y al
 * encoger. Antes, al encoger se esperaba 640 ms y el recorte viejo seguía a la
 * vista mientras la píldora ya era pequeña: dos tamaños y un salto final.
 */
watch(
  targetSize,
  (size, previous) => {
    // El tamaño del aviso es un objeto nuevo en cada evaluación, así que el
    // watch salta aunque las medidas no cambien: sin esta comparación por
    // valor, actualizar el texto de un aviso redimensionaría la ventana al
    // mismo tamaño que ya tenía, y ese `SetWindowPos` de más se ve parpadear.
    const hasNotice = !!notice.value;
    // Un aviso que aparece, cambia o se retira es cosa del sistema; el resto,
    // del usuario. Se calcula antes del `return` para no dejar `hadNotice` viejo.
    const duration = hasNotice ? MORPH_NOTICE_IN_MS : hadNotice ? MORPH_NOTICE_OUT_MS : MORPH_USER_MS;
    hadNotice = hasNotice;
    if (previous && size.width === previous.width && size.height === previous.height) return;

    morphMs.value = duration;
    resizeWindow(size, duration);
  },
  { immediate: true },
);

watch(expanded, (isExpanded) => {
  if (!isExpanded) openedByHover = false;
  // El volumen puede haber cambiado desde el sistema mientras estaba cerrada.
  if (isExpanded) refreshVolume();
  // Al contraerse, la letra se cierra para no reabrirse enorme.
  else lyricsOpen.value = false;
});

/** Letra sincronizada de la pista, y si su panel está abierto. */
const lyricsOpen = ref(false);
const trackKey = computed(() => {
  const { title, artist } = media.state.value;
  return title ? `${title} — ${artist}` : "";
});
// Reloj local: la posición del sistema llega a saltos de 250 ms y la letra
// necesita seguirla fotograma a fotograma. Solo corre con el panel abierto.
const trackPosition = useSmoothPosition(() => media.state.value, lyricsOpen);
const lyrics = useLyrics(trackKey, trackPosition);
// Se exponen sueltos: dentro del template un computed anidado en un objeto
// no se desenvuelve solo, y `lyrics.available.value` quedaría indefinido.
const lyricsAvailable = lyrics.available;
const lyricLines = lyrics.visibleLines;

/** Volumen maestro del sistema, de 0 a 100. */
const volume = ref(50);
const muted = ref(false);

/** Relee el volumen real del sistema. */
async function refreshVolume() {
  const result = await invokeResult<[number, boolean]>("volume_get");
  if (result) {
    volume.value = result[0];
    muted.value = result[1];
  }
}

function setVolume(value: number) {
  volume.value = value;
  if (muted.value && value > 0) muted.value = false;
  invokeWindow("volume_set", { value });
}

async function toggleMute() {
  const next = await invokeResult<boolean>("volume_toggle_mute");
  if (next !== undefined) muted.value = next;
}

/**
 * Abrir al pasar el ratón (ajuste `hoverOpen`), además del clic.
 *
 * Un pequeño retardo al entrar evita que la isla se abra al cruzarla de paso, y
 * otro al salir evita que parpadee si el ratón roza el borde. Una isla abierta
 * así se recoge sola al alejarse; con un clic encima se «fija» y deja de hacerlo,
 * como si se hubiera abierto con el clic. Con un aviso en pantalla no se abre:
 * el primer toque tiene que descartarlo, y desplegarla debajo lo taparía.
 */
const hoverOpen = ref(true);
let openedByHover = false;
let hoverTimer: number | undefined;
const clearHoverTimer = () => {
  if (hoverTimer !== undefined) window.clearTimeout(hoverTimer);
  hoverTimer = undefined;
};

/** El ratón está sobre la isla / sobre la franja entre ella y el borde (vista redondeada). */
let overIsland = false;
let inGap = false;

function handleIslandEnter() {
  overIsland = true;
  clearHoverTimer();
  if (!hoverOpen.value || expanded.value || notice.value) return;
  hoverTimer = window.setTimeout(() => {
    if (notice.value || expanded.value) return;
    openedByHover = true;
    expanded.value = true;
  }, 140);
}

function collapseIfHoverOpened() {
  clearHoverTimer();
  if (!openedByHover) return;
  hoverTimer = window.setTimeout(() => {
    if (!openedByHover) return;
    openedByHover = false;
    expanded.value = false;
  }, 320);
}

function handleIslandLeave(event: MouseEvent) {
  overIsland = false;
  clearHoverTimer();
  // Subir hasta la franja del borde sigue siendo estar en la isla.
  if (inGap) return;
  if (!openedByHover) return;
  // Arrastrando la barra de progreso o el volumen el ratón puede salirse de la
  // isla sin querer soltarla: se espera a que suelte para decidir.
  if (event.buttons !== 0) {
    window.addEventListener(
      "pointerup",
      () => {
        const stillOver = document.querySelector(".island")?.matches(":hover") ?? false;
        if (!stillOver) collapseIfHoverOpened();
      },
      { once: true },
    );
    return;
  }
  collapseIfHoverOpened();
}

function toggleIsland() {
  // Con un aviso en pantalla, el primer toque lo descarta, esté la isla como
  // esté: abrirla o cerrarla debajo dejaría el aviso tapado y la cola seguiría
  // corriendo por detrás. En una notificación de otra aplicación, además abre esa
  // aplicación (Rust busca cuál por el id; no abre el chat concreto).
  if (notice.value) {
    const id = notice.value.kind === "toast" ? notice.value.data?.id : undefined;
    if (typeof id === "number") void invokeResult("toast_open", { id });
    notifications.dismiss();
    return;
  }
  // Un clic sobre una isla abierta por el ratón la deja abierta, no la cierra.
  if (expanded.value && openedByHover) {
    openedByHover = false;
    return;
  }
  openedByHover = false;
  expanded.value = !expanded.value;
}

function handleIslandClick(event: MouseEvent) {
  if (event.target instanceof Element && event.target.closest("button")) return;
  toggleIsland();
}

function handleIslandKeydown(event: KeyboardEvent) {
  if (event.target !== event.currentTarget) return;
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    toggleIsland();
  }
}

/** Segundos de salto, leídos de los ajustes del usuario. */
const skipSeconds = ref(10);
/** Segundos en la hora de la vista de inicio. Con `!== undefined`, no por verdad: `false` es un valor válido. */
const homeSeconds = ref(true);
/** Ajustes que gobiernan la presentación de la canción (`notifications` y `notifyTrack`). */
const notificationsOn = ref(true);
const trackNoticeOn = ref(true);

/**
 * Presenta la canción cuando empieza a sonar o cambia de pista, también si el
 * cambio viene de otra aplicación (el reproductor, el navegador, las teclas
 * multimedia): todo llega por SMTC, da igual quién lo provocó.
 *
 * - La primera lectura es la línea base: abrir la isla con música sonando ya no
 *   es una novedad.
 * - Con la isla abierta no se presenta: el reproductor ya enseña la canción.
 * - Se espera un instante antes de avisar: al cambiar de pista SMTC pasa por
 *   estados intermedios (título nuevo con la carátula vieja, pausa breve) y se
 *   presentaría dos veces o con la imagen equivocada.
 * - Si la carátula llega después del aviso, se actualiza en el sitio (mismo tipo).
 */
let trackPrimed = false;
let lastTrack = "";
let lastPlaying = false;
let trackTimer: number | undefined;
let presented: { track: string; at: number; artworkLength: number } | null = null;

function presentTrack() {
  const s = media.state.value;
  if (!trackNoticeOn.value || !notificationsOn.value || expanded.value) return;
  if (!s.hasSession || !s.playing || !s.title) return;
  presented = { track: `${s.title}|${s.artist}`, at: Date.now(), artworkLength: s.artwork.length };
  notifications.notify({
    kind: "track",
    label: s.title,
    duration: 4200,
    data: { deployed: true, title: s.title, subtitle: s.artist || s.album, art: s.artwork || null },
  });
}

watch(
  () => {
    const s = media.state.value;
    return `${media.live.value}|${s.hasSession}|${s.playing}|${s.title}|${s.artist}`;
  },
  () => {
    const s = media.state.value;
    // Fuera de la app de escritorio la pista es de ejemplo: no se presenta.
    if (!media.live.value) return;
    const track = `${s.title}|${s.artist}`;
    const playing = s.hasSession && s.playing && !!s.title;
    if (!trackPrimed) {
      trackPrimed = true;
      lastTrack = track;
      lastPlaying = playing;
      return;
    }
    const started = playing && !lastPlaying;
    const changed = playing && track !== lastTrack;
    lastTrack = track;
    lastPlaying = playing;
    if (!started && !changed) return;
    if (trackTimer !== undefined) window.clearTimeout(trackTimer);
    trackTimer = window.setTimeout(presentTrack, 700);
  },
);

// La carátula de un reproductor web llega después del título.
watch(
  () => media.state.value.artwork.length,
  (length) => {
    const s = media.state.value;
    if (!presented || Date.now() - presented.at > 4000) return;
    if (presented.track !== `${s.title}|${s.artist}` || length === presented.artworkLength) return;
    if (notice.value?.kind === "track") presentTrack();
  },
);
/** Mostrar CPU y memoria en la vista de inicio (ajuste `homeStats`). */
const showHomeStats = ref(true);
/** Última lectura de CPU y memoria; `null` hasta tener una con sentido. */
const homeStats = ref<{ cpu: number; memory: number; memoryUsedGb: number; memoryTotalGb: number } | null>(null);

/**
 * Mide CPU y memoria solo mientras su página de la vista de inicio está abierta y a la vista.
 *
 * El uso de CPU se calcula entre dos consultas seguidas, así que la primera no
 * sirve y se descarta: sin eso la barra arrancaría siempre en 0 %.
 */
let statsTimer: number | undefined;
watch(
  () => expanded.value && showHomeStats.value && homePage.value === "system",
  (visible) => {
    if (statsTimer !== undefined) window.clearInterval(statsTimer);
    statsTimer = undefined;
    homeStats.value = null;
    if (!visible) return;
    let primed = false;
    const sample = async () => {
      const stats = await invokeResult<{ cpu: number; memory: number; memoryUsedGb: number; memoryTotalGb: number }>("system_stats");
      if (!stats) return;
      if (primed) homeStats.value = { cpu: stats.cpu, memory: stats.memory, memoryUsedGb: stats.memoryUsedGb, memoryTotalGb: stats.memoryTotalGb };
      primed = true;
    };
    void sample();
    statsTimer = window.setInterval(sample, 1500);
  },
  { immediate: true },
);

onMounted(async () => {
  const stored = await invokeResult<{
    skipSeconds?: number;
    position?: Alignment;
    collapsedStyle?: CollapsedStyle;
    islandStyle?: IslandStyle;
    homeSeconds?: boolean;
    homeStats?: boolean;
    audioVisualizer?: boolean;
    weather?: boolean;
    weatherCity?: string;
    speedtest?: boolean;
    hoverOpen?: boolean;
    notifications?: boolean;
    notifyTrack?: boolean;
  }>("settings_get");
  if (stored?.homeSeconds !== undefined) homeSeconds.value = stored.homeSeconds;
  if (stored?.hoverOpen !== undefined) hoverOpen.value = stored.hoverOpen;
  if (stored?.notifications !== undefined) notificationsOn.value = stored.notifications;
  if (stored?.notifyTrack !== undefined) trackNoticeOn.value = stored.notifyTrack;
  if (stored?.homeStats !== undefined) showHomeStats.value = stored.homeStats;
  if (stored?.audioVisualizer !== undefined) audioOn.value = stored.audioVisualizer;
  if (stored?.weather !== undefined) weatherOn.value = stored.weather;
  if (stored?.weatherCity !== undefined) weatherCity.value = stored.weatherCity;
  if (stored?.speedtest !== undefined) speedtestOn.value = stored.speedtest;
  if (stored?.skipSeconds) skipSeconds.value = stored.skipSeconds;
  if (stored?.position) alignment.value = stored.position;
  if (stored?.collapsedStyle) collapsedStyle.value = stored.collapsedStyle;
  if (stored?.islandStyle) islandStyle.value = stored.islandStyle;

  // El primer intento de redimensionar ocurre antes de que la API de Tauri
  // esté cargada, así que se repite aquí ya con el módulo disponible.
  await invokeWindow("island_resize", targetSize.value);
  refreshVolume();

  // Los ajustes pueden cambiar mientras la isla está abierta.
  try {
    const { listen } = await import("@tauri-apps/api/event");
    // Rust avisa cuando el cursor está en la franja de encima de la isla: esa franja
    // queda fuera de la ventana y la página no ve el ratón sobre ella. Mientras esté
    // ahí, la isla no se recoge; al salir de ella sin volver a la isla, sí.
    await listen<{ r: number; g: number; b: number }>("island://backdrop", (event) => {
      ring.value = `rgb(${event.payload.r}, ${event.payload.g}, ${event.payload.b})`;
    });
    await listen<{ inside: boolean }>("island://gap", (event) => {
      inGap = event.payload.inside;
      if (inGap) {
        clearHoverTimer();
        // Con la isla cerrada y flotando, la franja de encima también abre al acercar el ratón: es parte de
        // la isla, y si no, subir hasta el borde no haría nada.
        if (!expanded.value && hoverOpen.value && !notice.value) {
          hoverTimer = window.setTimeout(() => {
            if (notice.value || expanded.value) return;
            openedByHover = true;
            expanded.value = true;
          }, 140);
        }
      } else if (!overIsland) collapseIfHoverOpened();
    });
    await listen<{
      skipSeconds?: number;
      position?: Alignment;
      collapsedStyle?: CollapsedStyle;
      islandStyle?: IslandStyle;
      homeSeconds?: boolean;
      homeStats?: boolean;
      audioVisualizer?: boolean;
      weather?: boolean;
      weatherCity?: string;
      speedtest?: boolean;
      hoverOpen?: boolean;
      notifications?: boolean;
      notifyTrack?: boolean;
    }>("settings://changed", (event) => {
      if (event.payload?.hoverOpen !== undefined) hoverOpen.value = event.payload.hoverOpen;
      if (event.payload?.notifications !== undefined) notificationsOn.value = event.payload.notifications;
      if (event.payload?.notifyTrack !== undefined) trackNoticeOn.value = event.payload.notifyTrack;
      if (event.payload?.homeStats !== undefined) showHomeStats.value = event.payload.homeStats;
      if (event.payload?.audioVisualizer !== undefined) audioOn.value = event.payload.audioVisualizer;
      if (event.payload?.weather !== undefined) weatherOn.value = event.payload.weather;
      if (event.payload?.weatherCity !== undefined) weatherCity.value = event.payload.weatherCity;
      if (event.payload?.speedtest !== undefined) speedtestOn.value = event.payload.speedtest;
      if (event.payload?.homeSeconds !== undefined) homeSeconds.value = event.payload.homeSeconds;
      if (event.payload?.skipSeconds) skipSeconds.value = event.payload.skipSeconds;
      if (event.payload?.position) alignment.value = event.payload.position;
      if (event.payload?.collapsedStyle) collapsedStyle.value = event.payload.collapsedStyle;
      if (event.payload?.islandStyle) islandStyle.value = event.payload.islandStyle;
    });
  } catch {
    // Fuera de Tauri no hay eventos que escuchar.
  }
});


/**
 * Adelanta o retrocede la reproducción los segundos indicados.
 *
 * Algunos reproductores (Apple Music) no admiten cambiar la posición por
 * SMTC: en ese caso los botones aparecen deshabilitados.
 */
function skipBy(seconds: number) {
  const { position, duration, canSeek } = media.state.value;
  // Los botones ya se deshabilitan cuando no se puede: esto solo protege
  // la llamada por teclado.
  if (!canSeek || duration <= 0) return;
  const target = Math.min(duration, Math.max(0, position + seconds));
  media.seek(target);
}


/** Salta a la proporcion de la pista indicada por el arrastre (0 a 1). */
function scrubTo(ratio: number) {
  const { duration, canSeek } = media.state.value;
  if (!canSeek || duration <= 0) return;
  media.seek(ratio * duration);
}
</script>

<template>
  <!-- `island-stage` no tiene estilos propios: es el asa de los selectores de la
       isla de más abajo. La ventana tiene tamaño fijo y Rust recorta la zona
       visible, así que la isla crece hacia el centro sin depender del viewport.
       Sin relleno superior: cuelga del borde de la pantalla y su primer píxel es
       también el primero de la ventana. -->
  <div class="island-stage box-border grid h-screen w-screen items-start pt-0" :class="stageAlignment">
    <div class="island-wrap" :class="{ 'expanded-round': islandStyle === 'round', 'collapsed-round': islandStyle === 'round' }" :style="{ '--morph': `${morphMs}ms`, '--art-accent': artAccent || undefined, '--island-ring': ring }">
      <!-- Tinte de la carátula sobre cada muesca. El relleno base de las
           muescas es negro, y sin esto quedan como dos parches oscuros a los
           lados de una isla coloreada. -->
      <!-- Cambian con el mismo fundido que el fondo de la píldora: si no, las
           muescas saltarían de color mientras el resto se funde. -->
      <Transition name="art-fade">
        <div
          v-if="trackArtwork"
          :key="`l${trackArtwork}`"
          class="notch-art notch-art-left"
          :style="{ '--art': `url(${trackArtwork})` }"
          aria-hidden="true"
        ></div>
      </Transition>
      <Transition name="art-fade">
        <div
          v-if="trackArtwork"
          :key="`r${trackArtwork}`"
          class="notch-art notch-art-right"
          :style="{ '--art': `url(${trackArtwork})` }"
          aria-hidden="true"
        ></div>
      </Transition>
    <TheIsland
      :mode="displayMode"
      :notice="notice"
      :collapsed-style="collapsedStyle"
      :expanded="expanded"
      :playing="playing"
      :recording="false"
      :recording-seconds="0"
      :timer-running="false"
      timer-label="00:00"
      call-label="00:00"
      :call-active="false"
      :muted="muted"
      :volume="volume"
      :track-title="trackTitle"
      :track-subtitle="trackSubtitle"
      :track-initial="trackInitial"
      :track-artwork="trackArtwork"
      :position-label="positionLabel"
      :duration-label="durationLabel"
      :remaining-label="remainingLabel"
      :track-progress="trackProgress"
      :can-seek="canSeek"
      :skip-seconds="skipSeconds"
      :home-seconds="homeSeconds"
      :home-stats="homeStats"
      :privacy="privacy"
      :downloading="downloading"
      :weather="weather"
      :speedtest-enabled="speedtestOn"
      :system-enabled="showHomeStats"
      :speed="speed"
      v-model:home-page="homePage"
      :carousel="true"
      :has-media="hasMedia"
      :lyrics-available="lyricsAvailable"
      :lyrics-open="lyricsOpen"
      :lyric-lines="lyricLines"
      :format-time="formatTime"
      @island-click="handleIslandClick"
      @island-keydown="handleIslandKeydown"
      @island-enter="handleIslandEnter"
      @island-leave="handleIslandLeave"
      @speedtest="startSpeedtest"
      @choose="mode = $event"
      @toggle-play="media.togglePlay()"
      @next="media.next()"
      @previous="media.previous()"
      @scrub="scrubTo"
      @toggle-mute="toggleMute"
      @volume="setVolume"
      @toggle-lyrics="lyricsOpen = !lyricsOpen"
      @skip="skipBy"
    />
    </div>
  </div>
</template>

<style>
/* La ventana es transparente: solo debe verse la isla, nunca un recuadro.
   Se fuerza sobre el fondo global que define style.css para la demo. */
:root,
html,
body,
#app {
  margin: 0;
  min-height: 0;
  overflow: hidden;
  background: transparent !important;
}

/* En la ventana flotante la isla se centra sola, sin posicionamiento absoluto.
   Se anula la animación de entrada de la demo: anima `margin-top` desde -8px
   y deja el contenido descentrado dentro de la píldora. */
.island-stage .island {
  position: relative;
  animation: none;
  margin-top: 0;
  top: auto;
  left: auto;
  transform: none;
  /* Entre el relleno de las muescas (z-index 1) y su tinte (3): la píldora
     tapa el negro que asoma por el costado, y el tinte se dibuja por encima
     de ambos para cubrir la curva entera. */
  z-index: 2;
  /* Las esquinas de arriba quedan rectas: de ellas nacen las muescas. En la
     demo del navegador la isla flota sobre un escritorio simulado y conserva
     sus cuatro esquinas redondeadas, por eso esto vive aquí y no en las
     clases compartidas. Hay que ganar a las utilidades `rounded-[…]`. */
  border-top-left-radius: 0 !important;
  border-top-right-radius: 0 !important;
  /* Sin contorno: las muescas son elementos aparte y no pueden llevarlo, así
     que la línea de la píldora se cortaría en seco al llegar a ellas y
     dejaría dos trazos sueltos. Contra el negro del marco no aporta nada. */
  border: none !important;
}

/* --- Esquinas invertidas --- */
/* La isla cuelga del borde superior y sus esquinas de arriba se abren HACIA
   AFUERA: en vez de morder la píldora, añaden material a cada lado que se
   funde con el canto de la pantalla. `border-radius` no sirve, porque
   siempre curva hacia dentro.

   Cada muesca es un cuadrado pegado al costado, por fuera de la isla, al que
   una máscara le quita el cuarto de círculo que mira hacia ella. Lo que
   queda es el triángulo curvo que une el borde superior con el lateral. */
.island-wrap {
  --notch: 14px;
  /* Tamaño de la píldora, para encuadrar el tinte de las muescas igual que
     ella. Coincide con las clases `w-[…] h-[…]` de TheIsland.vue. */
  --island-width: 116px;
  --island-height: 34px;
  position: relative;
  display: inline-block;
}

.island-wrap::before,
.island-wrap::after {
  content: "";
  position: absolute;
  top: 0;
  width: var(--notch);
  height: var(--notch);
  background: #050506;
  pointer-events: none;
  /* Bajo la píldora: así el solape no tapa su contenido. */
  z-index: 1;
  /* Acompañan el redimensionado de la píldora en vez de saltar al valor
     final: `--notch` cambia de golpe al entrar `.is-expanded`, y sin esta
     transición las muescas parpadean a su tamaño nuevo mientras la isla
     todavía se está moviendo. Misma curva y duración que `.island-morph`. */
  transition: width var(--morph, 0.58s), height var(--morph, 0.58s), left var(--morph, 0.58s), right var(--morph, 0.58s);
  /* Misma curva sin rebote que `.island-morph`. */
  transition-timing-function: cubic-bezier(.22, 1, .36, 1);
}

.island-wrap::before {
  left: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
}

.island-wrap::after {
  right: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
}

/* Tinte de la carátula sobre las muescas, con la misma receta que
   `.art-backdrop` aplica a la píldora: misma imagen, mismo desenfoque, misma
   capa oscura y misma opacidad. Así el color sigue la regla de la isla en vez
   de aproximarse con un tono fijo.

   Va en `z-index: 3`, por encima de `.island-wrap::before/::after`: esos
   pintan el negro base y, al venir después en el árbol con el mismo nivel,
   taparían el tinte —el de la derecha en particular, que es `::after`. */
.notch-art {
  position: absolute;
  top: 0;
  width: var(--notch);
  height: var(--notch);
  z-index: 3;
  overflow: hidden;
  opacity: 0.4;
  pointer-events: none;
  /* En sincronía con la muesca de debajo: si el relleno negro se redimensiona
     animado y el tinte salta, asoma el negro por los bordes a media
     transición. La opacidad también cambia entre estados (0.4 ↔ 0.22). */
  transition: width var(--morph, 0.58s), height var(--morph, 0.58s), left var(--morph, 0.58s), right var(--morph, 0.58s), opacity var(--morph, 0.58s);
  /* Misma curva sin rebote que `.island-morph`. */
  transition-timing-function: cubic-bezier(.22, 1, .36, 1);
}

.notch-art-left {
  left: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 0 100%, transparent 99%, #000 100%);
}

.notch-art-right {
  right: calc(var(--notch) * -1);
  -webkit-mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
  mask: radial-gradient(circle var(--notch) at 100% 100%, transparent 99%, #000 100%);
}

/* La carátula se encuadra como en la píldora —misma escala y mismo centro—
   para que el tinte continúe sin salto al cruzar la juntura. */
.notch-art::before {
  content: "";
  position: absolute;
  width: calc(var(--island-width) * 1.6);
  height: calc(var(--island-height) * 1.6);
  top: calc(var(--island-height) * -0.3);
  background-image: var(--art);
  background-size: cover;
  background-position: center;
  filter: blur(20px) saturate(1.6);
}

.notch-art-left::before {
  left: calc(var(--island-width) * -0.3 + var(--notch));
}

.notch-art-right::before {
  right: calc(var(--island-width) * -0.3 + var(--notch));
}

.notch-art::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(5, 5, 6, 0.35), rgba(5, 5, 6, 0.75));
  background-size: 100% var(--island-height);
}

/* Compacta, la píldora atenúa su tinte; las muescas siguen la misma regla. */
.island-wrap:not(:has(.island.is-expanded)) .notch-art {
  opacity: 0.22;
}

.island-wrap:not(:has(.island.is-expanded)) .notch-art::after {
  background: linear-gradient(180deg, rgba(5, 5, 6, 0.6), rgba(5, 5, 6, 0.8));
}

/* Expandida la isla es mucho mayor: la muesca crece para que la transición
   entre el marco y el costado siga leyéndose igual. */
.island-wrap:has(.island.is-expanded) {
  --notch: 22px;
  --island-width: 330px;
  --island-height: 108px;
}

/* La canción presentada mide 54 px de alto: la muesca (22 px) es la del panel, que
   es lo que Rust usa para cualquier altura de más de 40 px. */
.island-wrap:has(.island.is-track) {
  --island-height: 54px;
}

/* Vista expandida redondeada: sin muescas y con las esquinas de arriba curvas, como
   las de abajo. `--notch` a 0 las encoge con la misma transición que las crea, así
   que no desaparecen de golpe; sus tintes de carátula usan la misma variable. Lo
   replica `Shape::for_size` en lib.rs (notch 0, top_radius 24): si cambias una
   medida, cambia la otra. Va después de `.is-track` para ganarle en el orden. */
.island-wrap.expanded-round:has(.island.is-expanded) {
  --notch: 0px;
}
.island-wrap.expanded-round .island.is-expanded {
  border-top-left-radius: 18px !important;
  border-top-right-radius: 18px !important;
}
/* Y flota: 6 px de separación con el borde de la pantalla (`ROUND_GAP` en lib.rs).
   Se anima con el tamaño, así que la isla baja mientras se abre y sube al cerrarse. */
.island-wrap {
  transition: margin-top var(--morph, 0.58s) cubic-bezier(.22, 1, .36, 1);
}
.island-wrap.expanded-round:has(.island.is-expanded) {
  margin-top: 6px;
}

/* Vista minimizada redondeada: sin muescas, las cuatro esquinas curvas (radio de media altura: 17 px) y flotando
   6 px por debajo del borde, como la expandida redondeada. La pestaña mínima solo pierde las muescas y sigue
   pegada. Lo replica `Shape::for_size` en lib.rs (notch 0, radio 17, top_radius 17, top_offset 6): si cambias
   una medida, cambia la otra. Son los mismos selectores que los de la expandida, pero para la isla sin abrir. */
.island-wrap.collapsed-round:not(:has(.island.is-expanded)) {
  --notch: 0px;
}
.island-wrap.collapsed-round .island:not(.is-expanded):not(.is-tab) {
  border-radius: 17px !important;
}
.island-wrap.collapsed-round:not(:has(.island.is-expanded)):not(:has(.island.is-tab)) {
  margin-top: 6px;
}

/* Anillo del estilo flotante: 2 px opacos del color del escritorio alrededor de la isla (`--island-ring`, muestreado en
   Rust). El recorte nativo sube 1 px (`g` en `set_island_region`), así que su borde duro cae sobre el anillo y no se
   ve, y el suavizado del borde de la isla se hace contra el anillo, un color opaco, en lugar de contra el blanco que
   hay detrás de los píxeles a medias. Sustituye a la sombra de caída, que el recorte ya cortaba. La pestaña mínima, pegada
   al borde, no lo lleva. */
.island-wrap.collapsed-round .island:not(.is-tab) {
  box-shadow: 0 0 0 2px var(--island-ring) !important;
}

/* Pestaña mínima: la muesca se reduce a 5 px, porque una de 14 px colgaría por
   debajo de una isla de 8 px de alto. Misma medida que `Shape::for_size` en
   lib.rs; si cambias una, cambia la otra. */
.island-wrap:has(.island.is-tab) {
  --notch: 5px;
  --island-width: 48px;
  --island-height: 8px;
}

/* La sombra grande de la isla sobraría en una pestaña de 8 px: se sustituye
   por un filo claro en el borde inferior, para que se distinga también sobre
   ventanas oscuras. */
.island-stage .island.is-tab {
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35), inset 0 -1px 0 rgba(255, 255, 255, 0.18);
}

/* Con el panel de karaoke la píldora se alarga a 540 px; sin esto el tinte
   de las muescas seguiría encuadrado para 330 y mostraría otro trozo de la
   carátula que la isla. */
.island-wrap:has(.island.is-expanded.has-lyrics) {
  --island-width: 540px;
}

/* Colgando del borde, la sombra solo tiene sentido hacia abajo y a los lados;
   hacia arriba caería fuera de la pantalla. */
.island-stage .island,
.island-stage .island.is-expanded {
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
}
</style>
