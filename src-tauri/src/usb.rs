//! Unidades extraíbles (memorias USB, tarjetas SD): aviso al conectar o retirar.
//!
//! Se comparan cada 2 s las letras de unidad con un volumen montado. Es más
//! simple que registrar notificaciones de dispositivo, que exigen una ventana con
//! su bucle de mensajes, y basta para un aviso.
//!
//! Limitación: Windows presenta como «fijos» los discos duros externos USB, y
//! solo las unidades «extraíbles» (memorias, lectores de tarjetas) se detectan.
//!
//! Solo se lee la letra y la etiqueta del volumen; no se toca su contenido.

use serde::Serialize;

/// Evento al conectar o retirar una unidad extraíble.
pub const USB_EVENT: &str = "usb://changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbChange {
    /// Etiqueta del volumen; vacía si no tiene.
    pub name: String,
    /// Letra de la unidad, sin dos puntos («E»).
    pub letter: String,
    pub connected: bool,
}

/// Unidad extraíble con un volumen montado: letra y etiqueta.
type Drive = (char, String);

/// Unidades extraíbles que tienen medio dentro, por orden de letra.
///
/// Un lector de tarjetas vacío ya figura como unidad extraíble: solo cuenta
/// cuando hay volumen, porque es la tarjeta lo que se inserta.
#[cfg(target_os = "windows")]
fn removable_drives() -> Vec<Drive> {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW};

    const DRIVE_REMOVABLE: u32 = 2;

    let mask = unsafe { GetLogicalDrives() };
    let mut drives = Vec::new();
    // A y B son disqueteras: ni se consultan.
    for index in 2..26u32 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let letter = (b'A' + index as u8) as char;
        let root = HSTRING::from(format!("{letter}:\\"));
        if unsafe { GetDriveTypeW(&root) } != DRIVE_REMOVABLE {
            continue;
        }
        let mut label = [0u16; 64];
        let mounted = unsafe { GetVolumeInformationW(&root, Some(&mut label), None, None, None, None) };
        if mounted.is_err() {
            continue;
        }
        let end = label.iter().position(|&c| c == 0).unwrap_or(label.len());
        drives.push((letter, String::from_utf16_lossy(&label[..end])));
    }
    drives
}

#[cfg(not(target_os = "windows"))]
fn removable_drives() -> Vec<Drive> {
    Vec::new()
}

/// Cambios entre dos lecturas: primero las retiradas, luego las conexiones.
fn diff(before: &[Drive], after: &[Drive]) -> Vec<UsbChange> {
    let gone = before
        .iter()
        .filter(|(letter, _)| !after.iter().any(|(other, _)| other == letter))
        .map(|(letter, name)| (letter, name, false));
    let new = after
        .iter()
        .filter(|(letter, _)| !before.iter().any(|(other, _)| other == letter))
        .map(|(letter, name)| (letter, name, true));
    gone.chain(new)
        .map(|(letter, name, connected)| UsbChange {
            name: name.clone(),
            letter: letter.to_string(),
            connected,
        })
        .collect()
}

/// Arranca el sondeo. La lectura inicial es la línea base: lo que ya estaba
/// conectado no avisa.
#[cfg(target_os = "windows")]
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;
    use windows::Win32::System::Diagnostics::Debug::{SetThreadErrorMode, SEM_FAILCRITICALERRORS};

    std::thread::spawn(move || {
        // Sin esto, consultar una unidad sin medio puede abrir el diálogo «Inserte
        // un disco». Solo afecta a este hilo.
        unsafe {
            let _ = SetThreadErrorMode(SEM_FAILCRITICALERRORS, None);
        }
        let mut last = removable_drives();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(2000));
            let current = removable_drives();
            // Los ajustes se consultan en cada vuelta: apagar el aviso surte
            // efecto en caliente.
            let config = crate::settings::current();
            if config.notifications && config.notify_usb {
                for change in diff(&last, &current) {
                    let _ = app.emit(USB_EVENT, change);
                }
            }
            last = current;
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_app: tauri::AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_conexion_y_retirada_por_letra() {
        let before = vec![('E', "Fotos".to_string())];
        let after = vec![('F', String::new())];
        let changes = diff(&before, &after);
        assert_eq!(changes.len(), 2);
        assert_eq!((changes[0].letter.as_str(), changes[0].connected), ("E", false));
        assert_eq!(changes[0].name, "Fotos");
        assert_eq!((changes[1].letter.as_str(), changes[1].connected), ("F", true));
    }

    #[test]
    fn sin_cambios_no_avisa() {
        let drives = vec![('E', "Fotos".to_string())];
        assert!(diff(&drives, &drives).is_empty());
    }
}
