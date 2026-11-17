/**
 * Eventos del sistema que la isla anuncia como avisos: cambios de volumen y
 * batería baja. El backend solo emite cuando el ajuste `notifications` está
 * activo, así que aquí no se vuelve a comprobar.
 */
import { onUnmounted } from "vue";
import { isDesktopApp } from "@/lib/media";
import type { PowerState } from "@/lib/power";

/** Refleja `VolumeState` de `volume.rs`, que serializa en camelCase. */
export interface VolumeChange {
  level: number;
  muted: boolean;
}

/**
 * Escucha un evento de Tauri durante la vida del componente.
 *
 * `onUnmounted` se registra de forma síncrona, antes de cualquier `await`:
 * Vue solo lo admite dentro de `setup`. La baja se guarda para llamarla cuando
 * la suscripción asíncrona termine, aunque el componente ya se haya desmontado.
 */
function listenWhileMounted<T>(event: string, handler: (payload: T) => void) {
  if (!isDesktopApp()) return;

  let unlisten: (() => void) | undefined;
  let unmounted = false;
  onUnmounted(() => {
    unmounted = true;
    unlisten?.();
  });

  void (async () => {
    try {
      const { listen } = await import("@tauri-apps/api/event");
      const stop = await listen<T>(event, (e) => {
        if (e.payload) handler(e.payload);
      });
      if (unmounted) stop();
      else unlisten = stop;
    } catch {
      // Fuera de la app de escritorio no hay eventos que escuchar.
    }
  })();
}

/** Avisa cuando el volumen o el silencio cambian desde el sistema. */
export function useVolumeEvents(onChange: (change: VolumeChange) => void) {
  listenWhileMounted<VolumeChange>("volume://changed", onChange);
}

/** Refleja `AppToast` de `toasts.rs`, que serializa en camelCase. */
export interface AppToast {
  id: number;
  app: string;
  title: string;
  body: string;
  /** Foto de perfil del remitente (data URI), si el ajuste está activo y el aviso la trae. */
  avatar?: string | null;
  /** Logo de la aplicación (data URI), que la isla usa de avatar si no hay foto del remitente. */
  icon?: string | null;
}

/** Comprime espacios y saltos de línea: en una sola fila no tienen utilidad. */
const squash = (text: string) => text.replace(/\s+/g, " ").trim();

/** Compara sin tildes ni mayúsculas, para detectar repeticiones aunque cambie la forma. */
const normalize = (text: string) =>
  squash(text)
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();

/**
 * Ordena los campos de una notificación para la píldora.
 *
 * Cada aplicación rellena título y cuerpo a su manera, y varias dejan campos
 * redundantes: el título es solo el nombre de la app («Microsoft Teams»), o el
 * cuerpo empieza repitiendo el título. Mostrarlos tal cual gasta una de las
 * pocas filas de la píldora en algo que el usuario ya sabe. Sin tocar el
 * contenido: solo se quitan repeticiones y se reparten los campos.
 */
export function tidyToast(toast: AppToast): { title: string; body: string } {
  let title = squash(toast.title);
  let body = squash(toast.body);

  // El título solo repite la aplicación: el cuerpo pasa a ser el título.
  const app = normalize(toast.app);
  if (title && body && app && (normalize(title) === app || app.includes(normalize(title)))) {
    title = body;
    body = "";
  }

  // El cuerpo empieza con el título («Ana: Ana: hola»): se quita la repetición.
  if (title && body && normalize(body).startsWith(normalize(title))) {
    const rest = body.slice(title.length).replace(/^[\s:·\-–—]+/, "");
    if (rest) body = rest;
  }

  return { title, body };
}

export type ToastMediaKind = "sticker" | "gif" | "photo" | "voice";

/**
 * Tipo de adjunto cuando el mensaje es solo eso.
 *
 * WhatsApp no entrega la imagen del sticker ni del GIF, ni el audio, en el aviso
 * (solo la foto de perfil): llega el texto «Sticker», «GIF», «Foto» o «Mensaje de
 * voz (0:08)», y es lo único que se puede reconocer. En un grupo puede ir
 * precedido del remitente («Ana: Sticker»). `detail` es la duración de un audio.
 */
export function toastMediaKind(text: string): { kind: ToastMediaKind; detail: string } | null {
  const m = normalize(text).match(
    /^(?:[^:d]{1,40}:s*)?(?:[^p{L}s]+s*)?(sticker|gif|foto|photo|imagen|image|mensaje de voz|nota de voz|voice message|audio)(?:s*((d{1,2}(?::d{2}){1,2})))?$/u,
  );
  if (!m) return null;
  const word = m[1];
  const kind: ToastMediaKind =
    word === "sticker" ? "sticker" : word === "gif" ? "gif" : /voz|voice|audio/.test(word) ? "voice" : "photo";
  return { kind, detail: m[2] ?? "" };
}

/** Avisa cuando una aplicación activada en ajustes envía una notificación. */
export function useToastEvents(onToast: (toast: AppToast) => void) {
  listenWhileMounted<AppToast>("toast://received", onToast);
}

/** Avisa cuando la batería cruza un umbral sin estar enchufado. */
export function useLowBatteryEvents(onLow: (state: PowerState) => void) {
  listenWhileMounted<PowerState>("power://low", onLow);
}

/** Refleja `WifiState` de `network.rs`, que serializa en camelCase. */
export interface WifiConnection {
  ssid: string | null;
  signal: number;
}

/** Avisa cuando el equipo se conecta a una red Wi-Fi o cambia de red. */
export function useWifiEvents(onConnect: (wifi: WifiConnection) => void) {
  listenWhileMounted<WifiConnection>("network://wifi", onConnect);
}

/** Refleja `BluetoothChange` de `bluetooth.rs`. */
export interface BluetoothChange {
  name: string;
  connected: boolean;
}

/** Avisa cuando un dispositivo Bluetooth se conecta o se desconecta. */
export function useBluetoothEvents(onChange: (change: BluetoothChange) => void) {
  listenWhileMounted<BluetoothChange>("bluetooth://changed", onChange);
}

/** Refleja `UsbChange` de `usb.rs`. */
export interface UsbChange {
  name: string;
  letter: string;
  connected: boolean;
}

/** Avisa cuando una unidad extraíble (memoria USB, tarjeta SD) se conecta o se retira. */
export function useUsbEvents(onChange: (change: UsbChange) => void) {
  listenWhileMounted<UsbChange>("usb://changed", onChange);
}

/** Avisa cuando Bloq Mayús o Bloq Num cambian de estado. */
export function useLockKeyEvents(onChange: (change: { key: "caps" | "num"; on: boolean }) => void) {
  listenWhileMounted<{ key: "caps" | "num"; on: boolean }>("keys://lock", onChange);
}

/** Avisa cuando se activa o apaga el modo No molestar. */
export function useFocusEvents(onChange: (on: boolean) => void) {
  listenWhileMounted<{ on: boolean }>("focus://changed", (state) => onChange(state.on));
}

/** Avisa cuando cambia el brillo de la pantalla integrada (0–100). */
export function useBrightnessEvents(onChange: (level: number) => void) {
  listenWhileMounted<{ level: number }>("brightness://changed", (state) => onChange(state.level));
}

/** Refleja `PrivacyState` de `privacy.rs`: la aplicación que usa cada dispositivo, o null. */
export interface PrivacyUse {
  microphone: string | null;
  camera: string | null;
}

/** Avisa cuando cambia qué aplicación usa el micrófono o la cámara. */
export function usePrivacyEvents(onChange: (state: PrivacyUse) => void) {
  listenWhileMounted<PrivacyUse>("privacy://changed", onChange);
}

/** Niveles de las cinco bandas del audio que suena (0–1), de grave a agudo. */
export function useAudioLevels(onLevels: (levels: number[]) => void) {
  listenWhileMounted<number[]>("audio://levels", onLevels);
}

/** Refleja `DownloadProgress` y `DownloadDone` de `downloads.rs`. */
export interface DownloadProgress {
  active: boolean;
  count: number;
  bytes: number;
  speed: number;
}
export interface DownloadDone {
  name: string;
  size: number;
}

/** Avisa del avance de las descargas del navegador y de cuándo termina una. */
export function useDownloadEvents(onProgress: (p: DownloadProgress) => void, onDone: (d: DownloadDone) => void) {
  listenWhileMounted<DownloadProgress>("downloads://progress", onProgress);
  listenWhileMounted<DownloadDone>("downloads://done", onDone);
}

/** Refleja `Progress` y `SpeedResult` de `speedtest.rs`. */
export interface SpeedProgress {
  phase: "ping" | "down";
  mbps: number;
}
export interface SpeedResult {
  down: number | null;
  ping: number | null;
  /** `limit`: el servicio limitó las pruebas seguidas; `network`: sin conexión; null: se midió. */
  error: "limit" | "network" | null;
}

/** Avisa del avance y del resultado del test de velocidad. */
export function useSpeedtestEvents(onProgress: (p: SpeedProgress) => void, onDone: (r: SpeedResult) => void) {
  listenWhileMounted<SpeedProgress>("speedtest://progress", onProgress);
  listenWhileMounted<SpeedResult>("speedtest://done", onDone);
}

/** Avisa cuando el equipo pierde o recupera la salida a internet. */
export function useInternetEvents(onChange: (online: boolean) => void) {
  listenWhileMounted<{ online: boolean }>("network://internet", (state) => onChange(state.online));
}
