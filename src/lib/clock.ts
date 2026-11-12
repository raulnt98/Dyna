/**
 * Reloj de reproducción suave para la letra.
 *
 * El backend envía la posición como mucho cada 250 ms, y entre un envío y otro
 * el valor se queda quieto: decidir la línea activa con él hace que se retrase
 * hasta un cuarto de segundo y que cambie a saltos. Aquí se avanza la posición
 * con el reloj del navegador entre mensajes, y cada mensaje solo corrige la
 * deriva.
 */
import { onUnmounted, ref, watch, type Ref } from "vue";

interface ClockSource {
  position: number;
  duration: number;
  playing: boolean;
  title: string;
  artist: string;
}

/**
 * Error por encima del cual la posición del sistema manda sin más.
 *
 * Por debajo es ruido de medida (SMTC interpola con su propio reloj), y
 * aplicarlo entero haría que la letra retrocediera o saltara de vez en cuando.
 * Por encima el usuario movió la barra o cambió de pista.
 */
const HARD_RESYNC_SECONDS = 1;

/** Parte del error que se corrige en cada mensaje cuando es solo ruido. */
const SOFT_CORRECTION = 0.25;

/**
 * Posición de la pista avanzada a cada fotograma mientras `active` sea cierto.
 *
 * Con `active` falso no hay bucle de animación y se devuelve la posición del
 * sistema tal cual: el reloj solo cuesta mientras la letra está a la vista.
 */
export function useSmoothPosition(source: () => ClockSource, active: Ref<boolean>) {
  const position = ref(source().position);

  let anchorPosition = source().position;
  let anchorTime = performance.now();
  let trackKey = `${source().title}\u{1}${source().artist}`;
  let frame: number | undefined;

  const predict = (now: number) => {
    const current = source();
    if (!current.playing) return anchorPosition;
    const elapsed = anchorPosition + (now - anchorTime) / 1000;
    // Sin duración conocida no hay tope que aplicar.
    return current.duration > 0 ? Math.min(elapsed, current.duration) : elapsed;
  };

  const tick = () => {
    position.value = predict(performance.now());
    frame = requestAnimationFrame(tick);
  };

  const stop = () => {
    if (frame !== undefined) cancelAnimationFrame(frame);
    frame = undefined;
  };

  const sync = () => {
    stop();
    const current = source();
    if (active.value && current.playing) frame = requestAnimationFrame(tick);
    else position.value = current.position;
  };

  // Cada mensaje del sistema reajusta el ancla.
  watch(
    source,
    (incoming) => {
      const now = performance.now();
      const key = `${incoming.title}\u{1}${incoming.artist}`;
      const error = incoming.position - predict(now);

      if (key !== trackKey || !incoming.playing || Math.abs(error) > HARD_RESYNC_SECONDS) {
        anchorPosition = incoming.position;
      } else {
        anchorPosition = predict(now) + error * SOFT_CORRECTION;
      }
      anchorTime = now;
      trackKey = key;
      sync();
    },
    { deep: false },
  );

  watch(active, sync, { immediate: true });
  onUnmounted(stop);

  return position;
}
