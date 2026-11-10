//! Letras sincronizadas desde LRCLIB.
//!
//! LRCLIB es una API pública y gratuita que no requiere registro. Devuelve las
//! letras en formato LRC (`[mm:ss.cc] texto`), que aquí se convierte en líneas
//! con su marca de tiempo en segundos.

use serde::Serialize;
use std::sync::Mutex;

/// Una línea de la letra con el momento en que empieza a sonar.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    /// Segundos desde el inicio de la pista.
    pub time: f64,
    pub text: String,
}

/// Resultado de buscar la letra de una pista.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// Líneas sincronizadas; vacío si la pista no tiene letra con tiempos.
    pub lines: Vec<LyricLine>,
    /// `true` cuando la búsqueda terminó, haya resultado o no.
    pub resolved: bool,
}


/// Letra de la última pista consultada, junto a su clave.
///
/// Evita repetir la petición en cada sondeo mientras suene la misma canción.
static CACHE: Mutex<Option<(String, Lyrics)>> = Mutex::new(None);

/// Convierte el cuerpo LRC en líneas ordenadas por tiempo.
///
/// Admite `[mm:ss.cc]` y `[mm:ss]`, y descarta las líneas sin texto, que en
/// LRC suelen marcar silencios.
fn parse_lrc(body: &str) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = body
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix('[')?;
            let (stamp, text) = rest.split_once(']')?;
            let (minutes, seconds) = stamp.split_once(':')?;
            let minutes: f64 = minutes.trim().parse().ok()?;
            let seconds: f64 = seconds.trim().parse().ok()?;
            let text = text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            Some(LyricLine {
                time: minutes * 60.0 + seconds,
                text,
            })
        })
        .collect();

    lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

/// Busca la letra de una pista en LRCLIB.
///
/// Devuelve `resolved: true` aunque no haya letra, para que la interfaz sepa
/// que ya no debe esperar. Un fallo de red se trata como "sin letra".
fn fetch(title: &str, artist: &str, album: &str, duration: f64) -> Lyrics {
    // Coincidencia exacta: es la más fiable cuando los metadatos encajan.
    let mut exact = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        urlencoding(title),
        urlencoding(artist)
    );
    if !album.is_empty() {
        exact.push_str(&format!("&album_name={}", urlencoding(album)));
    }
    if duration > 0.0 {
        exact.push_str(&format!("&duration={}", duration.round() as i64));
    }

    if let Some(lines) = request_synced(&exact) {
        return Lyrics {
            lines,
            resolved: true,
        };
    }

    // Respaldo: la búsqueda tolera que el álbum o la duración no coincidan,
    // algo habitual entre reproductores (recopilatorios, ediciones distintas).
    let search = format!(
        "https://lrclib.net/api/search?track_name={}&artist_name={}",
        urlencoding(title),
        urlencoding(artist)
    );
    let lines = request_synced(&search).unwrap_or_default();

    Lyrics {
        lines,
        resolved: true,
    }
}

/// Pide una URL de LRCLIB y devuelve las líneas sincronizadas que encuentre.
///
/// La API responde con un objeto o con una lista según el extremo, así que se
/// aceptan ambos. Devuelve `None` si no hay letra con marcas de tiempo.
fn request_synced(url: &str) -> Option<Vec<LyricLine>> {
    let mut response = ureq::get(url)
        .header("User-Agent", "DynamicIsland/0.1 (dynamic-island desktop)")
        .call()
        .ok()?;

    let value: serde_json::Value = response.body_mut().read_json().ok()?;

    // De una lista se toma el primer resultado que traiga letra sincronizada.
    let candidates: Vec<&serde_json::Value> = match value.as_array() {
        Some(items) => items.iter().collect(),
        None => vec![&value],
    };

    for item in candidates {
        if let Some(body) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
            let lines = parse_lrc(body);
            if !lines.is_empty() {
                return Some(lines);
            }
        }
    }
    None
}

/// Codifica un valor para usarlo en la cadena de consulta.
fn urlencoding(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// Devuelve la letra de la pista, consultando la red solo si cambió.
/// Extrae el nombre del artista cuando el reproductor le añade el álbum.
///
/// Apple Music informa el artista como "Miranda! — El Templo del Pop", y con
/// ese texto completo LRCLIB no encuentra nada. Se corta por el guion largo o
/// por " - ", que son los separadores que usan estos reproductores.
fn clean_artist(artist: &str) -> &str {
    for separator in [" — ", " – ", " - "] {
        if let Some((name, _)) = artist.split_once(separator) {
            return name.trim();
        }
    }
    artist.trim()
}

pub fn for_track(title: &str, artist: &str, album: &str, duration: f64) -> Lyrics {
    if title.is_empty() {
        return Lyrics::default();
    }
    let artist = clean_artist(artist);
    let key = format!("{title}\u{1}{artist}");

    if let Ok(cache) = CACHE.lock() {
        if let Some((cached_key, lyrics)) = cache.as_ref() {
            if cached_key == &key {
                return lyrics.clone();
            }
        }
    }

    let lyrics = fetch(title, artist, album, duration);
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some((key, lyrics.clone()));
    }
    lyrics
}
