/**
 * Puente con la música del sistema (SMTC de Windows) a través de Tauri.
 *
 * Dentro de la app de escritorio la isla refleja lo que realmente suena en
 * Spotify, Edge u otro reproductor compatible. En el navegador no existe esa
 * API, así que se conserva la pista de ejemplo de la demo.
 */
import { ref, readonly, onMounted, onUnmounted } from "vue";

/** Estado que envía el backend en Rust (campos en camelCase). */
export interface MediaState {
  hasSession: boolean;
  title: string;
  artist: string;
  album: string;
  playing: boolean;
  /** Segundos transcurridos de la pista. */
  position: number;
  /** Duración total en segundos; 0 si el reproductor no la informa. */
  duration: number;
  sourceApp: string;
  /** `true` si el reproductor permite saltar a otra posición. */
  canSeek: boolean;
  /** Carátula como data URI; cadena vacía si la pista no tiene. */
  artwork: string;
}

const MEDIA_EVENT = "smtc://media-changed";

/** Pista de ejemplo para la demo web, donde no hay SMTC. */
export const fallbackState: MediaState = {
  hasSession: true,
  title: "Midnight City",
  artist: "M83",
  album: "Hurry Up, We're Dreaming",
  playing: true,
  position: 84,
  duration: 243,
  sourceApp: "",
  canSeek: true,
  artwork: "",
};

/** `true` solo cuando la interfaz corre dentro de la ventana de Tauri. */
export const isDesktopApp = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Composable que mantiene el estado multimedia sincronizado con el sistema.
 *
 * Fuera de Tauri devuelve la pista de ejemplo y deja los controles inertes,
 * de modo que la demo web sigue comportándose como antes.
 */
export function useSystemMedia() {
  const state = ref<MediaState>({ ...fallbackState });
  /** Indica si el estado proviene del sistema o es la pista simulada. */
  const live = ref(false);
  let stopListening: (() => void) | undefined;

  async function playPauseFallback() {
    state.value = { ...state.value, playing: !state.value.playing };
  }

  /** Alterna reproducir/pausar en el reproductor del sistema. */
  let togglePlay = playPauseFallback;
  /** Salta a la pista siguiente. */
  let next = async () => {};
  /** Vuelve a la pista anterior. */
  let previous = async () => {};
  /** Salta a una posición de la pista, en segundos. */
  let seek = async (_seconds: number) => {};

  onMounted(async () => {
    if (!isDesktopApp()) return;

    const { invoke } = await import("@tauri-apps/api/core");
    const { listen } = await import("@tauri-apps/api/event");

    const apply = (incoming: MediaState) => {
      state.value = incoming;
      live.value = incoming.hasSession;
    };

    // Estado inicial, antes de que llegue el primer evento del sistema.
    try {
      apply(await invoke<MediaState>("media_state"));
    } catch {
      // Si el backend no responde, se mantiene la pista de ejemplo.
    }

    const unlisten = await listen<MediaState>(MEDIA_EVENT, (event) => {
      apply(event.payload);
    });
    stopListening = unlisten;

    // Un comando puede tardar en reflejarse en SMTC; se refresca al terminar.
    const run = async (command: string, args?: Record<string, unknown>) => {
      try {
        await invoke<boolean>(command, args);
        apply(await invoke<MediaState>("media_state"));
      } catch {
        // El reproductor rechazó la acción o se cerró: se ignora.
      }
    };

    togglePlay = () => run("media_play_pause");
    next = () => run("media_next");
    previous = () => run("media_previous");
    // El seek no relee el estado en cada llamada: durante un arrastre la
    // lectura llegaría con la posición anterior y la barra daría saltos.
    // El sondeo del backend ya refresca la posición poco después.
    seek = async (seconds: number) => {
      state.value = { ...state.value, position: seconds };
      try {
        await invoke<boolean>("media_seek", { seconds });
      } catch {
        // El reproductor rechazó el salto o se cerró.
      }
    };
  });

  onUnmounted(() => {
    stopListening?.();
  });

  return {
    state: readonly(state),
    live: readonly(live),
    togglePlay: () => togglePlay(),
    next: () => next(),
    previous: () => previous(),
    seek: (seconds: number) => seek(seconds),
  };
}
