//! Desenfoque del escritorio detrás de la isla (material «cristal líquido»).
//!
//! Windows no ofrece un desenfoque que respete la forma recortada de la ventana: el acrílico pinta toda la
//! ventana como un rectángulo y el desenfoque de DWM solo da transparencia. Así que se hace a mano: se copia de
//! la pantalla el rectángulo que hay detrás de **la isla más grande posible** (no el de la isla de ese momento),
//! se reduce, se desenfoca aquí y se manda a la página como imagen para pintarla debajo del velo del cristal.
//! Como la imagen cubre siempre ese rectángulo máximo y la isla está centrada, la página enseña la porción que
//! toca con `background-position: center top`: al abrir o cerrar la isla no hay que recapturar ni retirar nada, y
//! el fondo desenfocado está ahí en cada fotograma de la animación.
//!
//! **Privacidad.** Lee píxeles del escritorio solo mientras el cristal está activo y con desenfoque mayor que 0.
//! La imagen es pequeña (una cuarta parte) y ya desenfocada, vive en memoria y va únicamente a la ventana de la
//! isla: no se guarda ni sale del equipo, y nada de esto va a los registros.
//!
//! **Efecto secundario.** Para no copiarse a sí misma, la ventana de la isla se excluye de la captura
//! (`WDA_EXCLUDEFROMCAPTURE`): mientras el desenfoque está activo, la isla no sale en capturas de pantalla,
//! grabaciones ni al compartir pantalla. Al apagarlo se restablece.

/// Evento con la imagen desenfocada (`{ uri }`), o `uri` vacío si el cristal se apagó.
pub const EVENT: &str = "island://glass";

#[derive(Clone, serde::Serialize)]
struct GlassFrame {
    uri: String,
    /// Luminosidad media del fondo, 0–255: la página sube el velo cuando el fondo es claro para que el texto blanco se lea.
    luma: u8,
}

/// Cuánto se reduce la captura antes de desenfocarla: el desenfoque es barato y la imagen viaja pequeña (a la
/// mitad, para que con un desenfoque de 7 px el radio en la imagen reducida no sea de 1 o 2 píxeles).
#[cfg(windows)]
const DOWNSCALE: usize = 2;

/// Desenfoque fijo, en píxeles CSS (la sigma de un desenfoque gaussiano). Lo eligió el usuario; sin ajuste.
#[cfg(windows)]
const BLUR_PX: f64 = 7.0;

/// Cuánto se intensifica el color del fondo desenfocado: el cristal de verdad (el de iOS, por ejemplo) no solo
/// desenfoca, también satura lo que hay detrás, y sin eso el resultado queda apagado y turbio.
#[cfg(windows)]
const SATURATION: f32 = 1.45;

/// Cada cuánto se mira si hay que recapturar. Un fondo quieto no genera trabajo: se compara con el anterior.
#[cfg(windows)]
const TICK_MS: u64 = 200;

/// Arranca el hilo. Fuera de Windows no hace nada.
pub fn start(app: tauri::AppHandle) {
    #[cfg(windows)]
    std::thread::spawn(move || run(app));
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn run(app: tauri::AppHandle) {
    use crate::settings;
    use tauri::{Emitter, Manager};

    // La ventana existe desde el arranque, pero su identificador puede tardar un instante.
    let hwnd_raw = loop {
        if let Some(handle) = app.get_webview_window("main").and_then(|w| w.hwnd().ok()) {
            break handle.0 as isize;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    };

    let mut excluded = false;
    let mut last_hash = 0u64;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(TICK_MS));

        // Se consulta en cada vuelta, como en el resto de sondeos, para que apagarlo surta efecto en caliente.
        let config = settings::current();
        let on = config.glass && config.island_style == settings::CornerStyle::Round;
        if on != excluded {
            set_capture_excluded(hwnd_raw, on);
            excluded = on;
            last_hash = 0;
            if !on {
                let _ = app.emit(EVENT, GlassFrame { uri: String::new(), luma: 0 });
            }
        }
        if !on {
            continue;
        }

        let Some(frame) = capture(hwnd_raw) else {
            continue;
        };
        let hash = frame.hash;
        if hash == last_hash {
            continue;
        }
        last_hash = hash;
        // Solo en compilaciones de desarrollo: con DI_GLASS_DUMP=<carpeta> guarda el último fotograma como BMP para
        // revisarlo a la vista (con la isla excluida de la captura, una captura de pantalla no la enseña).
        #[cfg(debug_assertions)]
        if let Some(dir) = std::env::var_os("DI_GLASS_DUMP") {
            use base64::Engine;
            if let Some(b64) = frame.uri.split("base64,").nth(1) {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                    let _ = std::fs::write(std::path::Path::new(&dir).join("glass-frame.bmp"), bytes);
                }
            }
        }
        let _ = app.emit(EVENT, GlassFrame { uri: frame.uri, luma: frame.luma });
    }
}

/// Excluye (o vuelve a incluir) la ventana de las capturas de pantalla.
#[cfg(windows)]
fn set_capture_excluded(hwnd_raw: isize, excluded: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };
    unsafe {
        let _ = SetWindowDisplayAffinity(
            HWND(hwnd_raw as _),
            if excluded { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE },
        );
    }
}

#[cfg(windows)]
struct Frame {
    uri: String,
    hash: u64,
    luma: u8,
}

/// Copia, reduce y desenfoca el rectángulo de pantalla que hay detrás de la isla más grande posible: su ancho máximo
/// (centrado) por su alto máximo, desde la separación del borde. No depende del tamaño de la isla en ese momento.
#[cfg(windows)]
fn capture(hwnd_raw: isize) -> Option<Frame> {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
        ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HGDIOBJ,
        SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(HWND(hwnd_raw as _), &mut rect).ok()?;
    }
    let win_w = crate::MAX_ISLAND_WIDTH + crate::SIDE_MARGIN * 2.0;
    let scale = (rect.right - rect.left) as f64 / win_w;
    if scale <= 0.0 {
        return None;
    }
    let left = rect.left + (crate::island_left(crate::MAX_ISLAND_WIDTH) * scale).round() as i32;
    let top = rect.top + (crate::ROUND_GAP * scale).round() as i32;
    let pw = (crate::MAX_ISLAND_WIDTH * scale).round() as i32;
    let ph = (crate::MAX_ISLAND_HEIGHT * scale).round() as i32;
    if pw < 8 || ph < 8 {
        return None;
    }

    let mut bgra = vec![0u8; (pw * ph * 4) as usize];
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, pw, ph);
        let old = SelectObject(memory, HGDIOBJ(bitmap.0));
        let copied = BitBlt(memory, 0, 0, pw, ph, Some(screen), left, top, SRCCOPY | CAPTUREBLT).is_ok();
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: pw,
                biHeight: -ph, // de arriba abajo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let rows = if copied {
            GetDIBits(memory, bitmap, 0, ph as u32, Some(bgra.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS)
        } else {
            0
        };
        SelectObject(memory, old);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        if rows == 0 {
            return None;
        }
    }

    // Reducir: media de cada bloque de DOWNSCALE × DOWNSCALE píxeles.
    let (w, h) = ((pw as usize) / DOWNSCALE, (ph as usize) / DOWNSCALE);
    if w < 2 || h < 2 {
        return None;
    }
    let mut small = vec![0f32; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u32; 3];
            for dy in 0..DOWNSCALE {
                for dx in 0..DOWNSCALE {
                    let i = (((y * DOWNSCALE + dy) * pw as usize) + (x * DOWNSCALE + dx)) * 4;
                    sum[0] += bgra[i] as u32;
                    sum[1] += bgra[i + 1] as u32;
                    sum[2] += bgra[i + 2] as u32;
                }
            }
            let n = (DOWNSCALE * DOWNSCALE) as f32;
            let o = (y * w + x) * 3;
            small[o] = sum[0] as f32 / n;
            small[o + 1] = sum[1] as f32 / n;
            small[o + 2] = sum[2] as f32 / n;
        }
    }

    // Tres pasadas de caja de radio r equivalen a una gaussiana de sigma² ≈ r (r + 1): se despeja r para la sigma pedida,
    // en píxeles de la imagen reducida.
    let sigma = BLUR_PX * scale / DOWNSCALE as f64;
    let radius = (((1.0 + 4.0 * sigma * sigma).sqrt() - 1.0) / 2.0).round().max(1.0) as usize;
    box_blur(&mut small, w, h, radius);
    box_blur(&mut small, w, h, radius);
    box_blur(&mut small, w, h, radius);

    saturate(&mut small, SATURATION);
    let bytes: Vec<u8> = small.iter().map(|v| v.round().clamp(0.0, 255.0) as u8).collect();
    let luma = (bytes.chunks_exact(3).map(|p| 0.0722 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.2126 * p[2] as f32).sum::<f32>() / (bytes.len() / 3) as f32).round() as u8;
    let mut hash = 0xcbf29ce484222325u64;
    for b in &bytes {
        hash = (hash ^ *b as u64).wrapping_mul(0x100000001b3);
    }
    Some(Frame { uri: bmp_data_uri(&bytes, w, h), hash, luma })
}

/// Aumenta la saturación de una imagen BGR (3 canales por píxel): cada canal se aleja de su gris.
#[cfg(windows)]
fn saturate(data: &mut [f32], amount: f32) {
    for p in data.chunks_exact_mut(3) {
        let gray = 0.0722 * p[0] + 0.7152 * p[1] + 0.2126 * p[2];
        for c in p.iter_mut() {
            *c = gray + (*c - gray) * amount;
        }
    }
}

/// Desenfoque de caja separable con los bordes repetidos (sin oscurecer ni aclarar las orillas).
fn box_blur(data: &mut [f32], w: usize, h: usize, radius: usize) {
    let r = radius as isize;
    let window = (2 * radius + 1) as f32;
    let mut line = vec![0f32; w.max(h) * 3];
    // Horizontal.
    for y in 0..h {
        for c in 0..3 {
            let at = |x: isize| data[(y * w + x.clamp(0, w as isize - 1) as usize) * 3 + c];
            let mut sum: f32 = (-r..=r).map(at).sum();
            for x in 0..w {
                line[x * 3 + c] = sum / window;
                sum += at(x as isize + r + 1) - at(x as isize - r);
            }
        }
        data[y * w * 3..(y + 1) * w * 3].copy_from_slice(&line[..w * 3]);
    }
    // Vertical.
    for x in 0..w {
        for c in 0..3 {
            let at = |y: isize| data[(y.clamp(0, h as isize - 1) as usize * w + x) * 3 + c];
            let mut sum: f32 = (-r..=r).map(at).sum();
            for y in 0..h {
                line[y * 3 + c] = sum / window;
                sum += at(y as isize + r + 1) - at(y as isize - r);
            }
        }
        for y in 0..h {
            for c in 0..3 {
                data[(y * w + x) * 3 + c] = line[y * 3 + c];
            }
        }
    }
}

/// BMP de 24 bits como data URI. Chromium lo acepta en `background-image` y no hace falta un codificador PNG.
fn bmp_data_uri(bgr: &[u8], w: usize, h: usize) -> String {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(-(h as i32)).to_le_bytes()); // de arriba abajo
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in 0..h {
        out.extend_from_slice(&bgr[y * w * 3..(y + 1) * w * 3]);
        out.extend(std::iter::repeat_n(0u8, row - w * 3));
    }
    format!("data:image/bmp;base64,{}", BASE64.encode(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_desenfoque_conserva_un_color_liso() {
        let mut data = vec![100f32; 12 * 6 * 3];
        box_blur(&mut data, 12, 6, 3);
        assert!(data.iter().all(|v| (v - 100.0).abs() < 0.01));
    }

    #[test]
    fn el_desenfoque_reparte_un_punto() {
        let (w, h) = (9, 9);
        let mut data = vec![0f32; w * h * 3];
        let center = (4 * w + 4) * 3;
        data[center] = 810.0;
        box_blur(&mut data, w, h, 1);
        // Un punto de 810 en una caja de 3 × 3 se queda en 90.
        assert!((data[center] - 90.0).abs() < 0.01);
        assert!(data[(4 * w + 5) * 3] > 0.0);
    }

    #[test]
    fn saturar_conserva_el_gris_y_aleja_los_colores() {
        let mut gris = vec![120f32, 120.0, 120.0];
        saturate(&mut gris, 1.5);
        assert!(gris.iter().all(|v| (v - 120.0).abs() < 0.01));
        let mut color = vec![60f32, 100.0, 200.0];
        saturate(&mut color, 1.5);
        assert!(color[2] > 200.0 && color[0] < 60.0);
    }

    #[test]
    fn el_bmp_tiene_cabecera_y_tamano_correctos() {
        let uri = bmp_data_uri(&[0u8; 5 * 3 * 2], 5, 2);
        assert!(uri.starts_with("data:image/bmp;base64,"));
    }
}
