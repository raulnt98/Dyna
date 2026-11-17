mod audio;
mod avatars;
mod bluetooth;
mod brightness;
mod downloads;
mod focus;
mod keys;
mod lyrics;
mod network;
mod power;
mod privacy;
mod settings;
mod shell_icon;
mod smtc;
mod speedtest;
mod sysstats;
mod toasts;
mod usb;
mod volume;
mod weather;

use settings::{Position, Settings};
use bluetooth::BLUETOOTH_EVENT;
use network::{InternetState, INTERNET_EVENT, WIFI_EVENT};
use power::{LOW_BATTERY_EVENT, POWER_EVENT};
use volume::VOLUME_EVENT;
use smtc::{Command, MediaState, MEDIA_EVENT};
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

/// Holgura alrededor de la isla dentro de la ventana, para su sombra y para
/// dar margen al ratón al acercarse. La isla va centrada en la ventana, así
/// que este valor es también su separación del borde de la pantalla.
const WINDOW_PADDING: f64 = 18.0;

/// Margen extra para las esquinas invertidas.
///
/// Las muescas que unen la isla con el canto de la pantalla se dibujan fuera
/// de su ancho nominal: 22 px por lado con la isla expandida (`--notch` en
/// IslandApp.vue). Sin esta holgura se recortarían contra el borde de la
/// ventana. El resto es margen de seguridad.
const OVERSHOOT_MARGIN: f64 = 32.0;

/// Ancho máximo que puede tener la isla (con la letra abierta, 540 px).
const MAX_ISLAND_WIDTH: f64 = 540.0;
/// Alto máximo de la isla expandida.
const MAX_ISLAND_HEIGHT: f64 = 108.0;
/// Holgura horizontal a cada lado de la isla dentro de su zona visible.
const SIDE_MARGIN: f64 = WINDOW_PADDING + OVERSHOOT_MARGIN;

/// Tamaño actual de la isla, para recolocarla al cambiar los ajustes.
static ISLAND_SIZE: std::sync::Mutex<(f64, f64)> = std::sync::Mutex::new((116.0, 34.0));

/// Separación con el borde de la pantalla de la vista expandida redondeada, en píxeles
/// lógicos. La misma medida que `margin-top` de `.island-wrap.expanded-round` en
/// IslandApp.vue: si cambias una, cambia la otra.
const ROUND_GAP: f64 = 6.0;

/// Evento: el cursor entra o sale de la franja entre el borde de la pantalla y la isla
/// (variantes redondeadas, que flotan). Esa franja queda fuera del recorte de la ventana, así
/// que la página no recibe ningún evento de ratón sobre ella; se averigua desde aquí.
const GAP_EVENT: &str = "island://gap";

#[derive(Clone, serde::Serialize)]
struct GapHover {
    inside: bool,
}

/// Forma de la isla en un instante: lo que el recorte de la ventana dibuja.
///
/// Además del tamaño lleva la muesca y el radio, porque en el CSS también
/// cambian con el estado (14 px y 18 compacta, 22 y 18 expandida) y se animan
/// con la misma curva que el ancho y el alto.
#[derive(Clone, Copy, PartialEq)]
struct Shape {
    width: f64,
    height: f64,
    notch: f64,
    radius: f64,
    /// Radio de las esquinas de arriba (0 = rectas, de ellas nacen las muescas).
    top_radius: f64,
    /// Separación entre el borde de la pantalla y la isla (0 = pegada). Solo la
    /// tiene la vista expandida redondeada, que flota en vez de colgar.
    top_offset: f64,
}

impl Shape {
    /// Forma de reposo para un tamaño de isla. La altura distingue el estado:
    /// la pestaña mínima mide 8 px, la píldora compacta 34 y cualquier panel
    /// pasa de 40. Las medidas de muesca y radio replican las del CSS
    /// (`--notch` en IslandApp.vue y los `rounded-[…]` de TheIsland.vue).
    fn for_size(width: f64, height: f64) -> Shape {
        // Un solo ajuste manda sobre la isla cerrada y la abierta: o las dos con muescas, o las dos flotantes.
        let round = settings::current().island_style == settings::CornerStyle::Round;
        let (round_expanded, round_collapsed) = (round, round);
        let (notch, radius, top_radius, top_offset) = if height > 40.0 {
            // Vista expandida: con el estilo redondeado no hay muescas y las
            // esquinas de arriba se curvan como las de abajo.
            if round_expanded {
                (0.0, 18.0, 18.0, ROUND_GAP)
            } else {
                (22.0, 18.0, 0.0, 0.0)
            }
        } else if height > 12.0 {
            // Píldora cerrada. Redondeada: sin muescas, las cuatro esquinas curvas (radio de media altura) y
            // flotando como la expandida redondeada. Lo replica `.island-wrap.collapsed-round` (IslandApp.vue).
            if round_collapsed {
                (0.0, 17.0, 17.0, ROUND_GAP)
            } else {
                (14.0, 18.0, 0.0, 0.0)
            }
        } else if round_collapsed {
            // Pestaña sin muescas: solo la lengüeta con las esquinas de abajo redondeadas, pegada al borde.
            (0.0, 4.0, 0.0, 0.0)
        } else {
            // Pestaña: una muesca de 14 px colgaría por debajo de ella.
            (5.0, 4.0, 0.0, 0.0)
        };
        Shape {
            width,
            height,
            notch,
            radius,
            top_radius,
            top_offset,
        }
    }

    fn lerp(from: Shape, to: Shape, t: f64) -> Shape {
        let mix = |a: f64, b: f64| a + (b - a) * t;
        Shape {
            width: mix(from.width, to.width),
            height: mix(from.height, to.height),
            notch: mix(from.notch, to.notch),
            radius: mix(from.radius, to.radius),
            top_radius: mix(from.top_radius, to.top_radius),
            top_offset: mix(from.top_offset, to.top_offset),
        }
    }
}

/// Forma que tiene ahora mismo el recorte, para que una animación interrumpida
/// parta de donde está y no de donde iba (igual que hace una transición CSS).
static DISPLAY: std::sync::Mutex<Shape> = std::sync::Mutex::new(Shape {
    width: 116.0,
    height: 34.0,
    notch: 14.0,
    radius: 18.0,
    top_radius: 0.0,
    top_offset: 0.0,
});

/// Generación de la animación en curso: al cambiar, el hilo anterior se retira.
static ANIM_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Duración del `.island-morph` del CSS, en segundos, cuando la isla la abre o la
/// cierra el usuario. Los avisos del sistema piden una más corta (ver `island_resize`).
const MORPH_SECS: f64 = 0.58;

/// `cubic-bezier(.22, 1, .36, 1)`, la curva de `.island-morph`: devuelve el
/// progreso de la animación para un tiempo normalizado `t` de 0 a 1.
///
/// Hace falta replicarla: el CSS anima el tamaño de la píldora y Rust el del
/// recorte, y si las curvas difieren uno asoma por detrás del otro.
fn morph_ease(t: f64) -> f64 {
    let (x1, y1, x2, y2) = (0.22, 1.0, 0.36, 1.0);
    let bezier = |s: f64, a: f64, b: f64| {
        let inv = 1.0 - s;
        3.0 * inv * inv * s * a + 3.0 * inv * s * s * b + s * s * s
    };
    // El tiempo es la coordenada x de la curva: se busca el parámetro `s` que
    // la produce y con él se lee la y. Bisección: 20 pasos dan sobrada precisión.
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        if bezier(mid, x1, x2) < t {
            low = mid;
        } else {
            high = mid;
        }
    }
    bezier((low + high) / 2.0, y1, y2)
}

/// Región de la píldora construida fila a fila, con el círculo de cada esquina exacto.
///
/// `CreateRoundRectRgn` aproxima la esquina con una elipse de Bresenham que deja
/// tramos planos y escalones desiguales: es lo que se veía «mordido». Aquí cada fila de
/// píxeles se calcula con la circunferencia verdadera, de modo que los escalones son los
/// mínimos posibles y simétricos entre las cuatro esquinas.
///
/// Un píxel solo entra si queda **entero** dentro de la forma (se evalúa su borde más
/// exterior): un píxel a medias tiene el antialiasing del CSS, es decir, es medio
/// transparente, y Windows pinta de pálido lo transparente en el borde de la región.
/// Los costados rectos van a ras, como siempre. `left`/`right` son el borde izquierdo y
/// el derecho (exclusivo) en píxeles; `bottom_r` y `top_r`, los radios de las esquinas
/// de abajo y de arriba (0 = rectas) y `notch`, el tamaño de las muescas cóncavas (0 =
/// sin ellas), todo en píxeles.
///
/// Las muescas entran en la misma cuenta: en cada fila de su altura el tramo se
/// ensancha por los dos lados hasta el círculo del CSS (el que tiene el centro en la
/// esquina inferior exterior de su cuadrado), con el mismo criterio de «píxel entero».
/// Antes eran cuadrados menos elipses con holguras puestas a ojo (3 px recortados, círculo
/// 2 px mayor), y esas holguras eran lo que dejaba la punta de la muesca «mordida».
#[cfg(windows)]
unsafe fn pill_region(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    bottom_r: f64,
    top_r: f64,
    notch: f64,
) -> windows::Win32::Graphics::Gdi::HRGN {
    use windows::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, HGDIOBJ, RGN_OR};

    let region = CreateRectRgn(0, 0, 0, 0);
    // Filas seguidas con el mismo tramo se agrupan en un solo rectángulo.
    let mut run: Option<(i32, i32, i32)> = None; // (x0, x1, y de inicio)
    let emit = |run: (i32, i32, i32), end_y: i32| {
        let rect = CreateRectRgn(run.0, run.2, run.1, end_y);
        let _ = CombineRgn(Some(region), Some(region), Some(rect), RGN_OR);
        let _ = DeleteObject(HGDIOBJ(rect.0));
    };
    for (index, span) in pill_spans(left, top, right, bottom, bottom_r, top_r, notch).into_iter().enumerate() {
        let y = top + index as i32;
        match run {
            Some((x0, x1, _)) if (x0, x1) == span => {}
            Some(previous) => {
                emit(previous, y);
                run = Some((span.0, span.1, y));
            }
            None => run = Some((span.0, span.1, y)),
        }
    }
    if let Some(last) = run {
        emit(last, bottom);
    }
    region
}

/// Tramo horizontal `(x0, x1)` (x1 exclusivo) de cada fila de píxeles de la forma, de
/// `top` a `bottom`. Es la geometría pura de `pill_region`, aparte para poder comprobarla.
fn pill_spans(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    bottom_r: f64,
    top_r: f64,
    notch: f64,
) -> Vec<(i32, i32)> {
    // Cuánto se mete el borde en una fila por el círculo de una esquina, o 0 si la fila
    // queda fuera de la zona curva. `edge` es el borde del píxel que más se mete.
    let inset = |edge: f64, center: f64, r: f64| -> f64 {
        if r <= 0.0 {
            return 0.0;
        }
        let dy = (edge - center).abs();
        if dy >= r {
            r
        } else {
            r - (r * r - dy * dy).sqrt()
        }
    };

    let mut spans = Vec::with_capacity((bottom - top).max(0) as usize);
    for y in top..bottom {
        // Abajo, el borde que más se mete es el inferior del píxel; arriba, el superior.
        let from_bottom = if ((y + 1) as f64) > bottom as f64 - bottom_r {
            inset((y + 1) as f64, bottom as f64 - bottom_r, bottom_r)
        } else {
            0.0
        };
        let from_top = if (y as f64) < top as f64 + top_r {
            inset(y as f64, top as f64 + top_r, top_r)
        } else {
            0.0
        };
        let cut = from_bottom.max(from_top).ceil() as i32;
        let mut span = (left + cut, right - cut);
        if notch > 0.5 {
            // Distancia vertical del borde inferior del píxel al centro del círculo de la
            // muesca (que está a `notch` por debajo del borde de arriba). Fuera de su
            // altura no hay muesca.
            let d = (top as f64 + notch) - (y + 1) as f64;
            if d >= 0.0 {
                let x_off = (notch * notch - d * d).max(0.0).sqrt();
                let outer_left = (left as f64 - notch + x_off).ceil() as i32;
                let outer_right = (right as f64 + notch - x_off).floor() as i32;
                span = (span.0.min(outer_left), span.1.max(outer_right));
            }
        }
        spans.push(span);
    }
    spans
}

/// Recorta la ventana a la forma de la isla.
///
/// Píldora redondeada más las dos muescas cóncavas de las esquinas. No puede
/// quedar ningún píxel transparente dentro: Windows acaba pintando de pálido
/// los de la zona visible (franjas claras a cada lado de la píldora al cabo de
/// unos segundos). Las medidas replican las del CSS. `left` es el borde
/// izquierdo de la píldora dentro de la ventana, en píxeles lógicos.
#[cfg(windows)]
unsafe fn set_island_region(
    hwnd: windows::Win32::Foundation::HWND,
    scale: f64,
    left: f64,
    shape: &Shape,
) {
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, SetWindowRgn, HGDIOBJ,
    };

    let to_px = |value: f64| (value * scale).round() as i32;
    let Shape { width, height, notch, radius, top_radius, top_offset } = *shape;
    let right = left + width;
    // `CreateRoundRectRgn` deja fuera su columna derecha, a diferencia de
    // `CreateRectRgn` (que usan las muescas): sin el +1 queda un hueco de un
    // píxel entre la píldora y la muesca derecha, que Windows pinta de claro
    // (línea vertical solo a la derecha). Un píxel hacia dentro en la base por
    // lo mismo; los costados van a ras.
    // Estilo flotante (sin muescas, separado del borde): el recorte sube 1 px por todos los lados para dejar sitio a
    // un anillo OPACO y negro (`--island-ring`). El suavizado del borde se hace
    // entre la isla y ese anillo, dos colores opacos, y el borde duro del recorte cae sobre el anillo, que es del
    // color del escritorio y por tanto no se ve.
    //
    // Por qué no basta con dejar que el CSS suavice contra lo que haya detrás: un píxel a medias dentro de esta
    // ventana NO se mezcla con el escritorio sino con **blanco** (medido en la app real: sobre un escritorio de 39 y
    // una isla de 10, los píxeles del borde valían 89, 69 y 101, más claros que los dos). Sobre un fondo blanco ese
    // fallo es invisible, que es por lo que una prueba con fondo blanco da por buena una solución que no lo es.
    let g: i32 = if top_offset > 0.5 { 1 } else { 0 };
    let region = pill_region(
        to_px(left) - g,
        to_px(top_offset) - g,
        to_px(right) + g,
        to_px(top_offset + height) - 1 + g,
        radius * scale + g as f64,
        top_radius * scale + g as f64,
        notch * scale,
    );
    // Windows se queda con la región al aceptarla: solo hay que liberarla si
    // la llamada falla.
    if SetWindowRgn(hwnd, Some(region), true) == 0 {
        let _ = DeleteObject(HGDIOBJ(region.0));
    }
}

/// ¿Está el cursor en la franja de encima de la isla, entre ella y el borde de la pantalla?
///
/// Lo cuenta como parte de la isla: con el estilo redondeado la isla flota unos píxeles por
/// debajo del borde, y sin esto, subir el ratón hasta el borde la cerraría. La franja se
/// toma entre el borde de la pantalla y la isla, dentro de su ancho. La escala sale del
/// tamaño real de la ventana, que es fijo, para no depender del monitor.
#[cfg(windows)]
fn cursor_in_gap(hwnd_raw: isize, island_width: f64) -> bool {
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

    let mut cursor = POINT::default();
    let mut rect = RECT::default();
    unsafe {
        if GetCursorPos(&mut cursor).is_err() || GetWindowRect(HWND(hwnd_raw as _), &mut rect).is_err() {
            return false;
        }
    }
    let win_w = MAX_ISLAND_WIDTH + SIDE_MARGIN * 2.0;
    let scale = (rect.right - rect.left) as f64 / win_w;
    if scale <= 0.0 {
        return false;
    }
    let left = rect.left as f64 + island_left(island_width) * scale;
    let right = left + island_width * scale;
    // Desde el borde de la pantalla (la ventana puede estar despegada por el ajuste de
    // separación) hasta donde empieza la isla.
    let top = rect.top as f64 - settings::current().top_margin.max(0.0) * scale;
    let island_top = rect.top as f64 + ROUND_GAP * scale;
    let (x, y) = (cursor.x as f64, cursor.y as f64);
    x >= left && x <= right && y >= top && y < island_top
}

/// Vigila el cursor sobre la franja y avisa a la isla solo cuando cambia.
#[cfg(windows)]
fn start_gap_watch(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || {
        // La ventana existe desde el arranque, pero su identificador puede tardar un instante.
        let hwnd_raw = loop {
            if let Some(handle) = app.get_webview_window("main").and_then(|w| w.hwnd().ok()) {
                break handle.0 as isize;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        };
        let mut was_inside = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(40));
            let (width, height) = ISLAND_SIZE.lock().map(|size| *size).unwrap_or((116.0, 34.0));
            // Solo hay franja con el estilo flotante, en la isla abierta y en la píldora cerrada. La pestaña
            // (8 px) no flota y no la tiene.
            let has_gap = settings::current().island_style == settings::CornerStyle::Round && height > 12.0;
            let inside = has_gap && cursor_in_gap(hwnd_raw, width);
            if inside != was_inside {
                was_inside = inside;
                let _ = app.emit(GAP_EVENT, GapHover { inside });
            }
        }
    });
}

#[cfg(not(windows))]
fn start_gap_watch(_app: tauri::AppHandle) {}

/// Borde izquierdo de la píldora dentro de la ventana fija, según la alineación.
fn island_left(width: f64) -> f64 {
    let win_w = MAX_ISLAND_WIDTH + SIDE_MARGIN * 2.0;
    let area_w = width + SIDE_MARGIN * 2.0;
    let area_x = match settings::current().position {
        Position::Left => 0.0,
        Position::Center => (win_w - area_w) / 2.0,
        Position::Right => win_w - area_w,
    };
    area_x + SIDE_MARGIN
}

/// Lleva el recorte de la isla a un tamaño nuevo animándolo.
///
/// **El recorte debe seguir a la píldora del CSS**: si cambia de golpe, durante
/// los 0,58 s de la transición queda su forma vieja y Windows pinta de pálido
/// lo que sobra (al achicar se veía un rectángulo grande y luego un salto). Un
/// hilo lo interpola con la misma curva y duración que `.island-morph`.
///
/// El recorte no debe ir por delante de la píldora, ni al crecer: lo que asoma
/// de más lo pinta Windows de pálido, mientras que quedarse unos milisegundos
/// corto solo recorta un borde de la píldora, que apenas se nota. La latencia
/// natural de la llamada ya lo deja ligeramente por detrás.
fn animate_island(window: &WebviewWindow, width: f64, height: f64, secs: f64) {
    use std::sync::atomic::Ordering;

    if let Ok(mut size) = ISLAND_SIZE.lock() {
        *size = (width, height);
    }
    let generation = ANIM_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let target = Shape::for_size(width, height);
    let from = DISPLAY.lock().map(|shape| *shape).unwrap_or(target);
    if from == target {
        return;
    }

    let window = window.clone();
    // El `hwnd` y la escala se piden una vez, aquí: cada una de esas llamadas a
    // `WebviewWindow` espera al hilo principal, y hacerlo en cada fotograma
    // (unos 70 en la animación) frenaba la isla en equipos lentos. El `hwnd`
    // viaja como entero porque `HWND` no es `Send`.
    #[cfg(windows)]
    let hwnd_raw = window.hwnd().ok().map(|handle| handle.0 as isize);
    #[cfg(windows)]
    let scale = window.scale_factor().unwrap_or(1.0);

    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        loop {
            if ANIM_GEN.load(Ordering::SeqCst) != generation {
                return;
            }
            let t = (start.elapsed().as_secs_f64() / secs).min(1.0);
            if t >= 1.0 {
                // Fotograma final: el completo, con posición, desenfoque y repintado.
                if ANIM_GEN.load(Ordering::SeqCst) == generation {
                    place_island(&window, width, height);
                }
                return;
            }
            let shape = Shape::lerp(from, target, morph_ease(t));
            #[cfg(windows)]
            if let Some(raw) = hwnd_raw {
                unsafe {
                    set_island_region(
                        windows::Win32::Foundation::HWND(raw as _),
                        scale,
                        island_left(shape.width),
                        &shape,
                    );
                }
            }
            if let Ok(mut shown) = DISPLAY.lock() {
                *shown = shape;
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
    });
}

/// Pantalla donde cuelga la isla según el ajuste `monitor`: 0 es la principal; n ≥ 1, la
/// n-ésima de las que informa Windows. Si esa pantalla ya no existe (se desenchufó el
/// segundo monitor), vuelve a la principal en vez de quedarse sin isla.
fn chosen_monitor(window: &WebviewWindow) -> Option<tauri::Monitor> {
    let index = settings::current().monitor as usize;
    if index >= 1 {
        if let Some(monitor) = window.available_monitors().ok().and_then(|list| list.into_iter().nth(index - 1)) {
            return Some(monitor);
        }
    }
    window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
}

/// Una pantalla para el selector de los ajustes.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorOption {
    index: u32,
    label: String,
}

/// Pantallas disponibles, para el selector de los ajustes. El 0 es siempre «principal»:
/// se sigue a la principal aunque Windows cambie cuál es.
#[tauri::command]
fn monitors_list(window: WebviewWindow) -> Vec<MonitorOption> {
    let primary = window.primary_monitor().ok().flatten().map(|m| (m.position().x, m.position().y));
    let mut options = vec![MonitorOption { index: 0, label: "Pantalla principal".into() }];
    if let Ok(list) = window.available_monitors() {
        for (i, monitor) in list.iter().enumerate() {
            let size = monitor.size();
            let is_primary = primary == Some((monitor.position().x, monitor.position().y));
            options.push(MonitorOption {
                index: i as u32 + 1,
                label: format!(
                    "Pantalla {} · {}×{}{}",
                    i + 1,
                    size.width,
                    size.height,
                    if is_primary { " (principal)" } else { "" }
                ),
            });
        }
    }
    options
}

/// Coloca la ventana en el borde superior de la pantalla activa.
///
/// `width` y `height` son el tamaño de la isla. **La ventana no cambia de
/// tamaño**: mide siempre lo que cabe la isla más grande. Redimensionarla al
/// contraer obligaba a WebView2 a actualizar su viewport unos fotogramas
/// después, y durante ese rato se veía el fotograma viejo en otra posición
/// (la píldora duplicada). En su lugar se recorta con `SetWindowRgn` a la
/// zona que antes ocupaba la ventana: lo de fuera no se dibuja ni captura
/// clics, así que la ventana ancha no tapa nada del escritorio.
fn place_island(window: &WebviewWindow, width: f64, height: f64) {
    // Una colocación directa cancela cualquier animación del recorte en curso.
    ANIM_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut size) = ISLAND_SIZE.lock() {
        *size = (width, height);
    }
    let shape = Shape::for_size(width, height);
    if let Ok(mut shown) = DISPLAY.lock() {
        *shown = shape;
    }

    // Zona visible: la isla más su holgura. Es lo que antes era la ventana.
    let area_w = width + SIDE_MARGIN * 2.0;
    // Ventana fija, dimensionada para la isla más grande.
    let win_w = MAX_ISLAND_WIDTH + SIDE_MARGIN * 2.0;
    let win_h = MAX_ISLAND_HEIGHT + WINDOW_PADDING;

    // Se ancla a la pantalla elegida en los ajustes (por defecto, la principal): usar el
    // monitor actual haría que la isla saltara de pantalla según dónde quedara tras el
    // último movimiento.
    let Some(monitor) = chosen_monitor(window) else {
        // Sin monitor conocido no hay forma de centrar: al menos se ajusta el tamaño,
        // aunque quede donde estaba.
        let _ = window.set_size(LogicalSize::new(win_w, win_h));
        return;
    };
    let scale = monitor.scale_factor();
    let screen = monitor.size().to_logical::<f64>(scale);
    let origin = monitor.position().to_logical::<f64>(scale);

    // La alineación y el margen salen de los ajustes del usuario. La zona
    // visible se pega al lado que toque dentro de la ventana fija, de modo
    // que su posición en pantalla es la que tendría una ventana a medida.
    let config = settings::current();
    let (x, area_x) = match config.position {
        Position::Left => (origin.x + 24.0, 0.0),
        Position::Center => (
            origin.x + (screen.width - win_w) / 2.0,
            (win_w - area_w) / 2.0,
        ),
        Position::Right => (
            origin.x + screen.width - win_w - 24.0,
            win_w - area_w,
        ),
    };
    // Colgando del borde, la ventana arranca en el canto de la pantalla. El
    // ajuste de separación todavía permite despegarla, pero su valor por
    // defecto es 0: la forma supone que la isla nace del marco.
    let y = origin.y + config.top_margin.max(0.0);

    // Mover y recortar. `SetWindowPos` mantiene siempre el mismo tamaño.
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER,
        };

        if let Ok(handle) = window.hwnd() {
            // El scale factor de la ventana puede diferir del monitor en
            // configuraciones multipantalla con distinto DPI.
            let scale = window.scale_factor().unwrap_or(scale);
            let to_px = |value: f64| (value * scale).round() as i32;
            let hwnd = HWND(handle.0 as _);
            let ok = unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    to_px(x),
                    to_px(y),
                    to_px(win_w),
                    to_px(win_h),
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                )
            };
            if ok.is_ok() {
                unsafe {
                    set_island_region(hwnd, scale, area_x + SIDE_MARGIN, &shape);
                    // Sin esto quedan píxeles de la expansión anterior dentro
                    // de la zona visible: al no cambiar el tamaño de la
                    // ventana, nada obliga a WebView2 a repintarla entera.
                    // Tao hace transparente la ventana con un desenfoque de
                    // región vacía (`0,0,-1,-1`). Con la ventana recortada,
                    // Windows lo trata como «toda la zona visible» y pinta
                    // de pálido el fondo desenfocado al perder el foco. Una
                    // región de 1x1 px en la esquina de la ventana, fuera de
                    // la zona visible, mantiene la transparencia sin
                    // desenfoque a la vista.
                    reapply_transparency(hwnd);
                }
                return;
            }
        }
    }

    // Fuera de Windows, o si `SetWindowPos` falla, se recurre a las dos
    // llamadas por separado (sin recorte: la ventana queda a tamaño fijo).
    let _ = window.set_size(LogicalSize::new(win_w, win_h));
    let _ = window.set_position(LogicalPosition::new(x, y));
}

/// Reaplica la transparencia y repinta la ventana recortada.
///
/// Tao hace transparente la ventana con un desenfoque de región vacía
/// (`0,0,-1,-1`). Con la ventana recortada, Windows lo trata como «toda la zona
/// visible» y pinta de pálido el fondo desenfocado cuando la ventana cambia de
/// estado. Una región de 1x1 px en la esquina de la ventana, fuera de la zona
/// visible, mantiene la transparencia sin desenfoque a la vista. Hay que
/// repetirlo tras cualquier `SetWindowPos` que toque la ventana, y repintar:
/// al no cambiar el tamaño, nada obliga a WebView2 a hacerlo por su cuenta.
#[cfg(windows)]
fn reapply_transparency(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Graphics::Dwm::{
        DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
    };
    use windows::Win32::Graphics::Gdi::{
        CreateRectRgn, DeleteObject, RedrawWindow, HGDIOBJ, RDW_ALLCHILDREN, RDW_INVALIDATE,
        RDW_UPDATENOW,
    };
    unsafe {
        let corner = CreateRectRgn(0, 0, 1, 1);
        let blur = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            fEnable: true.into(),
            hRgnBlur: corner,
            fTransitionOnMaximized: false.into(),
        };
        let _ = DwmEnableBlurBehindWindow(hwnd, &blur);
        let _ = DeleteObject(HGDIOBJ(corner.0));
        let _ = RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        );
    }
}

/// Vuelve a colocar la ventana por encima de las demás, sin activarla.
///
/// `always_on_top` se aplica una sola vez; si otra ventana «siempre visible»
/// pasa por delante después (un reproductor flotante, el administrador de
/// tareas), la isla queda debajo y un aviso saldría tapado. Respeta el ajuste:
/// con «Siempre visible» apagado no se toca el orden.
///
/// Solo actúa si hay de verdad una ventana visible «siempre visible» por
/// delante. Tocar el orden con `SetWindowPos` no es gratis en una ventana
/// recortada y transparente (ver `reapply_transparency`), y hacerlo en cada
/// cambio de tamaño, cuando la isla ya está arriba, solo arriesgaba artefactos.
fn bring_to_front(window: &WebviewWindow) {
    if !settings::current().always_on_top {
        return;
    }
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindow, GetWindowLongPtrW, IsWindowVisible, SetWindowPos, GWL_EXSTYLE, GW_HWNDPREV,
            HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
        };
        let Ok(handle) = window.hwnd() else { return };
        let hwnd = HWND(handle.0 as _);

        // Las ventanas por encima de la nuestra, de la más cercana hacia arriba.
        // Si alguna es visible y «siempre visible», nos está tapando.
        let covered = unsafe {
            let mut above = GetWindow(hwnd, GW_HWNDPREV).ok();
            let mut found = false;
            while let Some(other) = above {
                let topmost = GetWindowLongPtrW(other, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
                if topmost && IsWindowVisible(other).as_bool() {
                    found = true;
                    break;
                }
                above = GetWindow(other, GW_HWNDPREV).ok();
            }
            found
        };
        if !covered {
            return;
        }

        // SWP_NOACTIVATE: subir la isla no debe robar el foco a lo que el
        // usuario esté escribiendo.
        let raised = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
        if raised.is_ok() {
            reapply_transparency(hwnd);
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

/// Ajusta la ventana al tamaño que pide la isla al expandirse o contraerse.
///
/// `duration_ms` es la duración de la transición CSS que se está lanzando a la vez:
/// el recorte tiene que durar lo mismo que la píldora, o uno asoma por detrás del
/// otro. Sin valor se usa la de siempre. Se acota para que un dato absurdo no deje
/// el recorte atascado ni lo salte.
#[tauri::command]
fn island_resize(window: WebviewWindow, width: f64, height: f64, duration_ms: Option<f64>) {
    bring_to_front(&window);
    let secs = duration_ms.map_or(MORPH_SECS, |ms| (ms / 1000.0).clamp(0.15, 1.0));
    animate_island(&window, width, height, secs);
}

/// Deja pasar los clics a lo que haya debajo, o vuelve a capturarlos.
///
/// Mientras la isla está compacta no debe robar clics al escritorio; al
/// expandirse necesita recibirlos para sus botones.
#[tauri::command]
fn island_set_interactive(window: WebviewWindow, interactive: bool) {
    let _ = window.set_ignore_cursor_events(!interactive);
}

/// Letra sincronizada de la pista que suena ahora.
///
/// Devuelve una letra vacía si el usuario desactivó la búsqueda en los
/// ajustes, de modo que nada sale del equipo sin su permiso.
#[tauri::command(async)]
fn lyrics_current() -> lyrics::Lyrics {
    if !settings::current().lyrics {
        return lyrics::Lyrics::default();
    }
    let state = smtc::read_state();
    // El karaoke es solo para streaming: con un reproductor local la letra ni
    // se consulta, y por tanto no sale nada del equipo.
    if !state.has_session || !lyrics::is_streaming_source(&state.source_app) {
        return lyrics::Lyrics::default();
    }
    lyrics::for_track(&state.title, &state.artist, &state.album, state.duration)
}

/// Estado actual de la batería y la corriente.
///
/// Estado del permiso de Windows para leer notificaciones (ver `toasts::access_status`).
#[tauri::command(async)]
fn toasts_access() -> &'static str {
    toasts::access_status()
}

/// Abre la página de Windows donde se concede el acceso a notificaciones.
#[tauri::command(async)]
fn toasts_open_privacy() -> bool {
    toasts::open_privacy_settings()
}

/// Abre la aplicación que emitió un aviso, al hacer clic en él en la isla.
///
/// Recibe el id del aviso, no un identificador de aplicación: ver `toasts::open`.
#[tauri::command(async)]
fn toast_open(id: u32) -> bool {
    toasts::open(id)
}

/// La isla lo pide al arrancar: el sondeo solo avisa de los cambios, así que
/// sin esto no sabría si ya estaba enchufado.
#[tauri::command(async)]
fn power_state() -> power::PowerState {
    power::read_state()
}

/// Volumen maestro del sistema, de 0 a 100, y si está silenciado.
#[tauri::command(async)]
fn volume_get() -> (u8, bool) {
    (volume::get(), volume::muted())
}

/// Ajusta el volumen maestro del sistema.
#[tauri::command(async)]
fn volume_set(value: u8) -> bool {
    volume::set(value)
}

/// Silencia o restaura el audio del sistema.
#[tauri::command(async)]
fn volume_toggle_mute() -> bool {
    let next = !volume::muted();
    volume::set_muted(next);
    next
}

/// Devuelve los ajustes guardados.
#[tauri::command]
fn settings_get() -> Settings {
    settings::current()
}

/// Guarda los ajustes y los aplica de inmediato a la isla.
#[tauri::command]
fn settings_set(app: tauri::AppHandle, settings: Settings) {
    let autostart_changed = settings::current().autostart != settings.autostart;
    settings::store(&settings);

    if autostart_changed {
        settings::set_autostart(settings.autostart);
    }

    if let Some(island) = app.get_webview_window("main") {
        let _ = island.set_always_on_top(settings.always_on_top);
        // Se reposiciona con el tamaño de isla que tenía.
        let (width, height) = ISLAND_SIZE.lock().map(|size| *size).unwrap_or((116.0, 34.0));
        place_island(&island, width, height);
        // La isla necesita saber los ajustes (alineación, salto de los botones).
        let _ = island.emit("settings://changed", &settings);
    }
}

/// Cierra la ventana de ajustes sin terminar la aplicación.
#[tauri::command]
fn settings_close(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.hide();
    }
}

/// Abre el panel de ajustes, creándolo si aún no existe.
fn open_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        // Una ventana minimizada sigue existiendo y `show()` no la restaura: sin
        // `unminimize()` el clic en «Ajustes…» no hacía nada visible.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("Ajustes · Dynamic Island")
    .inner_size(440.0, 640.0)
    // Tamaño fijo: el panel está maquetado para estas medidas. `resizable(false)`
    // por sí solo no siempre desactiva el botón de maximizar, así que se quita
    // aparte y se fijan también los límites.
    .resizable(false)
    .maximizable(false)
    .min_inner_size(440.0, 640.0)
    .max_inner_size(440.0, 640.0)
    .center()
    .build();
}

/// Devuelve el estado multimedia actual del sistema.
#[tauri::command(async)]
fn media_state() -> MediaState {
    smtc::read_state()
}

/// Alterna reproducir/pausar en el reproductor activo.
#[tauri::command(async)]
fn media_play_pause() -> bool {
    smtc::control(Command::PlayPause)
}

/// Salta a la pista siguiente.
#[tauri::command(async)]
fn media_next() -> bool {
    smtc::control(Command::Next)
}

/// Vuelve a la pista anterior.
#[tauri::command(async)]
fn media_previous() -> bool {
    smtc::control(Command::Previous)
}

/// Salta a una posición de la pista actual, en segundos desde el inicio.
#[tauri::command(async)]
fn media_seek(seconds: f64) -> bool {
    smtc::control(Command::Seek(seconds))
}

/// Crea el icono de la bandeja con su menú.
///
/// La isla no aparece en la barra de tareas, así que este icono es la única
/// forma de abrir los ajustes o cerrar la aplicación.
fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let settings_item = MenuItem::with_id(app, "settings", "Ajustes…", true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(app, "toggle", "Mostrar u ocultar", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&settings_item, &toggle_item, &separator, &quit_item],
    )?;

    let mut builder = TrayIconBuilder::with_id("island-tray")
        .menu(&menu)
        .tooltip("Dynamic Island")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => open_settings(app),
            "toggle" => {
                if let Some(window) = app.get_webview_window("main") {
                    match window.is_visible() {
                        Ok(true) => {
                            let _ = window.hide();
                        }
                        _ => {
                            let _ = window.show();
                        }
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });

    // El icono de la ventana sirve también para la bandeja.
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            media_state,
            media_play_pause,
            media_next,
            media_previous,
            media_seek,
            island_resize,
            island_set_interactive,
            settings_get,
            settings_set,
            settings_close,
            volume_get,
            volume_set,
            volume_toggle_mute,
            lyrics_current,
            toast_open,
            toasts_access,
            toasts_open_privacy,
            power_state,
            monitors_list,
            weather::weather_current,
            weather::weather_search,
            weather::weather_locate_here,
            speedtest::speedtest_run,
            sysstats::system_stats
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // La ventana nace oculta: se coloca arriba y centrada antes de
            // mostrarla, para que no aparezca un instante en otra posición.
            if let Some(window) = app.get_webview_window("main") {
                let config = settings::current();
                let _ = window.set_always_on_top(config.always_on_top);
                place_island(&window, 116.0, 34.0);
                let _ = window.show();
            }

            // Icono en la bandeja: el panel sigue accesible con la isla en
            // segundo plano, que es la única parte visible de la aplicación.
            build_tray(app.handle())?;

            // Evento inmediato cuando el sistema cambia de reproductor. Si no
            // se puede registrar, el sondeo de abajo sigue cubriendo el caso.
            let event_handle = handle.clone();
            let _watching = smtc::watch(move || {
                let _ = event_handle.emit(MEDIA_EVENT, smtc::read_state());
            });

            // SMTC no notifica el avance continuo de la pista, así que la
            // posición se refresca por sondeo. Solo se emite cuando el estado
            // cambia, para no despertar al frontend sin motivo; la carátula
            // está cacheada por pista, así que no se redecodifica en cada vuelta.
            std::thread::spawn(move || {
                let mut last = MediaState::default();
                loop {
                    let current = smtc::read_state();
                    // La carátula es un data URI de cientos de kB y cotejarla
                    // entera cuatro veces por segundo no aporta nada: se compara
                    // su longitud, que basta para notar que llegó la imagen
                    // (YouTube Music la publica después del título) o que
                    // cambió la del tema anterior por la nueva.
                    let changed = current.title != last.title
                        || current.artwork.len() != last.artwork.len()
                        || current.artist != last.artist
                        || current.playing != last.playing
                        || current.has_session != last.has_session
                        || current.can_seek != last.can_seek
                        || current.duration != last.duration
                        || (current.position - last.position).abs() > 0.08;

                    if changed {
                        // Con la misma carátula que el evento anterior no se reenvía (ver `artwork_kept`).
                        let mut payload = current.clone();
                        if !last.artwork.is_empty() && payload.artwork == last.artwork {
                            payload.artwork.clear();
                            payload.artwork_kept = true;
                        }
                        let _ = handle.emit(MEDIA_EVENT, payload);
                        last = current;
                    }
                    // 250 ms: la letra de karaoke necesita más resolución que
                    // el segundo que tarda SMTC en refrescar su posición.
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
            });

            // Alimentación: Windows no ofrece un evento sencillo fuera del
            // bucle de mensajes de una ventana, así que se sondea. Dos
            // segundos bastan para un aviso de cargador y el coste es una
            // llamada a `GetSystemPowerStatus`, sin COM ni asignaciones.
            let power_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last = power::read_state();
                let mut warned = u8::MAX;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    let current = power::read_state();
                    // Solo interesa enchufar o desenchufar. El porcentaje
                    // cambia solo cada varios minutos y no justifica despertar
                    // al frontend ni relanzar un aviso ya mostrado.
                    // El ajuste se consulta aquí, en cada vuelta, y no al
                    // montar el hilo: así desactivar los avisos surte efecto
                    // en caliente, sin reiniciar la isla.
                    // Cada tipo tiene su interruptor además del general.
                    let config = settings::current();
                    if current.plugged != last.plugged && config.notifications && config.notify_charger {
                        let _ = power_handle.emit(POWER_EVENT, current.clone());
                    }

                    // Batería baja: se avisa una vez al cruzar cada umbral, no
                    // en cada vuelta mientras siga por debajo. `warned` guarda
                    // el umbral más bajo ya anunciado; enchufar o recuperar
                    // carga lo vuelve a armar.
                    if current.has_battery && !current.plugged {
                        let crossed = power::LOW_BATTERY_THRESHOLDS
                            .into_iter()
                            .find(|threshold| current.percent <= *threshold);
                        if let Some(threshold) = crossed {
                            if warned > threshold && config.notifications && config.notify_low_battery {
                                warned = threshold;
                                let _ = power_handle.emit(LOW_BATTERY_EVENT, current.clone());
                            }
                        }
                    }
                    if current.plugged || current.percent > power::LOW_BATTERY_REARM {
                        warned = u8::MAX;
                    }
                    last = current;
                }
            });

            // Red: la API WLAN tiene notificaciones, pero exigen callbacks
            // sobre una DLL cargada; con una consulta cada 3 s basta y no
            // despierta al frontend salvo ante un cambio. El estado al
            // arrancar es la línea base y no avisa.
            let wifi_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last = network::read_state().ssid;
                let mut online = network::has_internet();
                // Lecturas seguidas sin internet: una sola puede ser un bache
                // de NCSI al cambiar de red, y no merece un aviso.
                let mut offline_reads = 0u32;
                let mut announced_wifi_at: Option<std::time::Instant> = None;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(3000));
                    let current = network::read_state();
                    // Los ajustes se consultan en cada vuelta para que
                    // apagar el aviso surta efecto en caliente.
                    let config = settings::current();
                    if current.ssid.is_some()
                        && current.ssid != last
                        && config.notifications
                        && config.notify_wifi
                    {
                        announced_wifi_at = Some(std::time::Instant::now());
                        let _ = wifi_handle.emit(WIFI_EVENT, current.clone());
                    }
                    last = current.ssid;

                    let now_online = network::has_internet();
                    offline_reads = if now_online { 0 } else { offline_reads + 1 };
                    let lost = online && offline_reads >= 2;
                    // Al conectarse a una red el internet «vuelve» solo: ya
                    // lo anuncia el aviso de Wi-Fi, y repetirlo seguido sería
                    // ruido. Solo se anuncia la recuperación sin cambio de red.
                    let recovered = !online
                        && now_online
                        && announced_wifi_at.is_none_or(|at| at.elapsed().as_secs() > 20);
                    if (lost || recovered) && config.notifications && config.notify_internet {
                        let _ = wifi_handle.emit(INTERNET_EVENT, InternetState { online: now_online });
                    }
                    if lost {
                        online = false;
                    } else if now_online {
                        online = true;
                    }
                }
            });

            // Bluetooth: se compara la lista de dispositivos conectados cada 3 s.
            // La primera lectura es la línea base (no avisa de lo que ya estaba
            // conectado) y una lectura fallida se salta, no cuenta como «todo
            // desconectado».
            let bluetooth_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last = bluetooth::connected_names();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(3000));
                    let Some(current) = bluetooth::connected_names() else {
                        continue;
                    };
                    if let Some(before) = &last {
                        // Los ajustes se consultan en cada vuelta para que
                        // apagar el aviso surta efecto en caliente.
                        let config = settings::current();
                        if config.notifications && config.notify_bluetooth {
                            for change in bluetooth::diff(before, &current) {
                                let _ = bluetooth_handle.emit(BLUETOOTH_EVENT, change);
                            }
                        }
                    }
                    last = Some(current);
                }
            });

            // Avisos que se sondean en su propio módulo: unidades USB, teclas de
            // bloqueo, modo de concentración y brillo del portátil.
            start_gap_watch(app.handle().clone());
            audio::start(app.handle().clone());
            downloads::start(app.handle().clone());
            privacy::start(app.handle().clone());
            usb::start(app.handle().clone());
            keys::start(app.handle().clone());
            focus::start(app.handle().clone());
            brightness::start(app.handle().clone());

            // Notificaciones de otras aplicaciones (apagado por defecto).
            toasts::start(app.handle().clone());

            // Volumen: Core Audio puede avisar de cambios, pero eso exige un
            // objeto COM con su propio bucle de mensajes. Un sondeo corto es
            // más sencillo y basta: la tecla de volumen cambia el valor a
            // saltos, y 120 ms se perciben como inmediatos.
            let volume_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last = volume::read();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                    let current = volume::read();
                    // El ajuste se consulta en cada vuelta, como en el sondeo
                    // de alimentación, para que apagarlo surta efecto en caliente.
                    if current != last {
                        let config = settings::current();
                        if config.notifications && config.notify_volume {
                            let _ = volume_handle.emit(VOLUME_EVENT, current);
                        }
                    }
                    last = current;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Dynamic Island");
}

#[cfg(test)]
mod region_tests {
    use super::pill_spans;

    /// Dibuja la forma en ASCII para verla con `--nocapture`.
    fn dibujo(spans: &[(i32, i32)], from: i32, to: i32) -> String {
        spans
            .iter()
            .map(|&(a, b)| (from..to).map(|x| if x >= a && x < b { '#' } else { '.' }).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn la_forma_es_simetrica() {
        // Panel con muescas a 125 %: 330 × 108 lógicos, muesca 22, radio 24.
        let (l, r) = (60, 60 + 413);
        let spans = pill_spans(l, 0, r, 134, 30.0, 0.0, 27.5);
        for (y, &(a, b)) in spans.iter().enumerate() {
            // El centro de la forma es (l + r) / 2: lo que sobra a un lado sobra al otro.
            assert_eq!((a - l) + (b - r), 0, "fila {y} descentrada: {a}..{b}");
        }
    }

    #[test]
    fn los_escalones_nunca_retroceden() {
        // Abajo el tramo solo puede estrecharse y arriba, ensancharse: un escalón que
        // vuelve atrás es justo lo que se ve «mordido».
        let spans = pill_spans(60, 0, 473, 134, 30.0, 0.0, 27.5);
        let notch_rows = (27.5_f64).ceil() as usize;
        for w in spans[..notch_rows].windows(2) {
            assert!(w[1].0 >= w[0].0, "la muesca retrocede: {:?} -> {:?}", w[0], w[1]);
        }
        for w in spans[notch_rows..].windows(2) {
            assert!(w[1].0 >= w[0].0, "la esquina retrocede: {:?} -> {:?}", w[0], w[1]);
        }
    }

    #[test]
    fn dibuja_la_esquina() {
        let spans = pill_spans(30, 0, 150, 60, 20.0, 0.0, 14.0);
        println!("{}", dibujo(&spans, 10, 60));
    }
}

/// TLS de las peticiones salientes con el almacén de certificados de Windows, no con los raíz que trae la
/// biblioteca. En equipos de empresa un proxy o antivirus re-firma el tráfico con un certificado propio que
/// solo el almacén de Windows conoce: con los raíz incluidos, la conexión fallaba (y el tiempo, las letras o
/// el test de velocidad se quedaban mudos sin explicar por qué).
pub(crate) fn tls_config() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build()
}
