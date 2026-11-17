/**
 * Color de acento sacado de una carátula.
 *
 * La onda de sonido y otros detalles toman el tono de la portada para que la isla
 * se vea de una pieza con lo que suena. Se muestrea una versión diminuta de la
 * imagen (32 × 32), se agrupan los píxeles por tono y gana el grupo con más color
 * y brillo, no el más numeroso: el fondo gris u oscuro de muchas portadas ocupa
 * más píxeles pero no es lo que se recuerda de ellas.
 *
 * El resultado se normaliza (saturación y luminosidad) para que se lea bien sobre
 * el negro de la isla: un azul marino o un marrón oscuro tal cual casi no se verían.
 */

/** Color por defecto cuando la portada es casi gris: un blanco suave. */
const NEUTRAL = "hsl(0 0% 88%)";

const BUCKETS = 12;

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0, l];
  const s = d / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (max === rn) h = ((gn - bn) / d) % 6;
  else if (max === gn) h = (bn - rn) / d + 2;
  else h = (rn - gn) / d + 4;
  return [(h * 60 + 360) % 360, s, l];
}

/** Tono de acento de la portada como `hsl(...)`, o `null` si no se pudo leer. */
export function accentFromArtwork(src: string): Promise<string | null> {
  return new Promise((resolve) => {
    const image = new Image();
    image.onerror = () => resolve(null);
    image.onload = () => {
      try {
        const size = 32;
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (!ctx) return resolve(null);
        ctx.drawImage(image, 0, 0, size, size);
        const { data } = ctx.getImageData(0, 0, size, size);

        // Por grupo de tono: peso total y suma de sus componentes.
        const weight = new Array<number>(BUCKETS).fill(0);
        const sum = Array.from({ length: BUCKETS }, () => [0, 0, 0]);
        let colorful = 0;

        for (let i = 0; i < data.length; i += 4) {
          if (data[i + 3] < 200) continue;
          const [h, s, l] = rgbToHsl(data[i], data[i + 1], data[i + 2]);
          // Los casi negros, casi blancos y casi grises no aportan tono.
          if (l < 0.1 || l > 0.93 || s < 0.18) continue;
          colorful++;
          const bucket = Math.floor(h / (360 / BUCKETS)) % BUCKETS;
          // Pesa más lo saturado y lo que no es ni muy oscuro ni muy claro.
          const w = s * (1 - Math.abs(l - 0.5) * 1.2);
          weight[bucket] += w;
          sum[bucket][0] += data[i] * w;
          sum[bucket][1] += data[i + 1] * w;
          sum[bucket][2] += data[i + 2] * w;
        }

        // Una portada casi en blanco y negro no tiene un tono que elegir.
        if (colorful < 24) return resolve(NEUTRAL);

        let best = 0;
        for (let b = 1; b < BUCKETS; b++) if (weight[b] > weight[best]) best = b;
        if (weight[best] <= 0) return resolve(NEUTRAL);

        const r = sum[best][0] / weight[best];
        const g = sum[best][1] / weight[best];
        const b = sum[best][2] / weight[best];
        const [h, s] = rgbToHsl(r, g, b);
        // Sobre negro: saturación viva y luminosidad media-alta, sea cual sea la portada.
        const saturation = Math.min(0.9, Math.max(0.55, s));
        resolve(`hsl(${Math.round(h)} ${Math.round(saturation * 100)}% 68%)`);
      } catch {
        resolve(null);
      }
    };
    image.src = src;
  });
}
