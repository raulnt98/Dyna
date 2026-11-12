/**
 * Letras sincronizadas de la pista en curso.
 *
 * El backend consulta LRCLIB y devuelve las líneas con su marca de tiempo;
 * aquí solo se decide cuál toca resaltar según la posición de reproducción.
 */
import { computed, ref, watch, type Ref } from "vue";

/** Una línea de la letra y el segundo en que empieza. */
export interface LyricLine {
  time: number;
  text: string;
}

interface LyricsPayload {
  lines: LyricLine[];
  resolved: boolean;
}

/**
 * Mantiene la letra de la pista actual y la línea que suena.
 *
 * @param trackKey Identificador de la pista: al cambiar, se busca de nuevo.
 * @param position Segundos reproducidos, para seguir la letra.
 */
export function useLyrics(trackKey: Ref<string>, position: Ref<number>) {
  const lines = ref<LyricLine[]>([]);
  const loading = ref(false);

  /** `true` solo cuando la pista tiene letra sincronizada disponible. */
  const available = computed(() => lines.value.length > 0);

  /**
   * Adelanto aplicado a la letra, en segundos.
   *
   * Las marcas LRC indican cuándo empieza a cantarse la línea, así que
   * resaltarla justo en ese instante se percibe tarde: para cuando el ojo la
   * localiza, la voz ya va por delante. Un pequeño adelanto la deja a tiempo.
   */
  const LEAD_SECONDS = 0.75;

  /** Índice de la línea que corresponde a la posición actual. */
  const activeIndex = computed(() => {
    if (lines.value.length === 0) return -1;
    const now = position.value + LEAD_SECONDS;
    // Búsqueda binaria: se evalúa a cada fotograma y las letras tienen cientos
    // de líneas. Devuelve la última cuya marca ya pasó.
    let low = 0;
    let high = lines.value.length - 1;
    let index = -1;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (lines.value[mid].time <= now) {
        index = mid;
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }
    return index;
  });

  /** Ventana de líneas alrededor de la actual, para mostrar contexto. */
  const visibleLines = computed(() => {
    if (lines.value.length === 0) return [];
    const current = Math.max(0, activeIndex.value);
    const start = Math.max(0, current - 1);
    return lines.value.slice(start, start + 4).map((line, offset) => ({
      ...line,
      active: start + offset === activeIndex.value,
    }));
  });

  // Al cambiar de pista se pide la letra nueva.
  watch(
    trackKey,
    async (key) => {
      lines.value = [];
      if (!key) return;
      loading.value = true;
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const result = await invoke<LyricsPayload>("lyrics_current");
        lines.value = result?.lines ?? [];
      } catch {
        // Sin Tauri o sin red: la isla sigue funcionando sin letra.
        lines.value = [];
      } finally {
        loading.value = false;
      }
    },
    { immediate: true },
  );

  return { lines, available, activeIndex, visibleLines, loading };
}
