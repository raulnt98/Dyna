//! Teclas de bloqueo (Bloq Mayús y Bloq Num): aviso al activarlas o apagarlas.
//!
//! `GetKeyState` devuelve el estado de la cola de entrada del hilo que pregunta.
//! Desde un hilo sin ventanas esa cola no recibe mensajes y el valor se queda
//! viejo, así que la lectura se hace en el hilo principal (el que dibuja la
//! ventana y siempre procesa entrada). El hilo del sondeo solo le encarga la
//! lectura cada 150 ms.

use serde::Serialize;

/// Evento al cambiar una tecla de bloqueo.
pub const KEYS_EVENT: &str = "keys://lock";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockKey {
    /// `"caps"` o `"num"`.
    pub key: &'static str,
    pub on: bool,
}

/// Último estado visto, `None` hasta la primera lectura (que es la línea base).
#[cfg(target_os = "windows")]
static LAST: std::sync::Mutex<Option<(bool, bool)>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
fn read() -> (bool, bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CAPITAL, VK_NUMLOCK};

    // El bit bajo de `GetKeyState` es el de «alternado» (la luz del teclado).
    let toggled = |vk: i32| unsafe { GetKeyState(vk) } & 1 != 0;
    (toggled(VK_CAPITAL.0 as i32), toggled(VK_NUMLOCK.0 as i32))
}

#[cfg(target_os = "windows")]
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(150));
        let emitter = app.clone();
        let _ = app.run_on_main_thread(move || {
            let (caps, num) = read();
            let Ok(mut last) = LAST.lock() else { return };
            let Some((was_caps, was_num)) = *last else {
                *last = Some((caps, num));
                return;
            };
            *last = Some((caps, num));
            // Se consulta en cada lectura para que apagarlo surta efecto en caliente.
            let config = crate::settings::current();
            if !(config.notifications && config.notify_lock_keys) {
                return;
            }
            if caps != was_caps {
                let _ = emitter.emit(KEYS_EVENT, LockKey { key: "caps", on: caps });
            }
            if num != was_num {
                let _ = emitter.emit(KEYS_EVENT, LockKey { key: "num", on: num });
            }
        });
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_app: tauri::AppHandle) {}
