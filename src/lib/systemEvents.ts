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

/** Avisa cuando el equipo pierde o recupera la salida a internet. */
export function useInternetEvents(onChange: (online: boolean) => void) {
  listenWhileMounted<{ online: boolean }>("network://internet", (state) => onChange(state.online));
}
