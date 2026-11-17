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

/// Cómo se ve la isla cuando está cerrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollapsedStyle {
    /// La píldora compacta, con carátula y ondas.
    #[default]
    Pill,
    /// Una pestaña mínima pegada al borde: casi no tapa nada de lo que hay debajo.
    Tab,
}

/// Estilo de la isla: dos «packs» completos que valen para la isla cerrada y para la abierta a la vez.
///
/// Mismo tamaño en los dos: solo cambian los bordes de arriba y si la isla cuelga del borde o flota.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CornerStyle {
    /// Con muescas (esquinas invertidas): la isla cuelga del borde de la pantalla y se funde con él, cerrada
    /// (píldora o pestaña) y abierta.
    #[default]
    Notch,
    /// Flotante redondeada: sin muescas, las esquinas de arriba redondeadas y la isla unos píxeles por debajo
    /// del borde, cerrada y abierta (la pestaña mínima, que es solo una lengüeta, sigue pegada).
    Round,
}

/// Una aplicación detectada y si el usuario quiere ver sus notificaciones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationApp {
    /// Nombre tal como lo informa Windows («Microsoft Teams»).
    pub name: String,
    pub enabled: bool,
}

/// Preferencias que el usuario puede cambiar desde el panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub position: Position,
    pub top_margin: f64,
    pub always_on_top: bool,
    /// Aspecto de la isla cerrada: píldora o pestaña mínima.
    pub collapsed_style: CollapsedStyle,
    /// Estilo de la isla, cerrada y abierta: con muescas o flotante redondeada. Antes se llamaba
    /// `expandedStyle` y solo valía para la abierta: el alias deja leer un archivo antiguo.
    #[serde(alias = "expandedStyle")]
    pub island_style: CornerStyle,
    /// Material de la isla: cristal líquido (translúcido, con desenfoque del escritorio) en lugar de negro sólido.
    /// Solo vale con el estilo flotante: con muescas, las esquinas invertidas no casan con un material translúcido.
    pub glass: bool,
    /// Avisar cuando una aplicación usa el micrófono o la cámara.
    pub notify_privacy: bool,
    /// Avisar de las descargas del navegador (inicio y fin, con la velocidad).
    pub notify_downloads: bool,
    /// Que la onda de la isla siga el sonido real del sistema. Captura el audio de salida
    /// solo para medir su nivel (nada se guarda ni se envía), pero es sensible: nace apagado.
    pub audio_visualizer: bool,
    /// Mostrar el tiempo en la vista de inicio. Envía la ciudad a Open-Meteo: nace apagado.
    pub weather: bool,
    /// Ciudad para el tiempo, tal como la escribe el usuario.
    pub weather_city: String,
    /// Coordenadas de la ciudad elegida de la lista o de la ubicación actual; sin ellas se busca por nombre.
    pub weather_lat: Option<f64>,
    pub weather_lon: Option<f64>,
    /// Mostrar el botón de test de velocidad en la vista de inicio. Solo mide al pulsarlo.
    pub speedtest: bool,
    /// Pantalla donde cuelga la isla: 0 es la principal; n ≥ 1, la n-ésima que informe Windows.
    pub monitor: u32,
    pub autostart: bool,
    /// Avisar de sucesos del sistema, como conectar el cargador.
    pub notifications: bool,
    /// Avisar al conectar o desconectar el cargador. Solo cuenta si
    /// `notifications` está activo, que es el interruptor general.
    pub notify_charger: bool,
    /// Avisar de los cambios de volumen y del silencio.
    pub notify_volume: bool,
    /// Avisar al bajar al 20 % y al 10 % de batería.
    pub notify_low_battery: bool,
    /// Avisar al conectarse a una red Wi-Fi.
    pub notify_wifi: bool,
    /// Avisar al perder o recuperar la salida a internet.
    pub notify_internet: bool,
    /// Avisar al conectar o desconectar un dispositivo Bluetooth.
    pub notify_bluetooth: bool,
    /// Presentar la canción al empezar a sonar o al cambiar de pista.
    pub notify_track: bool,
    /// Abrir la isla al pasar el ratón por encima, además de con el clic.
    pub hover_open: bool,
    /// Avisar al conectar o retirar una unidad USB extraíble.
    pub notify_usb: bool,
    /// Avisar al activar o apagar Bloq Mayús y Bloq Num.
    pub notify_lock_keys: bool,
    /// Avisar al activar o apagar el modo No molestar.
    pub notify_focus: bool,
    /// Avisar al cambiar el brillo de la pantalla integrada.
    pub notify_brightness: bool,
    /// Mostrar el uso de CPU y memoria en la vista de inicio.
    pub home_stats: bool,
    /// Página «Baterías» del carrusel: portátil y dispositivos Bluetooth conectados que informen su carga.
    pub home_batteries: bool,
    /// Buscar letras sincronizadas en LRCLIB (envía título y artista).
    pub lyrics: bool,
    /// Mostrar en la isla las notificaciones de otras aplicaciones (Teams,
    /// Outlook…). Lee contenido privado de Windows, aunque no sale del equipo:
    /// por eso nace apagado.
    pub app_notifications: bool,
    /// Aplicaciones que la isla ha visto enviar notificaciones, cada una con su
    /// interruptor. La lista la rellena la propia app al detectarlas (ver
    /// `toasts.rs`); el usuario solo activa las que quiere.
    pub notification_apps: Vec<NotificationApp>,
    /// Segundos que saltan los botones de avance y retroceso.
    pub skip_seconds: f64,
    /// Mostrar los segundos en la hora de la vista de inicio (la que sale cuando
    /// no hay ningún medio).
    pub home_seconds: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            position: Position::Center,
            // La isla cuelga del borde superior: por defecto no se despega.
            top_margin: 0.0,
            always_on_top: true,
            collapsed_style: CollapsedStyle::Pill,
            island_style: CornerStyle::Notch,
            glass: false,
            notify_privacy: true,
            notify_downloads: true,
            audio_visualizer: false,
            weather: false,
            weather_city: String::new(),
            weather_lat: None,
            weather_lon: None,
            speedtest: true,
            monitor: 0,
            autostart: false,
            notifications: true,
            notify_charger: true,
            notify_volume: true,
            notify_low_battery: true,
            notify_wifi: true,
            notify_internet: true,
            notify_bluetooth: true,
            notify_track: true,
            hover_open: true,
            notify_usb: true,
            notify_lock_keys: true,
            notify_focus: true,
            notify_brightness: true,
            home_stats: true,
            home_batteries: true,
            lyrics: true,
            app_notifications: false,
            notification_apps: Vec::new(),
            skip_seconds: 10.0,
            home_seconds: true,
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

/// La isla va siempre **centrada y pegada al borde superior**: no es un ajuste. Los campos `position`
/// y `top_margin` siguen en el archivo (para que uno antiguo se lea sin errores), pero aquí se fuerzan
/// a su valor fijo al leer y al guardar: un `settings.json` viejo con la isla a un lado no puede dejarla
/// descentrada sin forma de arreglarlo desde el panel.
fn pinned(mut settings: Settings) -> Settings {
    settings.position = Position::Center;
    settings.top_margin = 0.0;
    settings
}

/// Guarda los ajustes en memoria y en disco.
pub fn store(settings: &Settings) {
    let settings = pinned(settings.clone());
    if let Ok(mut guard) = CURRENT.lock() {
        *guard = Some(settings.clone());
    }
    save(&settings);
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
    serde_json::from_str(&text).map(pinned).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_archivo_antiguo_no_puede_descentrar_la_isla() {
        // Como lo dejaba la versión que sí tenía selector: isla a la izquierda y separada del borde.
        let old = r#"{"position":"left","topMargin":40.0,"alwaysOnTop":false}"#;
        let loaded: Settings = serde_json::from_str(old).map(pinned).unwrap();
        assert_eq!(loaded.position, Position::Center);
        assert_eq!(loaded.top_margin, 0.0);
        // Y lo demás del archivo se respeta.
        assert!(!loaded.always_on_top);
    }

    #[test]
    fn el_estilo_antiguo_se_sigue_leyendo() {
        // Antes el ajuste se llamaba `expandedStyle` y solo valía para la isla abierta: quien lo tenía en
        // «redondeada» no debe perderlo al actualizar.
        let old: Settings = serde_json::from_str(r#"{"expandedStyle":"round"}"#).unwrap();
        assert_eq!(old.island_style, CornerStyle::Round);
        let new: Settings = serde_json::from_str(r#"{"islandStyle":"round"}"#).unwrap();
        assert_eq!(new.island_style, CornerStyle::Round);
        // Y sin nada, con muescas.
        let none: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(none.island_style, CornerStyle::Notch);
    }
}
