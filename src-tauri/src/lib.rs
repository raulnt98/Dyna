mod lyrics;
mod power;
mod settings;
mod smtc;
mod volume;

use settings::{Position, Settings};
use power::POWER_EVENT;
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

/// Coloca la ventana centrada en el borde superior de la pantalla activa.
///
/// `width` y `height` son el tamaño de la isla; la ventana añade el margen
/// necesario alrededor.
fn place_island(window: &WebviewWindow, width: f64, height: f64) {
    // La isla cuelga del borde superior: arriba no lleva holgura, porque su
    // primer píxel coincide con el de la pantalla. Abajo sí, para la sombra.
    let win_w = width + (WINDOW_PADDING + OVERSHOOT_MARGIN) * 2.0;
    let win_h = height + WINDOW_PADDING;

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

    // La alineación y el margen salen de los ajustes del usuario. El relleno
    // de la ventana ya forma parte de la separación visible.
    let config = settings::current();
    let x = match config.position {
        Position::Left => origin.x + 24.0,
        Position::Center => origin.x + (screen.width - win_w) / 2.0,
        Position::Right => origin.x + screen.width - win_w - 24.0,
    };
    // Colgando del borde, la ventana arranca en el canto de la pantalla. El
    // ajuste de separación todavía permite despegarla, pero su valor por
    // defecto es 0: la forma supone que la isla nace del marco.
    let y = origin.y + config.top_margin.max(0.0);

    // Mover y redimensionar en una sola operación.
    //
    // `set_size` y `set_position` son dos llamadas Win32 independientes: entre
    // una y otra el compositor llega a dibujar un fotograma con el tamaño
    // nuevo y la posición vieja. Como la isla va centrada, expandirla de 116 a
    // 330 px desplaza su X más de cien píxeles, y ese paso intermedio se ve
    // como un tirón lateral. `SetWindowPos` aplica ambas cosas de golpe.
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
            let ok = unsafe {
                SetWindowPos(
                    HWND(handle.0 as _),
                    None,
                    to_px(x),
                    to_px(y),
                    to_px(win_w),
                    to_px(win_h),
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                )
            };
            if ok.is_ok() {
                return;
            }
        }
    }

    // Fuera de Windows, o si `SetWindowPos` falla, se recurre a las dos
    // llamadas por separado.
    let _ = window.set_size(LogicalSize::new(win_w, win_h));
    let _ = window.set_position(LogicalPosition::new(x, y));
}

/// Ajusta la ventana al tamaño que pide la isla al expandirse o contraerse.
#[tauri::command]
fn island_resize(window: WebviewWindow, width: f64, height: f64) {
    place_island(&window, width, height);
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
#[tauri::command]
fn lyrics_current() -> lyrics::Lyrics {
    if !settings::current().lyrics {
        return lyrics::Lyrics::default();
    }
    let state = smtc::read_state();
    if !state.has_session {
        return lyrics::Lyrics::default();
    }
    lyrics::for_track(&state.title, &state.artist, &state.album, state.duration)
}

/// Estado actual de la batería y la corriente.
///
/// La isla lo pide al arrancar: el sondeo solo avisa de los cambios, así que
/// sin esto no sabría si ya estaba enchufado.
#[tauri::command]
fn power_state() -> power::PowerState {
    power::read_state()
}

/// Volumen maestro del sistema, de 0 a 100, y si está silenciado.
#[tauri::command]
fn volume_get() -> (u8, bool) {
    (volume::get(), volume::muted())
}

/// Ajusta el volumen maestro del sistema.
#[tauri::command]
fn volume_set(value: u8) -> bool {
    volume::set(value)
}

/// Silencia o restaura el audio del sistema.
#[tauri::command]
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
        // Se reposiciona con su tamaño actual, que la ventana ya conoce.
        if let Ok(size) = island.outer_size() {
            let scale = island.scale_factor().unwrap_or(1.0);
            let logical = size.to_logical::<f64>(scale);
            place_island(
                &island,
                logical.width - WINDOW_PADDING * 2.0,
                logical.height - WINDOW_PADDING * 2.0,
            );
        }
        // La isla necesita saber los ajustes para el hover y el modo oculto.
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
    .inner_size(420.0, 560.0)
    .resizable(false)
    .center()
    .build();
}

/// Indica si el cursor está sobre la ventana de la isla.
///
/// Cuando la ventana ignora el ratón no recibe eventos de hover, así que el
/// frontend consulta la posición del cursor para saber cuándo reaccionar.
#[tauri::command]
fn island_cursor_over(window: WebviewWindow) -> bool {
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return false;
    };
    let Ok(cursor) = window.cursor_position() else {
        return false;
    };

    // Solo la franja central interesa: es donde se dibuja la isla, no el
    // margen transparente que la rodea.
    let padding = WINDOW_PADDING * window.scale_factor().unwrap_or(1.0);
    let left = pos.x as f64 + padding;
    let right = pos.x as f64 + size.width as f64 - padding;
    let top = pos.y as f64 + padding;
    let bottom = pos.y as f64 + size.height as f64 - padding;

    cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom
}

/// Devuelve el estado multimedia actual del sistema.
#[tauri::command]
fn media_state() -> MediaState {
    smtc::read_state()
}

/// Alterna reproducir/pausar en el reproductor activo.
#[tauri::command]
fn media_play_pause() -> bool {
    smtc::control(Command::PlayPause)
}

/// Salta a la pista siguiente.
#[tauri::command]
fn media_next() -> bool {
    smtc::control(Command::Next)
}

/// Vuelve a la pista anterior.
#[tauri::command]
fn media_previous() -> bool {
    smtc::control(Command::Previous)
}

/// Salta a una posición de la pista actual, en segundos desde el inicio.
#[tauri::command]
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
            island_cursor_over,
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
                    // Se compara sin la carátula: es un data URI de cientos de
                    // kB y cotejarlo cuatro veces por segundo no aporta nada,
                    // porque solo cambia cuando cambia el título.
                    let changed = current.title != last.title
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
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    let current = power::read_state();
                    // Solo interesa enchufar o desenchufar. El porcentaje
                    // cambia solo cada varios minutos y no justifica despertar
                    // al frontend ni relanzar un aviso ya mostrado.
                    // El ajuste se consulta aquí, en cada vuelta, y no al
                    // montar el hilo: así desactivar los avisos surte efecto
                    // en caliente, sin reiniciar la isla.
                    if current.plugged != last.plugged && settings::current().notifications {
                        let _ = power_handle.emit(POWER_EVENT, current.clone());
                    }
                    last = current;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Dynamic Island");
}
