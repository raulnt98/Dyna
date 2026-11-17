/**
 * Cola de avisos de la isla.
 *
 * Un aviso interrumpe lo que la isla esté mostrando, vive unos segundos y se
 * retira solo, dejando la isla como estaba. La cola existe porque dos sucesos
 * pueden coincidir: sin ella el segundo pisaría al primero a media animación,
 * que es justo el salto brusco que la isla debe evitar.
 */
import { computed, onUnmounted, ref } from "vue";

/** Tipos de aviso que la isla sabe dibujar. */
export type NoticeKind = "power" | "volume" | "battery" | "toast" | "wifi" | "internet" | "bluetooth" | "bluetooth-off" | "usb" | "caps" | "num" | "focus" | "brightness" | "track" | "mic" | "camera" | "download" | "speed";

export interface Notice {
  /** Identifica al emisor: dos avisos del mismo tipo se reemplazan, no se encolan. */
  kind: NoticeKind;
  /** Texto corto de la píldora compacta. Nada largo: hay sitio para unos caracteres. */
  label: string;
  /** Milisegundos en pantalla antes de retirarse solo. */
  duration: number;
  /** Datos propios del aviso, para que la vista dibuje lo suyo. */
  data?: Record<string, unknown>;
  /**
   * El aviso se encola aunque haya otro del mismo tipo, y no se acorta cuando
   * llega el siguiente. Para mensajes: dos mensajes seguidos son dos mensajes,
   * no el mismo aviso actualizado, a diferencia del volumen o el cargador.
   */
  stack?: boolean;
}

/** Máximo de avisos apilados esperando turno; los más antiguos se descartan. */
const MAX_STACKED = 5;

/**
 * Tiempo mínimo que un aviso permanece visible aunque llegue otro detrás.
 *
 * Un aviso tarda ~0.4 s en abrirse y ~0.34 s en cerrarse: relevarlo antes de
 * eso lo dejaría a medio dibujar. El mínimo deja además un rato para leerlo.
 */
const MIN_VISIBLE_MS = 1200;

/**
 * Ancho que necesita la píldora para un aviso, en píxeles.
 *
 * La ventana nativa se redimensiona desde Rust antes de que el CSS pinte, así
 * que no se puede medir el texto ya dibujado: hay que estimarlo. Los 76 px
 * cubren el relleno (12 por lado), el icono (20) y el hueco (10), y cada
 * carácter de la fuente a 12 px y peso 600 ocupa algo menos de 7 px.
 */
export function noticeWidth(label: string): number {
  const estimated = 76 + label.length * 6.9;
  // El mínimo iguala a la píldora compacta, para que un aviso corto no la
  // encoja; el máximo evita que un texto inesperado se coma la pantalla.
  return Math.round(Math.min(260, Math.max(116, estimated)));
}

/**
 * Ancho de la píldora cerrada de la vista de inicio (sin música): la hora sola, o la hora, una raya y el tiempo
 * («12:35 | ☀ 24°»). Como `noticeWidth`, **se estima**: la ventana nativa se redimensiona desde Rust antes de que el CSS
 * pinte, y la isla (`TheIsland.vue`) y su ventana (`IslandApp.vue`) tienen que usar el mismo número.
 *
 * `seconds` dice si la hora lleva segundos (8 caracteres en lugar de 5) y `temp` es el texto de la temperatura
 * («24°») o `null` si no hay tiempo. A 12 px y peso 650 una cifra ocupa unos 7,4 px.
 */
export function homePillWidth(seconds: boolean, temp: string | null): number {
  const time = (seconds ? 8 : 5) * 7.4;
  // Hueco + raya + hueco + icono + hueco + temperatura.
  const weather = temp ? 10 + 1 + 10 + 14 + 5 + temp.length * 7.4 : 0;
  return Math.round(Math.max(116, 28 + time + weather));
}

export function useNotifications() {
  /** Aviso en pantalla, o `null` si la isla muestra su contenido normal. */
  const current = ref<Notice | null>(null);
  /** Avisos esperando turno, en orden de llegada. */
  const queue = ref<Notice[]>([]);

  let hideTimer: number | undefined;
  let shownAt = 0;

  const clearTimer = () => {
    if (hideTimer !== undefined) {
      window.clearTimeout(hideTimer);
      hideTimer = undefined;
    }
  };

  /** Muestra el siguiente de la cola, o deja la isla libre si no queda ninguno. */
  const advance = () => {
    clearTimer();
    const next = queue.value.shift() ?? null;
    current.value = next;
    if (!next) return;

    shownAt = Date.now();
    hideTimer = window.setTimeout(advance, next.duration);
  };

  /**
   * Encola un aviso, o lo muestra si la isla está libre.
   *
   * Un aviso del mismo tipo reemplaza al anterior en vez de acumularse: al
   * enchufar y desenchufar seguido interesa el estado final, no la secuencia.
   */
  const notify = (notice: Notice) => {
    if (!notice.stack) {
      queue.value = queue.value.filter((item) => item.kind !== notice.kind);
    }

    if (!current.value) {
      queue.value.push(notice);
      advance();
      return;
    }

    if (!notice.stack && current.value.kind === notice.kind) {
      // Mismo emisor: se actualiza en el sitio y se reinicia la cuenta, para
      // que el texto nuevo no herede los milisegundos que le quedaban al viejo.
      current.value = notice;
      clearTimer();
      shownAt = Date.now();
      hideTimer = window.setTimeout(advance, notice.duration);
      return;
    }

    queue.value.push(notice);
    // Con una avalancha de mensajes solo interesan los últimos.
    const stacked = queue.value.filter((item) => item.stack);
    if (stacked.length > MAX_STACKED) {
      queue.value = queue.value.filter((item) => item !== stacked[0]);
    }

    // Un mensaje en pantalla se lee entero: no se acorta por el siguiente.
    if (current.value.stack) return;

    // El aviso en curso conserva su tiempo mínimo en pantalla; pasado ese
    // umbral cede el turno enseguida para que el siguiente no espere de más.
    const shownFor = Date.now() - shownAt;
    if (shownFor >= MIN_VISIBLE_MS) advance();
  };

  /** Retira el aviso en curso, por ejemplo si el usuario toca la isla. */
  const dismiss = () => advance();

  onUnmounted(clearTimer);

  return {
    current: computed(() => current.value),
    notify,
    dismiss,
  };
}
