//! Modo No molestar / asistente de concentración: aviso al activarlo o apagarlo.
//!
//! Windows no documenta una API para saber si No molestar está activo.
//! `SHQueryUserNotificationState` es la consulta oficial del Shell sobre si el
//! usuario acepta avisos, y devuelve `QUNS_QUIET_TIME` cuando el modo de
//! concentración los está silenciando. Se ignoran a propósito los demás estados
//! (pantalla completa, presentación): no son un modo que el usuario active, y
//! avisar de ellos sería ruido cada vez que abre un juego o un vídeo.
//!
//! **Sin verificar en todas las versiones de Windows**: si la consulta no
//! distingue el modo en alguna, el aviso simplemente no llega.

use serde::Serialize;

/// Evento al activar o apagar el modo de concentración.
pub const FOCUS_EVENT: &str = "focus://changed";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusState {
    pub on: bool,
}

#[cfg(target_os = "windows")]
fn is_on() -> Option<bool> {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_QUIET_TIME};

    // Un fallo es «no se sabe», no «apagado»: quien llama se salta la vuelta.
    let state = unsafe { SHQueryUserNotificationState() }.ok()?;
    Some(state == QUNS_QUIET_TIME)
}

#[cfg(target_os = "windows")]
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || {
        let mut last = is_on();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let Some(current) = is_on() else { continue };
            if last.is_some_and(|before| before != current) {
                let config = crate::settings::current();
                if config.notifications && config.notify_focus {
                    let _ = app.emit(FOCUS_EVENT, FocusState { on: current });
                }
            }
            last = Some(current);
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_app: tauri::AppHandle) {}
