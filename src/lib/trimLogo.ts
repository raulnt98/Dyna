/**
 * Recorta el margen transparente de un logo (data URI) y lo deja cuadrado.
 *
 * El logo que entrega Windows para una app empaquetada viene dentro de un lienzo con mucho margen vacío (el
 * dibujo ocupa a veces la mitad), así que en una insignia pequeña se veía como un punto. Aquí se mide el
 * rectángulo con algo dibujado (alfa visible) y se recorta a él. Va con caché por logo: el mismo se pide en
 * cada aviso de esa app. Si algo falla (sin canvas, imagen ilegible, lienzo vacío) devuelve el original.
 */
const cache = new Map<string, Promise<string>>();

/** Alfa por debajo del cual un píxel cuenta como vacío (el suavizado de bordes y las sombras casi invisibles). */
const MIN_ALPHA = 24;
/** Margen que se deja alrededor del dibujo, como fracción de su lado. */
const BREATHING = 0.06;
const OUT_SIZE = 128;

export function trimLogo(src: string): Promise<string> {
  let result = cache.get(src);
  if (!result) {
    result = trim(src).catch(() => src);
    cache.set(src, result);
  }
  return result;
}

async function trim(src: string): Promise<string> {
  const image = new Image();
  image.src = src;
  await image.decode();
  const w = image.naturalWidth;
  const h = image.naturalHeight;
  if (!w || !h) return src;

  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return src;
  ctx.drawImage(image, 0, 0);
  const { data } = ctx.getImageData(0, 0, w, h);

  let minX = w, minY = h, maxX = -1, maxY = -1;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (data[(y * w + x) * 4 + 3] >= MIN_ALPHA) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
    }
  }
  if (maxX < 0) return src;

  // Cuadrado centrado en el dibujo, con un poco de aire.
  const side = Math.max(maxX - minX + 1, maxY - minY + 1) * (1 + BREATHING * 2);
  const cx = (minX + maxX + 1) / 2;
  const cy = (minY + maxY + 1) / 2;

  const out = document.createElement("canvas");
  out.width = OUT_SIZE;
  out.height = OUT_SIZE;
  const octx = out.getContext("2d");
  if (!octx) return src;
  octx.imageSmoothingQuality = "high";
  octx.drawImage(image, cx - side / 2, cy - side / 2, side, side, 0, 0, OUT_SIZE, OUT_SIZE);
  return out.toDataURL("image/png");
}
