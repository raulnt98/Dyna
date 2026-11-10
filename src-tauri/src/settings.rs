//! Ajustes de la isla: persistencia en disco y aplicación en caliente.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Alineación horizontal de la isla en la pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Left,
    Center,
    Right,
}

/// Preferencias que el usuario puede cambiar desde el panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub position: Position,
    pub top_margin: f64,
    pub always_on_top: bool,
    pub expand_on_hover: bool,
    pub hide_when_idle: bool,
    pub autostart: bool,
    /// Buscar letras sincronizadas en LRCLIB (envía título y artista).
    pub lyrics: bool,
    /// Segundos que saltan los botones de avance y retroceso.
    pub skip_seconds: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            position: Position::Center,
            // La isla cuelga del borde superior: por defecto no se despega.
            top_margin: 0.0,
            always_on_top: true,
            expand_on_hover: false,
            hide_when_idle: false,
            autostart: false,
            lyrics: true,
            skip_seconds: 10.0,
        }
    }
}

/// Ajustes vigentes, compartidos entre la bandeja, la ventana y el sondeo.
pub static CURRENT: Mutex<Option<Settings>> = Mutex::new(None);

/// Devuelve los ajustes en memoria, cargándolos del disco la primera vez.
pub fn current() -> Settings {
    if let Ok(mut guard) = CURRENT.lock() {
        if let Some(settings) = guard.as_ref() {
            return settings.clone();
        }
        let loaded = load();
        *guard = Some(loaded.clone());
        return loaded;
    }
    Settings::default()
}

/// Guarda los ajustes en memoria y en disco.
pub fn store(settings: &Settings) {
    if let Ok(mut guard) = CURRENT.lock() {
        *guard = Some(settings.clone());
    }
    save(settings);
}

/// Ruta del archivo de ajustes, junto a los datos del usuario.
fn path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    let dir = std::path::PathBuf::from(base).join("DynamicIsland");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("settings.json"))
}

/// Lee los ajustes del disco; ante cualquier fallo, usa los valores por defecto.
fn load() -> Settings {
    let Some(path) = path() else {
        return Settings::default();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Escribe los ajustes en disco. Un fallo aquí no debe interrumpir la app.
fn save(settings: &Settings) {
    let Some(path) = path() else {
        return;
    };
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(path, text);
    }
}

/// Clave del registro donde Windows guarda los programas de inicio.
#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Activa o desactiva el arranque con Windows para el usuario actual.
///
/// Se escribe en la clave `Run` del usuario, que no requiere privilegios de
/// administrador. Devuelve `false` si no se pudo aplicar.
#[cfg(target_os = "windows")]
pub fn set_autostart(enabled: bool) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let value = format!("\"{}\"", exe.display());

    // Se usa reg.exe para no añadir una dependencia solo por esto.
    let output = if enabled {
        std::process::Command::new("reg")
            .args([
                "add",
                &format!(r"HKCU\{RUN_KEY}"),
                "/v",
                "DynamicIsland",
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .output()
    } else {
        std::process::Command::new("reg")
            .args([
                "delete",
                &format!(r"HKCU\{RUN_KEY}"),
                "/v",
                "DynamicIsland",
                "/f",
            ])
            .output()
    };

    output.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn set_autostart(_enabled: bool) -> bool {
    false
}
