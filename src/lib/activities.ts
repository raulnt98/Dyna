/**
 * Actividades que la isla puede mostrar.
 *
 * `home` no se elige: es la vista de inicio que sustituye al reproductor
 * cuando no hay ningún medio (ver `displayMode` en IslandApp.vue).
 */
export type Activity =
  | "home"
  | "idle"
  | "ring"
  | "timer"
  | "record"
  | "music"
  | "airdrop"
  | "call"
  | "message";

/** Fondos disponibles en el escritorio simulado de la demo. */
export type Wallpaper = "aurora" | "dusk" | "blue";
