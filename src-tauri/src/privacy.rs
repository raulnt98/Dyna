//! Micrófono y cámara en uso: qué aplicación los tiene abiertos ahora mismo.
//!
//! Windows apunta en el registro, por cada aplicación, cuándo empezó y cuándo dejó de usar
//! el micrófono y la cámara (`CapabilityAccessManager\ConsentStore`). Es lo mismo que
//! consulta el icono de privacidad de la barra de tareas. Una aplicación lo está usando si
//! tiene hora de inicio y su hora de fin es 0.
//!
//! Solo se lee el **nombre de la aplicación**, nunca audio ni vídeo, y no se guarda ni se
//! envía a ningún sitio: viaja de este hilo a la isla.

use serde::Serialize;

/// Evento al cambiar quién usa el micrófono o la cámara.
pub const PRIVACY_EVENT: &str = "privacy://changed";

/// Aplicación que usa cada dispositivo, o `None` si no lo usa nadie. Si hay varias, la
/// primera por orden alfabético: el aviso es una pista, no un inventario.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyState {
    pub microphone: Option<String>,
    pub camera: Option<String>,
}

/// Nombre legible de la aplicación a partir de la clave del registro.
///
/// Las de escritorio llevan su ruta con `#` en lugar de `\` (`C:#Program Files#Google#
/// Chrome#chrome.exe`) y las empaquetadas su identificador (`MSTeams_8wekyb3d8bbwe`,
/// `Microsoft.WindowsCamera_8wekyb3d8bbwe`).
pub fn app_name(key: &str) -> String {
    let raw = if key.contains('#') {
        let file = key.rsplit('#').next().unwrap_or(key);
        file.trim_end_matches(".exe").trim_end_matches(".EXE").to_string()
    } else {
        let id = key.split('_').next().unwrap_or(key);
        id.rsplit('.').next().unwrap_or(id).to_string()
    };
    // `WindowsCamera` → `Windows Camera`: se separa donde una minúscula da paso a una mayúscula.
    let mut out = String::with_capacity(raw.len() + 2);
    let mut previous: Option<char> = None;
    for c in raw.chars() {
        if c.is_uppercase() && previous.is_some_and(|p| p.is_lowercase()) {
            out.push(' ');
        }
        out.push(c);
        previous = Some(c);
    }
    let mut chars = out.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(target_os = "windows")]
fn in_use(kind: &str) -> Option<String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        RRF_RT_REG_QWORD,
    };

    // Instante (en 100 ns desde 1601, como las horas del registro) anterior al cual una
    // entrada abierta se da por abandonada.
    let stale_before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 / 100 + 116_444_736_000_000_000)
        .unwrap_or(u64::MAX)
        .saturating_sub(12 * 3600 * 10_000_000);
    let base = format!(r"Software\Microsoft\Windows\CapabilityAccessManager\ConsentStore\{kind}");
    // Las aplicaciones empaquetadas cuelgan de `base`; las de escritorio, de `NonPackaged`.
    let mut found: Vec<String> = Vec::new();

    for path in [base.clone(), format!(r"{base}\NonPackaged")] {
        unsafe {
            let mut key = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path.as_str()), None, KEY_READ, &mut key) != ERROR_SUCCESS {
                continue;
            }
            let mut index = 0u32;
            loop {
                let mut name = [0u16; 512];
                let mut len = name.len() as u32;
                let status = RegEnumKeyExW(key, index, Some(windows::core::PWSTR(name.as_mut_ptr())), &mut len, None, None, None, None);
                if status != ERROR_SUCCESS {
                    break;
                }
                index += 1;
                let app = String::from_utf16_lossy(&name[..len as usize]);
                if app == "NonPackaged" {
                    continue;
                }
                let sub = HSTRING::from(app.as_str());
                let read = |value: PCWSTR| -> Option<u64> {
                    let mut out = 0u64;
                    let mut size = std::mem::size_of::<u64>() as u32;
                    let status = RegGetValueW(
                        key,
                        &sub,
                        value,
                        RRF_RT_REG_QWORD,
                        None,
                        Some(&mut out as *mut u64 as *mut _),
                        Some(&mut size),
                    );
                    (status == ERROR_SUCCESS).then_some(out)
                };
                let start = read(w!("LastUsedTimeStart")).unwrap_or(0);
                let stop = read(w!("LastUsedTimeStop")).unwrap_or(0);
                // Una entrada abierta desde hace más de 12 h es el resto de un cierre brusco
                // (la aplicación murió sin anotar la hora de fin), no una llamada en curso.
                if start > 0 && stop == 0 && start > stale_before {
                    found.push(app_name(&app));
                }
            }
            let _ = RegCloseKey(key);
        }
    }
    found.sort();
    found.dedup();
    found.into_iter().next()
}

#[cfg(target_os = "windows")]
pub fn read_state() -> PrivacyState {
    PrivacyState {
        microphone: in_use("microphone"),
        camera: in_use("webcam"),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn read_state() -> PrivacyState {
    PrivacyState::default()
}

/// Arranca el sondeo. Emite el estado al arrancar (puede haber algo en uso ya) y después
/// solo cuando cambia. El ajuste se consulta en cada vuelta: apagarlo surte efecto en caliente.
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || {
        let mut last = PrivacyState::default();
        loop {
            let config = crate::settings::current();
            let current = if config.notifications && config.notify_privacy {
                read_state()
            } else {
                PrivacyState::default()
            };
            if current != last {
                let _ = app.emit(PRIVACY_EVENT, current.clone());
                last = current;
            }
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::app_name;

    #[test]
    fn nombra_las_aplicaciones_empaquetadas() {
        assert_eq!(app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe"), "Windows Camera");
        assert_eq!(app_name("MSTeams_8wekyb3d8bbwe"), "MSTeams");
    }

    #[test]
    fn nombra_las_de_escritorio_por_su_ejecutable() {
        assert_eq!(app_name("C:#Program Files#Google#Chrome#Application#chrome.exe"), "Chrome");
        assert_eq!(app_name("C:#Users#a#AppData#Local#Discord#Update.exe"), "Update");
    }

    /// Lee el registro real: `cargo test --lib privacy -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn estado_real() {
        println!("{:?}", super::read_state());
    }
}
