//! Brillo de la pantalla integrada (portátiles): aviso al cambiarlo.
//!
//! Se lee con `IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS` sobre `\\.\LCD`, el
//! dispositivo del panel del portátil. Un monitor externo no lo tiene (el brillo
//! va por DDC/CI, otro mecanismo), y un equipo de sobremesa tampoco: ahí `read`
//! devuelve `None` y no se avisa de nada.

use serde::Serialize;

/// Evento con el brillo nuevo cuando cambia.
pub const BRIGHTNESS_EVENT: &str = "brightness://changed";

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrightnessState {
    /// De 0 a 100.
    pub level: u8,
}

/// Brillo actual, o `None` si no hay panel integrado o no se puede leer.
#[cfg(target_os = "windows")]
pub fn read() -> Option<u8> {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::IO::DeviceIoControl;

    const IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS: u32 = 0x0023_0498;
    // DISPLAY_BRIGHTNESS { ucDisplayPolicy, ucACBrightness, ucDCBrightness }.
    const DISPLAYPOLICY_DC: u8 = 2;

    unsafe {
        let handle = CreateFileW(
            w!("\\\\.\\LCD"),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .ok()?;

        let mut out = [0u8; 3];
        let mut returned = 0u32;
        let result = DeviceIoControl(
            handle,
            IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS,
            None,
            0,
            Some(out.as_mut_ptr().cast()),
            out.len() as u32,
            Some(&mut returned),
            None,
        );
        let _ = CloseHandle(handle);
        result.ok()?;

        // El nivel vigente es el de la fuente de energía actual.
        let level = if crate::power::read_state().plugged || out[0] != DISPLAYPOLICY_DC {
            out[1]
        } else {
            out[2]
        };
        Some(level.min(100))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn read() -> Option<u8> {
    None
}

/// Arranca el sondeo. Sin panel integrado no hace nada. La primera lectura es la
/// línea base: arrancar no avisa.
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || {
        // Sin `\\.\LCD` no hay nada que vigilar; se reintenta poco por si el
        // panel aparece más tarde (por ejemplo al acoplar el portátil).
        let mut last = read();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(if last.is_some() { 250 } else { 5000 }));
            let Some(current) = read() else {
                last = None;
                continue;
            };
            if last.is_some_and(|before| before != current) {
                let config = crate::settings::current();
                if config.notifications && config.notify_brightness {
                    let _ = app.emit(BRIGHTNESS_EVENT, BrightnessState { level: current });
                }
            }
            last = Some(current);
        }
    });
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    /// Depende del hardware: `cargo test --lib brightness -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lee_el_brillo_del_panel() {
        println!("brillo = {:?}", super::read());
    }
}
