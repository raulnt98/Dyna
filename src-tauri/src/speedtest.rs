//! Test de velocidad de internet, contra el servicio público de Cloudflare.
//!
//! **Sale a la red y mueve datos**, así que tiene su propio ajuste (`speedtest`) y su línea en
//! el README. Solo corre cuando el usuario pulsa el botón de la vista de inicio, nunca por su
//! cuenta, y habla únicamente con `speed.cloudflare.com` (fijo, sin redirecciones, con tiempo
//! límite). No se envía ningún dato del usuario: la descarga son ceros que el servidor genera
//! y la subida, ceros que se envían.
//!
//! Mide dos cosas: latencia (mediana de varias peticiones vacías) y bajada (bytes recibidos en
//! ~5 s). **No mide la subida**, a propósito: obligaba a enviar datos, duplicaba lo que se mueve en
//! cada prueba y era lo que más facilitaba que el servicio nos limitara (429). El progreso llega a
//! la isla por eventos.

use serde::Serialize;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const HOST: &str = "https://speed.cloudflare.com";
/// Evento con el avance: la fase y la velocidad del momento.
pub const PROGRESS_EVENT: &str = "speedtest://progress";
/// Evento con el resultado final.
pub const DONE_EVENT: &str = "speedtest://done";

/// Tamaño de cada petición de bajada: el servicio rechaza (403) las de más de 50 MB. Se usa el
/// máximo: cuantas menos peticiones, menos probable que el servicio nos limite (429).
const DOWN_CHUNK: u64 = 50_000_000;
const DOWN_SECS: f64 = 5.0;

/// Tras un 429 el servicio sigue rechazando un rato: no se vuelve a llamar hasta pasado este
/// tiempo, que además deja claro al usuario qué pasa en lugar de repetir el fallo cada vez.
const COOLDOWN: Duration = Duration::from_secs(3 * 60);
static LIMITED_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Solo una prueba a la vez: pulsar el botón dos veces no debe lanzar dos.
static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// `ping` o `down`.
    pub phase: &'static str,
    /// Megabits por segundo del momento; 0 en la fase de latencia.
    pub mbps: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeedResult {
    /// Bajada en Mbps; `None` si la fase falló.
    pub down: Option<f64>,
    /// Latencia en milisegundos.
    pub ping: Option<f64>,
    /// Por qué no salió bien: `limit` si el servicio limitó las pruebas seguidas (429) y
    /// `network` si no hubo conexión. `None` si se midió.
    pub error: Option<&'static str>,
}

fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
            .tls_config(crate::tls_config())
        .timeout_connect(Some(Duration::from_secs(4)))
        .timeout_global(Some(Duration::from_secs(20)))
        .max_redirects(0)
        .build();
    ureq::Agent::new_with_config(config)
}

/// Megabits por segundo a partir de bytes y segundos.
pub fn mbps(bytes: u64, secs: f64) -> f64 {
    if secs <= 0.0 {
        return 0.0;
    }
    bytes as f64 * 8.0 / secs / 1_000_000.0
}

/// Mediana de una lista de tiempos.
pub fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(values[values.len() / 2])
}

fn measure_ping(agent: &ureq::Agent) -> Option<f64> {
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let mut response = agent.get(format!("{HOST}/__down?bytes=0")).call().ok()?;
        let _ = response.body_mut().read_to_vec();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    median(times)
}

fn measure_down(agent: &ureq::Agent, emit: &dyn Fn(Progress)) -> Result<f64, &'static str> {
    let start = Instant::now();
    let mut total = 0u64;
    let (mut window_bytes, mut window_at) = (0u64, Instant::now());
    let mut buffer = vec![0u8; 64 * 1024];
    // Peticiones de 50 MB una tras otra hasta completar el tiempo.
    while start.elapsed().as_secs_f64() < DOWN_SECS {
        let mut response = match agent.get(format!("{HOST}/__down?bytes={DOWN_CHUNK}")).call() {
            Ok(response) => response,
            // Si ya había datos de peticiones anteriores se da por buena la medida; si no, se
            // dice por qué falló: 429 es «demasiadas pruebas seguidas», no «sin internet».
            Err(ureq::Error::StatusCode(429)) if total == 0 => return Err("limit"),
            Err(_) if total == 0 => return Err("network"),
            Err(_) => break,
        };
        let mut reader = response.body_mut().with_config().limit(DOWN_CHUNK + 1024).reader();
        loop {
            let Ok(n) = reader.read(&mut buffer) else { break };
            if n == 0 {
                break;
            }
            total += n as u64;
            window_bytes += n as u64;
            if window_at.elapsed().as_millis() >= 250 {
                emit(Progress { phase: "down", mbps: mbps(window_bytes, window_at.elapsed().as_secs_f64()) });
                window_bytes = 0;
                window_at = Instant::now();
            }
            if start.elapsed().as_secs_f64() >= DOWN_SECS {
                break;
            }
        }
    }
    // Los primeros instantes (arranque lento de TCP) cuentan: es lo que vería el usuario.
    if total > 0 {
        Ok(mbps(total, start.elapsed().as_secs_f64()))
    } else {
        Err("network")
    }
}

/// Lanza la prueba y devuelve el resultado; el avance va por `speedtest://progress`.
///
/// Va con `async` para no ocupar el hilo de la ventana: tarda más de diez segundos. Si ya hay
/// una en marcha devuelve `None` sin lanzar otra.
#[tauri::command(async)]
pub fn speedtest_run(app: AppHandle) -> Option<SpeedResult> {
    if !crate::settings::current().speedtest {
        return None;
    }
    if RUNNING.swap(true, Ordering::SeqCst) {
        return None;
    }
    let emit = |progress: Progress| {
        let _ = app.emit(PROGRESS_EVENT, progress);
    };
    let result = run(&emit);
    RUNNING.store(false, Ordering::SeqCst);
    let _ = app.emit(DONE_EVENT, result.clone());
    Some(result)
}

/// La prueba completa, con el avance entregado a `emit`. Aparte del comando para poder
/// ejecutarla sin ventana.
fn run(emit: &dyn Fn(Progress)) -> SpeedResult {
    // Si hace poco el servicio nos limitó, se responde igual sin tocar la red.
    if let Ok(until) = LIMITED_UNTIL.lock() {
        if until.is_some_and(|t| Instant::now() < t) {
            return SpeedResult { down: None, ping: None, error: Some("limit") };
        }
    }
    let agent = agent();
    emit(Progress { phase: "ping", mbps: 0.0 });
    let ping = measure_ping(&agent);
    let (down, error) = match measure_down(&agent, emit) {
        Ok(value) => (Some(value), None),
        Err(reason) => (None, Some(reason)),
    };
    if error == Some("limit") {
        if let Ok(mut until) = LIMITED_UNTIL.lock() {
            *until = Some(Instant::now() + COOLDOWN);
        }
    }
    SpeedResult { down, ping, error }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convierte_a_megabits() {
        // 12,5 MB en 1 s son 100 Mbps.
        assert!((mbps(12_500_000, 1.0) - 100.0).abs() < 1e-9);
        assert_eq!(mbps(1, 0.0), 0.0);
    }

    /// Prueba real contra Cloudflare (~11 s): `cargo test --lib speedtest -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn prueba_real() {
        let result = run(&|p| println!("  {} {:.1} Mbps", p.phase, p.mbps));
        println!("{result:?}");
    }

    #[test]
    fn la_mediana_aguanta_un_valor_atipico() {
        assert_eq!(median(vec![10.0, 12.0, 11.0, 400.0, 13.0]), Some(12.0));
        assert_eq!(median(vec![]), None);
    }
}
