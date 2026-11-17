//! Tiempo actual para la vista de inicio, de Open-Meteo.
//!
//! **Sale a la red**, así que tiene su propio ajuste (`weather`, apagado por defecto) y su
//! línea en el README. Se envía a Open-Meteo solo la ciudad que el usuario escribió en los
//! ajustes (para buscar sus coordenadas) y esas coordenadas (para pedir el tiempo): no se usa
//! la ubicación del equipo ni se manda nada más. El servicio no pide clave ni cuenta.
//!
//! Los servidores son los de `GEOCODING` y `FORECAST`, fijos: la ciudad solo va como
//! parámetro de consulta, nunca como parte de la dirección. Sin redirecciones, con tiempo
//! límite, y el resultado se recuerda unos minutos para no consultar en cada apertura.

use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const GEOCODING: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";

/// Cuánto se reutiliza una respuesta.
const CACHE_FOR: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    /// Nombre de la ciudad tal como la devuelve el servicio.
    pub city: String,
    /// Temperatura en °C, redondeada.
    pub temp: i32,
    /// Descripción corta en español, de unos 14 caracteres como mucho: va debajo del icono, en
    /// el hueco de la fecha, y con segundos en la hora no cabe más.
    pub text: String,
    /// Máxima y mínima de hoy en °C, si el servicio las da.
    pub max: Option<i32>,
    pub min: Option<i32>,
    /// Tipo de icono: `sun`, `moon`, `cloud`, `rain`, `snow`, `storm` o `fog`.
    pub kind: String,
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(4)))
            .timeout_global(Some(Duration::from_secs(8)))
            .max_redirects(0)
            .build();
        ureq::Agent::new_with_config(config)
    })
}

/// Descripción y tipo de icono para un código WMO de Open-Meteo.
pub fn describe(code: u32, is_day: bool) -> (&'static str, &'static str) {
    match code {
        0 => (if is_day { "Despejado" } else { "Noche clara" }, if is_day { "sun" } else { "moon" }),
        1 => ("Poco nublado", if is_day { "sun" } else { "moon" }),
        2 => ("Nubes y claros", "cloud"),
        3 => ("Nublado", "cloud"),
        45 | 48 => ("Niebla", "fog"),
        51 | 53 | 55 | 56 | 57 => ("Llovizna", "rain"),
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => ("Lluvia", "rain"),
        71 | 73 | 75 | 77 | 85 | 86 => ("Nieve", "snow"),
        95 | 96 | 99 => ("Tormenta", "storm"),
        _ => ("Variable", "cloud"),
    }
}

/// Coordenadas de la ciudad, recordadas por nombre: no cambian entre consultas.
static PLACES: Mutex<Vec<(String, f64, f64, String)>> = Mutex::new(Vec::new());
/// Último resultado: ciudad, cuándo, y el tiempo.
static LAST: Mutex<Option<(String, Instant, Weather)>> = Mutex::new(None);

fn locate(city: &str) -> Option<(f64, f64, String)> {
    let key = city.trim().to_lowercase();
    if let Some(hit) = PLACES.lock().ok()?.iter().find(|(k, ..)| *k == key) {
        return Some((hit.1, hit.2, hit.3.clone()));
    }
    let mut response = agent()
        .get(GEOCODING)
        .query("name", city.trim())
        .query("count", "1")
        .query("language", "es")
        .call()
        .ok()?;
    let value: serde_json::Value = response.body_mut().read_json().ok()?;
    let first = value.get("results")?.get(0)?;
    let lat = first.get("latitude")?.as_f64()?;
    let lon = first.get("longitude")?.as_f64()?;
    let name = first.get("name")?.as_str()?.to_string();
    if let Ok(mut places) = PLACES.lock() {
        if places.len() > 8 {
            places.remove(0);
        }
        places.push((key, lat, lon, name.clone()));
    }
    Some((lat, lon, name))
}

fn fetch(city: &str) -> Option<Weather> {
    let (lat, lon, name) = locate(city)?;
    let mut response = agent()
        .get(FORECAST)
        .query("latitude", format!("{lat:.4}"))
        .query("longitude", format!("{lon:.4}"))
        .query("current", "temperature_2m,weather_code,is_day")
        .query("daily", "temperature_2m_max,temperature_2m_min")
        .query("forecast_days", "1")
        .query("timezone", "auto")
        .call()
        .ok()?;
    let value: serde_json::Value = response.body_mut().read_json().ok()?;
    let current = value.get("current")?;
    let temp = current.get("temperature_2m")?.as_f64()?.round() as i32;
    let code = current.get("weather_code")?.as_u64()? as u32;
    let is_day = current.get("is_day").and_then(|v| v.as_i64()).unwrap_or(1) == 1;
    let (text, kind) = describe(code, is_day);
    // Máxima y mínima de hoy: son opcionales, sin ellas se enseña solo lo demás.
    let daily = |key: &str| {
        value
            .get("daily")
            .and_then(|d| d.get(key))
            .and_then(|list| list.get(0))
            .and_then(|v| v.as_f64())
            .map(|v| v.round() as i32)
    };
    Some(Weather {
        city: name,
        temp,
        text: text.to_string(),
        max: daily("temperature_2m_max"),
        min: daily("temperature_2m_min"),
        kind: kind.to_string(),
    })
}

/// Tiempo actual de la ciudad de los ajustes, o `None` si el ajuste está apagado, no hay
/// ciudad, no se encuentra o falla la red. Va aparte del hilo principal: espera a la red.
#[tauri::command(async)]
pub fn weather_current() -> Option<Weather> {
    let config = crate::settings::current();
    let city = config.weather_city.trim().to_string();
    if !config.weather || city.is_empty() {
        return None;
    }
    if let Ok(last) = LAST.lock() {
        if let Some((cached_city, at, weather)) = last.as_ref() {
            if *cached_city == city && at.elapsed() < CACHE_FOR {
                return Some(weather.clone());
            }
        }
    }
    let weather = fetch(&city)?;
    if let Ok(mut last) = LAST.lock() {
        *last = Some((city, Instant::now(), weather.clone()));
    }
    Some(weather)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_los_codigos_habituales() {
        assert_eq!(describe(0, true), ("Despejado", "sun"));
        assert_eq!(describe(0, false), ("Noche clara", "moon"));
        // Ninguna descripción pasa de 14 caracteres: es lo que cabe bajo el icono.
        for code in [0, 1, 2, 3, 45, 51, 61, 71, 95, 200] {
            assert!(describe(code, true).0.chars().count() <= 14, "{code}");
            assert!(describe(code, false).0.chars().count() <= 14, "{code}");
        }
        assert_eq!(describe(3, true), ("Nublado", "cloud"));
        assert_eq!(describe(63, true), ("Lluvia", "rain"));
        assert_eq!(describe(95, false), ("Tormenta", "storm"));
        assert_eq!(describe(200, true).1, "cloud");
    }

    /// Consulta real: `cargo test --lib weather -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn consulta_real() {
        println!("{:?}", fetch("Santiago"));
    }
}
