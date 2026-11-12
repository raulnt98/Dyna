mod lyrics;
mod power;
mod settings;
mod smtc;
mod toasts;
mod volume;

use settings::{Position, Settings};
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

/// Forma de la isla en un instante: lo que el recorte de la ventana dibuja.
///
/// Además del tamaño lleva la muesca y el radio, porque en el CSS también
/// cambian con el estado (14 px y 18 compacta, 22 y 24 expandida) y se animan
/// con la misma curva que el ancho y el alto.
#[derive(Clone, Copy, PartialEq)]
struct Shape {
    width: f64,
    height: f64,
    notch: f64,
    radius: f64,
}

impl Shape {
    /// Forma de reposo para un tamaño de isla. La altura distingue el estado:
    /// la pestaña mínima mide 8 px, la píldora compacta 34 y cualquier panel
    /// pasa de 40. Las medidas de muesca y radio replican las del CSS
    /// (`--notch` en IslandApp.vue y los `rounded-[…]` de TheIsland.vue).
    fn for_size(width: f64, height: f64) -> Shape {
        let (notch, radius) = if height > 40.0 {
            (22.0, 24.0)
        } else if height > 12.0 {
            (14.0, 18.0)
        } else {
            // Pestaña: una muesca de 14 px colgaría por debajo de ella.
            (5.0, 4.0)
        };
        Shape {
            width,
            height,
            notch,
            radius,
        }
    }

    fn lerp(from: Shape, to: Shape, t: f64) -> Shape {
        let mix = |a: f64, b: f64| a + (b - a) * t;
        Shape {
            width: mix(from.width, to.width),
            height: mix(from.height, to.height),
            notch: mix(from.notch, to.notch),
            radius: mix(from.radius, to.radius),
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
});

/// Generación de la animación en curso: al cambiar, el hilo anterior se retira.
static ANIM_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Duración del `.island-morph` del CSS, en segundos.
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
        CombineRgn, CreateEllipticRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject,
        SetWindowRgn, HGDIOBJ, RGN_DIFF, RGN_OR,
    };

    let to_px = |value: f64| (value * scale).round() as i32;
    let Shape { width, height, notch, radius } = *shape;
    let right = left + width;
    // `CreateRoundRectRgn` deja fuera su columna derecha, a diferencia de
    // `CreateRectRgn` (que usan las muescas): sin el +1 queda un hueco de un
    // píxel entre la píldora y la muesca derecha, que Windows pinta de claro
    // (línea vertical solo a la derecha). Un píxel hacia dentro en la base por
    // lo mismo; los costados van a ras.
    let region = CreateRoundRectRgn(
        to_px(left),
        to_px(-radius),
        to_px(right) + 1,
        to_px(height) - 1,
        to_px(radius * 2.0),
        to_px(radius * 2.0),
    );
    // Cada muesca es un cuadrado junto al costado, al que se le quita el
    // círculo centrado en su esquina inferior exterior (la máscara radial del CSS).
    for (box_left, circle_cx) in [(left - notch, left - notch), (right, right + notch)] {
        // Borde exterior de la muesca, tres píxeles hacia dentro: la forma
        // cóncava acaba en un filo de menos de un píxel de ancho que el CSS
        // pinta casi transparente pero la región incluye entero (punto blanco
        // a la izquierda, línea vertical a la derecha).
        let outer_left = if box_left < left { 3 } else { 0 };
        let outer_right = if box_left < left { 0 } else { 3 };
        let square = CreateRectRgn(
            to_px(box_left) + outer_left,
            0,
            to_px(box_left + notch) - outer_right,
            to_px(notch),
        );
        // El círculo es dos píxeles mayor que el del CSS: el borde cóncavo se
        // dibuja con antialiasing, y sin esta holgura quedan píxeles
        // semitransparentes dentro de la región que Windows pinta de claro
        // (puntos blancos junto a las muescas).
        let circle = CreateEllipticRgn(
            to_px(circle_cx - notch) - 2,
            -2,
            to_px(circle_cx + notch) + 2,
            to_px(notch * 2.0) + 2,
        );
        let _ = CombineRgn(Some(square), Some(square), Some(circle), RGN_DIFF);
        let _ = CombineRgn(Some(region), Some(region), Some(square), RGN_OR);
        let _ = DeleteObject(HGDIOBJ(square.0));
        let _ = DeleteObject(HGDIOBJ(circle.0));
    }
    // Windows se queda con la región al aceptarla: solo hay que liberarla si
    // la llamada falla.
    if SetWindowRgn(hwnd, Some(region), true) == 0 {
        let _ = DeleteObject(HGDIOBJ(region.0));
    }
}

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
fn animate_island(window: &WebviewWindow, width: f64, height: f64) {
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
            let t = (start.elapsed().as_secs_f64() / MORPH_SECS).min(1.0);
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

    // Se ancla al monitor principal: usar el monitor actual haría que la isla
    // saltara de pantalla según dónde quedara tras el último movimiento.
    let monitor = match window.primary_monitor() {
        Ok(Some(monitor)) => monitor,
        _ => match window.current_monitor() {
            Ok(Some(monitor)) => monitor,
            // Sin monitor conocido no hay forma de centrar: al menos se
            // ajusta el tamaño, aunque quede donde estaba.
            _ => {
                let _ = window.set_size(LogicalSize::new(win_w, win_h));
                return;
            }
        },
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
        use windows::Win32::Graphics::Gdi::{
            CreateRectRgn, DeleteObject, RedrawWindow, HGDIOBJ, RDW_ALLCHILDREN, RDW_INVALIDATE,
            RDW_UPDATENOW,
        };
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
                    {
                        use windows::Win32::Graphics::Dwm::{
                            DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE,
                            DWM_BLURBEHIND,
                        };
                        let corner = CreateRectRgn(0, 0, 1, 1);
                        let blur = DWM_BLURBEHIND {
                            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                            fEnable: true.into(),
                            hRgnBlur: corner,
                            fTransitionOnMaximized: false.into(),
                        };
                        let _ = DwmEnableBlurBehindWindow(hwnd, &blur);
                        let _ = DeleteObject(HGDIOBJ(corner.0));
                    }
                    let _ = RedrawWindow(
                        Some(hwnd),
                        None,
                        None,
                        RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW,
                    );
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

/// Ajusta la ventana al tamaño que pide la isla al expandirse o contraerse.
#[tauri::command]
fn island_resize(window: WebviewWindow, width: f64, height: f64) {
    animate_island(&window, width, height);
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
            power_state
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
                        let _ = handle.emit(MEDIA_EVENT, current.clone());
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
