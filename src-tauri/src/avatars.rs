//! Foto de perfil de las notificaciones (experimental).
//!
//! `UserNotificationListener` entrega solo el texto de un aviso. La foto del
//! contacto que WhatsApp enseña va como `<image placement="appLogoOverride">`
//! dentro del XML del toast, con la dirección https de su servidor. Ese XML
//! vive en la base de datos de notificaciones de Windows (`wpndatabase.db`), que
//! aquí se abre **solo para lectura**; la imagen se descarga con esa dirección.
//!
//! Es una salida de red y una lectura de contenido privado, así que:
//! - solo se hace para las aplicaciones que el usuario activó en
//!   `notification_apps`: esa activación es el permiso, no hay un interruptor
//!   aparte, y el panel de ajustes avisa de que se descarga la foto;
//! - solo se descarga de los servidores de `ALLOWED_HOSTS`, sin seguir
//!   redirecciones: un aviso de otra aplicación no puede hacer que la isla pida
//!   una dirección cualquiera;
//! - nada de lo leído se escribe en logs ni en disco: la foto vive en memoria,
//!   como data URI, y viaja solo a la ventana de la isla.
//!
//! No es una API documentada de Windows: el esquema de la base puede cambiar.
//! Cualquier fallo se resuelve en `None` y la isla vuelve a la inicial.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Servidores de los que se admite una foto. WhatsApp sirve las suyas desde
/// `pps.whatsapp.net`; se compara el final del nombre, no una coincidencia
/// parcial, para que `whatsapp.net.otro.com` no valga.
const ALLOWED_HOSTS: &[&str] = &["whatsapp.net"];

/// Una foto de perfil pesa unos pocos KB; más que esto no es una foto de perfil.
const MAX_BYTES: u64 = 256 * 1024;

/// Fotos recordadas en memoria. La dirección lleva firmas que cambian en cada
/// aviso, así que se recuerda por la dirección sin ellas: un contacto que
/// escribe varias veces no se vuelve a descargar.
const CACHE_LEN: usize = 24;
static CACHE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// Cliente con límites de tiempo y sin redirecciones.
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(3)))
            .timeout_global(Some(Duration::from_secs(5)))
            .max_redirects(0)
            .build();
        ureq::Agent::new_with_config(config)
    })
}

/// Foto de la notificación `id` como data URI, o `None` si no la hay.
pub fn for_notification(id: u32) -> Option<String> {
    let xml = payload(id)?;
    let url = logo_url(&xml)?;
    let key = url.split('?').next()?.to_string();

    if let Some(hit) = cached(&key) {
        return Some(hit);
    }
    let uri = download(&url)?;
    remember(key, uri.clone());
    Some(uri)
}

fn cached(key: &str) -> Option<String> {
    let cache = CACHE.lock().ok()?;
    cache.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn remember(key: String, uri: String) {
    if let Ok(mut cache) = CACHE.lock() {
        if cache.len() >= CACHE_LEN {
            cache.remove(0);
        }
        cache.push((key, uri));
    }
}

/// Dirección de la imagen `appLogoOverride` del toast, si es de un servidor admitido.
fn logo_url(xml: &str) -> Option<String> {
    let mut rest = xml;
    while let Some(start) = rest.find("<image") {
        let tag_end = rest[start..].find('>')? + start;
        let tag = &rest[start..tag_end];
        rest = &rest[tag_end..];

        if !tag.contains("appLogoOverride") {
            continue;
        }
        let src_at = tag.find("src=\"")? + 5;
        let src = &tag[src_at..];
        let url = src[..src.find('"')?].replace("&amp;", "&");
        return allowed(&url).then_some(url);
    }
    None
}

/// `true` solo para `https://` hacia un servidor de `ALLOWED_HOSTS`.
fn allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Con usuario o puerto la autoridad ya no es solo un nombre de servidor.
    if authority.is_empty() || authority.contains(['@', ':']) {
        return false;
    }
    let host = authority.to_ascii_lowercase();
    ALLOWED_HOSTS
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

fn download(url: &str) -> Option<String> {
    let mut response = agent().get(url).call().ok()?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .ok()?;
    // El tipo se decide por el contenido, no por lo que diga la cabecera.
    let mime = sniff(&bytes)?;
    Some(format!("data:{mime};base64,{}", BASE64.encode(&bytes)))
}

/// Tipo de imagen por sus primeros bytes; `None` si no es JPEG, PNG ni WebP.
pub(crate) fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// XML del toast `id`, leído de la base de datos de notificaciones.
#[cfg(windows)]
fn payload(id: u32) -> Option<String> {
    use rusqlite::types::ValueRef;
    use rusqlite::{Connection, OpenFlags};

    // Con barras normales: la ruta con `\` no la abre SQLite en esta base.
    let local = std::env::var("LOCALAPPDATA").ok()?.replace('\\', "/");
    let path = format!("{local}/Microsoft/Windows/Notifications/wpndatabase.db");

    // Solo lectura: Windows mantiene la base abierta y escribiendo (modo WAL).
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let _ = conn.busy_timeout(Duration::from_millis(300));

    let bytes: Vec<u8> = conn
        .query_row("SELECT Payload FROM Notification WHERE Id = ?1", [id], |row| {
            Ok(match row.get_ref(0)? {
                ValueRef::Blob(b) | ValueRef::Text(b) => b.to_vec(),
                _ => Vec::new(),
            })
        })
        .ok()?;
    decode(&bytes)
}

#[cfg(not(windows))]
fn payload(_id: u32) -> Option<String> {
    None
}

/// El XML se guarda en UTF-8 o en UTF-16 según la aplicación.
#[cfg(windows)]
fn decode(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let text = if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    Some(text.trim_start_matches('\u{feff}').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_admite_https_de_los_servidores_de_la_lista() {
        assert!(allowed("https://pps.whatsapp.net/v/t61/foto.jpg?oh=1"));
        assert!(allowed("https://whatsapp.net/x"));
        // Parecidos que no valen.
        assert!(!allowed("http://pps.whatsapp.net/foto.jpg"));
        assert!(!allowed("https://whatsapp.net.otro.com/foto.jpg"));
        assert!(!allowed("https://evilwhatsapp.net/foto.jpg"));
        assert!(!allowed("https://pps.whatsapp.net@otro.com/foto.jpg"));
        assert!(!allowed("https://pps.whatsapp.net:8443/foto.jpg"));
        assert!(!allowed("https://localhost/foto.jpg"));
        assert!(!allowed("file:///C:/foto.jpg"));
    }

    #[test]
    fn encuentra_la_imagen_del_aviso_y_decodifica_el_ampersand() {
        let xml = r#"<toast><visual><binding template="ToastGeneric">
            <image placement="hero" src="https://pps.whatsapp.net/otra.jpg"/>
            <image src="https://pps.whatsapp.net/v/t/foto.jpg?a=1&amp;b=2" placement="appLogoOverride" hint-crop="circle"/>
            </binding></visual></toast>"#;
        assert_eq!(
            logo_url(xml).as_deref(),
            Some("https://pps.whatsapp.net/v/t/foto.jpg?a=1&b=2")
        );
    }

    #[test]
    fn rechaza_una_imagen_de_un_servidor_no_admitido() {
        let xml = r#"<image placement="appLogoOverride" src="https://tracker.example.com/p.jpg"/>"#;
        assert_eq!(logo_url(xml), None);
    }

    #[test]
    fn reconoce_el_tipo_por_el_contenido() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("image/jpeg"));
        assert_eq!(sniff(&[0x89, b'P', b'N', b'G', 0, 0]), Some("image/png"));
        assert_eq!(sniff(b"<html>no soy una imagen</html>"), None);
    }
}
