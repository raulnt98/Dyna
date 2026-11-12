//! Integración con SMTC (System Media Transport Controls) de Windows.
//!
//! Lee la sesión multimedia activa del sistema (Spotify, Edge, reproductores
//! con soporte SMTC) y permite controlarla. En plataformas distintas de Windows
//! se compila una variante vacía para que el proyecto siga siendo portable.

use serde::Serialize;

/// Estado multimedia que se envía al frontend.
///
/// `position` y `duration` van en segundos. `has_session` en `false` significa
/// que no hay ninguna sesión multimedia activa en el sistema.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub has_session: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    /// AppUserModelId de la app que reproduce (por ejemplo "Spotify.exe").
    pub source_app: String,
    /// `true` si el reproductor admite saltar a otra posición de la pista.
    pub can_seek: bool,
    /// Carátula como data URI listo para un `<img>`, vacío si no hay ninguna.
    pub artwork: String,
}

/// Nombre del evento que recibe el frontend cuando cambia la reproducción.
pub const MEDIA_EVENT: &str = "smtc://media-changed";

/// Palabras que, dentro de un paréntesis o corchete, lo delatan como adorno del
/// título y no como parte de él: «(Official Video)», «[Letra / Lyrics]»…
const NOISE_STRONG: [&str; 14] = [
    "official", "oficial", "video", "vídeo", "videoclip", "lyric", "lyrics", "letra", "audio",
    "visualizer", "visualiser", "hd", "4k", "mv",
];

/// Palabras de relleno que acompañan a las anteriores sin delatar nada solas.
const NOISE_FILLER: [&str; 8] = ["music", "con", "with", "de", "the", "version", "clip", "m"];

/// Indica si el contenido de un paréntesis es solo adorno («Official Video»).
///
/// Todas sus palabras deben ser ruido y al menos una, de las fuertes: así
/// «(Live)», «(Remastered 2011)» o «(feat. Ana)» se conservan.
fn is_noise_group(inner: &str) -> bool {
    let lower = inner.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    !words.is_empty()
        && words.iter().all(|w| NOISE_STRONG.contains(w) || NOISE_FILLER.contains(w))
        && words.iter().any(|w| NOISE_STRONG.contains(w))
}

/// Quita del título los grupos entre () y [] que son solo adorno.
fn strip_noise_groups(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let chars: Vec<char> = title.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let (open, close) = match chars[i] {
            '(' => ('(', ')'),
            '[' => ('[', ']'),
            _ => {
                out.push(chars[i]);
                i += 1;
                continue;
            }
        };
        // Se busca el cierre; sin él, el carácter se trata como texto normal.
        match chars[i + 1..].iter().position(|c| *c == close) {
            Some(offset) => {
                let inner: String = chars[i + 1..i + 1 + offset].iter().collect();
                if is_noise_group(&inner) {
                    i += offset + 2;
                } else {
                    out.push(open);
                    i += 1;
                }
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Limpia título y artista tal como los publican los reproductores web.
///
/// YouTube Music informa al artista como «Nombre - Topic» o «NombreVEVO», y
/// los títulos llevan «(Official Video)» o «[Lyrics]», a veces con el propio
/// artista delante («Artista - Canción»). Eso ensucia la isla y además hace
/// que LRCLIB no encuentre la letra. Se quita solo lo que es adorno: si el
/// resultado quedara vacío se devuelve el texto original.
pub fn tidy_metadata(title: &str, artist: &str) -> (String, String) {
    // Artista: sin « - Topic» ni el «VEVO» pegado al nombre.
    let mut clean_artist = artist.trim().to_string();
    if clean_artist.to_lowercase().ends_with(" - topic") {
        clean_artist.truncate(clean_artist.len() - " - topic".len());
    }
    if clean_artist.len() > 4 && clean_artist.ends_with("VEVO") {
        clean_artist.truncate(clean_artist.len() - 4);
    }
    let clean_artist = clean_artist.trim().to_string();

    // Título: sin adornos, y sin el artista repetido al principio.
    let stripped = strip_noise_groups(title);
    let mut clean_title = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    clean_title = clean_title
        .trim_end_matches(|c: char| c.is_whitespace() || "-–—|".contains(c))
        .to_string();

    if !clean_artist.is_empty() {
        let lower = clean_title.to_lowercase();
        let prefix = clean_artist.to_lowercase();
        for separator in [" - ", " – ", " — "] {
            let full = format!("{prefix}{separator}");
            if lower.starts_with(&full) {
                clean_title = clean_title.chars().skip(full.chars().count()).collect();
                break;
            }
        }
    }

    let title_out = if clean_title.trim().is_empty() {
        title.trim().to_string()
    } else {
        clean_title.trim().to_string()
    };
    let artist_out = if clean_artist.is_empty() {
        artist.trim().to_string()
    } else {
        clean_artist
    };
    (title_out, artist_out)
}

#[cfg(test)]
mod tests {
    use super::tidy_metadata;

    fn tidy(title: &str, artist: &str) -> (String, String) {
        tidy_metadata(title, artist)
    }

    #[test]
    fn quita_topic_y_adornos() {
        assert_eq!(
            tidy("Blinding Lights (Official Video)", "The Weeknd - Topic"),
            ("Blinding Lights".into(), "The Weeknd".into())
        );
    }

    #[test]
    fn quita_el_artista_repetido_y_los_corchetes() {
        assert_eq!(
            tidy("The Weeknd - Blinding Lights [Lyrics]", "The Weeknd"),
            ("Blinding Lights".into(), "The Weeknd".into())
        );
    }

    #[test]
    fn quita_vevo_y_varios_grupos() {
        assert_eq!(
            tidy("Song (Official Music Video) [HD]", "ArtistVEVO"),
            ("Song".into(), "Artist".into())
        );
        assert_eq!(
            tidy("Canción [Letra / Lyrics]", "Artista"),
            ("Canción".into(), "Artista".into())
        );
    }

    #[test]
    fn conserva_lo_que_es_parte_del_titulo() {
        for title in ["Song (Live)", "Song (feat. Ana)", "Song (Remastered 2011)", "Song (Remix)"] {
            assert_eq!(tidy(title, "X"), (title.into(), "X".into()), "{title}");
        }
    }

    #[test]
    fn no_deja_el_titulo_vacio() {
        assert_eq!(tidy("(Official Video)", "X"), ("(Official Video)".into(), "X".into()));
    }

    #[test]
    fn un_parentesis_sin_cerrar_no_rompe_nada() {
        assert_eq!(tidy("Song (Live", "X"), ("Song (Live".into(), "X".into()));
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::MediaState;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use std::sync::Mutex;
    use windows::Media::Control::{
        CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionMediaProperties as MediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;

    /// Unidades de 100 ns por segundo: TimeSpan.Duration se mide en esos ticks.
    const TICKS_PER_SECOND: f64 = 10_000_000.0;

    /// Obtiene el manager de SMTC bloqueando hasta que esté listo.
    ///
    /// `RequestAsync` es asíncrono en WinRT; `join()` espera el resultado.
    fn manager() -> windows::core::Result<Manager> {
        Manager::RequestAsync()?.join()
    }

    /// Segundos transcurridos desde un instante de WinRT hasta ahora.
    ///
    /// `UniversalTime` cuenta en unidades de 100 ns desde el 1 de enero de
    /// 1601, el epoch de Windows; se traslada al de Unix para compararlo.
    fn ticks_since(universal_time: i64) -> f64 {
        /// Ticks de 100 ns entre 1601-01-01 y 1970-01-01.
        const EPOCH_DIFFERENCE: i64 = 116_444_736_000_000_000;

        let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            return 0.0;
        };
        let now_ticks = now.as_nanos() as i64 / 100 + EPOCH_DIFFERENCE;
        (now_ticks - universal_time) as f64 / TICKS_PER_SECOND
    }

    /// Indica si la sesión está reproduciendo ahora mismo.
    fn is_playing(session: &Session) -> bool {
        session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|status| status == PlaybackStatus::Playing)
            .unwrap_or(false)
    }

    /// Sesión multimedia que la isla debe mostrar y controlar.
    ///
    /// Windows llama «actual» a la última con la que se interactuó, y no tiene
    /// por qué ser la que suena: con Spotify en pausa y YouTube Music
    /// reproduciendo, la isla enseñaba y controlaba la pausada. Se prefiere la
    /// actual si está sonando; si no, cualquiera que suene; y solo si no suena
    /// ninguna, la actual.
    fn current_session() -> Option<Session> {
        let manager = manager().ok()?;
        let current = manager.GetCurrentSession().ok();
        if current.as_ref().is_some_and(is_playing) {
            return current;
        }
        if let Ok(sessions) = manager.GetSessions() {
            for session in sessions {
                if is_playing(&session) {
                    return Some(session);
                }
            }
        }
        current
    }

    /// Lee el estado completo de la sesión activa.
    ///
    /// Nunca falla hacia el llamador: si no hay sesión o alguna propiedad no
    /// está disponible, devuelve el estado por defecto o campos vacíos.
    pub fn read_state() -> MediaState {
        let Some(session) = current_session() else {
            return MediaState::default();
        };

        let mut state = MediaState {
            has_session: true,
            ..Default::default()
        };

        // Las propiedades de la pista llegan de forma asíncrona.
        if let Ok(props) = session
            .TryGetMediaPropertiesAsync()
            .and_then(|op| op.join())
        {
            let raw_title = props
                .Title()
                .map(|h| h.to_string_lossy())
                .unwrap_or_default();
            let raw_artist = props
                .Artist()
                .map(|h| h.to_string_lossy())
                .unwrap_or_default();
            // Se limpia aquí y no en la vista: así la isla, la clave de la
            // carátula y la búsqueda de letras ven todos el mismo texto.
            (state.title, state.artist) = super::tidy_metadata(&raw_title, &raw_artist);
            state.album = props
                .AlbumTitle()
                .map(|h| h.to_string_lossy())
                .unwrap_or_default();
            // La clave identifica la pista para no releer la misma carátula.
            let key = format!("{} — {}", state.title, state.artist);
            state.artwork = cached_artwork(&props, &key);
        }

        if let Ok(info) = session.GetPlaybackInfo() {
            state.playing = info
                .PlaybackStatus()
                .map(|status| status == PlaybackStatus::Playing)
                .unwrap_or(false);
            // No todos los reproductores permiten saltar de posición: Apple
            // Music, por ejemplo, acepta la llamada pero la ignora.
            state.can_seek = info
                .Controls()
                .and_then(|c| c.IsPlaybackPositionEnabled())
                .unwrap_or(false);
        }

        if let Ok(timeline) = session.GetTimelineProperties() {
            let ticks_to_secs = |ticks: i64| (ticks as f64 / TICKS_PER_SECOND).max(0.0);
            let start = timeline.StartTime().map(|t| t.Duration).unwrap_or(0);
            let position = timeline.Position().map(|t| t.Duration).unwrap_or(0);
            let end = timeline.EndTime().map(|t| t.Duration).unwrap_or(0);
            // La posición es absoluta: se descuenta StartTime para que empiece en 0.
            state.position = ticks_to_secs(position.saturating_sub(start));
            state.duration = ticks_to_secs(end.saturating_sub(start));

            // SMTC congela Position entre actualizaciones, que llegan como
            // mucho una vez por segundo. Para que la letra no vaya a tirones
            // se suma el tiempo transcurrido desde la última.
            if state.playing {
                if let Ok(updated) = timeline.LastUpdatedTime() {
                    let elapsed = ticks_since(updated.UniversalTime);
                    if (0.0..2.0).contains(&elapsed) {
                        state.position = (state.position + elapsed).min(state.duration);
                    }
                }
            }
        }

        state.source_app = session
            .SourceAppUserModelId()
            .map(|h| h.to_string_lossy())
            .unwrap_or_default();

        state
    }

    /// Tamaño máximo de carátula que se acepta (4 MB).
    ///
    /// Evita cargar en memoria una imagen desmedida enviada por un reproductor
    /// que informe un tamaño incorrecto.
    const MAX_ARTWORK_BYTES: u64 = 4 * 1024 * 1024;

    /// Carátula decodificada de la pista actual y cuántas veces se ha leído.
    struct ArtworkEntry {
        key: String,
        artwork: String,
        reads: u32,
    }

    static ARTWORK_CACHE: Mutex<Option<ArtworkEntry>> = Mutex::new(None);

    /// Lecturas tras las que una carátula con imagen se da por definitiva.
    ///
    /// Los reproductores web (YouTube Music) publican el título antes que la
    /// miniatura, y durante un instante esta sigue siendo la de la canción
    /// anterior. Guardar la primera lectura dejaba esa imagen fija toda la
    /// pista. A ~4 lecturas por segundo, 8 son unos dos segundos.
    const ARTWORK_SETTLE_READS: u32 = 8;

    /// Lecturas que se insisten si la pista no ha entregado ninguna imagen,
    /// unos diez segundos: pasado ese tiempo se asume que no tiene carátula.
    const ARTWORK_EMPTY_READS: u32 = 40;

    /// Devuelve la carátula de la pista.
    ///
    /// El estado se relee cada 250 ms para refrescar la posición, y
    /// decodificar la imagen en cada pasada sería un desperdicio: se reutiliza
    /// mientras no cambie la pista, **salvo al principio**, cuando se vuelve a
    /// leer unas cuantas veces hasta que la imagen se asienta (ver
    /// `ARTWORK_SETTLE_READS`). `key` identifica la pista (título y artista).
    fn cached_artwork(props: &MediaProperties, key: &str) -> String {
        let mut reads = 0;
        if let Ok(cache) = ARTWORK_CACHE.lock() {
            if let Some(entry) = cache.as_ref() {
                if entry.key == key {
                    let limit = if entry.artwork.is_empty() {
                        ARTWORK_EMPTY_READS
                    } else {
                        ARTWORK_SETTLE_READS
                    };
                    if entry.reads >= limit {
                        return entry.artwork.clone();
                    }
                    reads = entry.reads;
                }
            }
        }

        let artwork = read_artwork(props);
        if let Ok(mut cache) = ARTWORK_CACHE.lock() {
            *cache = Some(ArtworkEntry {
                key: key.to_string(),
                artwork: artwork.clone(),
                reads: reads + 1,
            });
        }
        artwork
    }

    /// Deduce el tipo de imagen a partir de sus primeros bytes.
    ///
    /// Se usa cuando el reproductor no declara un tipo utilizable. Si no se
    /// reconoce, se devuelve PNG, que los navegadores toleran bien.
    fn sniff_image_mime(bytes: &[u8]) -> &'static str {
        match bytes {
            [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
            [0x89, b'P', b'N', b'G', ..] => "image/png",
            [b'G', b'I', b'F', ..] => "image/gif",
            [b'B', b'M', ..] => "image/bmp",
            [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
            _ => "image/png",
        }
    }

    /// Lee la carátula de la pista y la devuelve como data URI.
    ///
    /// Devuelve una cadena vacía si la sesión no expone miniatura o si falla
    /// cualquier paso: la carátula es decorativa y nunca debe tumbar la lectura
    /// del resto del estado.
    fn read_artwork(props: &MediaProperties) -> String {
        let Ok(reference) = props.Thumbnail() else {
            return String::new();
        };
        let Ok(stream) = reference.OpenReadAsync().and_then(|op| op.join()) else {
            return String::new();
        };

        let size = stream.Size().unwrap_or(0);
        if size == 0 || size > MAX_ARTWORK_BYTES {
            return String::new();
        }

        // El stream es !Send: se lee aquí mismo, sin cruzar de hilo.
        let Ok(input) = stream.GetInputStreamAt(0) else {
            return String::new();
        };
        let Ok(reader) = DataReader::CreateDataReader(&input) else {
            return String::new();
        };

        // LoadAsync devuelve cuántos bytes llegaron realmente, que pueden ser
        // menos de los pedidos: el buffer se dimensiona con ese valor.
        let Ok(loaded) = reader.LoadAsync(size as u32).and_then(|op| op.join()) else {
            return String::new();
        };
        if loaded == 0 {
            return String::new();
        }

        let mut bytes = vec![0u8; loaded as usize];
        if reader.ReadBytes(&mut bytes).is_err() {
            return String::new();
        }

        // Algunos reproductores (Apple Music) devuelven varios tipos separados
        // por comas: un data URI solo admite uno, así que se toma el primero.
        let declared = stream
            .ContentType()
            .map(|h| h.to_string_lossy())
            .unwrap_or_default();
        let declared = declared
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();

        // Si el tipo declarado no sirve, se deduce de los bytes de la imagen.
        let mime = if declared.starts_with("image/") {
            declared
        } else {
            sniff_image_mime(&bytes).to_string()
        };

        format!("data:{};base64,{}", mime, BASE64.encode(&bytes))
    }

    /// Acciones de control admitidas sobre la sesión activa.
    pub enum Command {
        PlayPause,
        Next,
        Previous,
        /// Salta a una posición de la pista, en segundos desde el inicio.
        Seek(f64),
    }

    /// Envía un comando al reproductor activo.
    ///
    /// Devuelve `false` si no hay sesión o si el reproductor rechaza la acción
    /// (algunos no permiten saltar de pista ni buscar, por ejemplo en una radio).
    pub fn control(command: Command) -> bool {
        let Some(session) = current_session() else {
            return false;
        };
        let result = match command {
            Command::PlayPause => session.TryTogglePlayPauseAsync(),
            Command::Next => session.TrySkipNextAsync(),
            Command::Previous => session.TrySkipPreviousAsync(),
            Command::Seek(seconds) => {
                // La posición de SMTC es absoluta: se suma StartTime, que no
                // siempre es cero (por ejemplo en capítulos o emisiones).
                let start = session
                    .GetTimelineProperties()
                    .and_then(|t| t.StartTime())
                    .map(|t| t.Duration)
                    .unwrap_or(0);
                let ticks = (seconds.max(0.0) * TICKS_PER_SECOND) as i64;
                session.TryChangePlaybackPositionAsync(start.saturating_add(ticks))
            }
        };
        result.and_then(|op| op.join()).unwrap_or(false)
    }

    /// Avisa cuando el sistema cambia de reproductor activo, para reaccionar
    /// de inmediato sin esperar al siguiente sondeo.
    ///
    /// Los cambios dentro de una misma sesión (pista nueva, pausa, avance) los
    /// detecta el sondeo comparando estados, así que aquí solo se escucha el
    /// cambio de sesión. El token de registro se descarta a propósito: el
    /// manejador debe seguir activo mientras viva la aplicación.
    ///
    /// Devuelve `false` si no se pudo registrar; en ese caso el sondeo sigue
    /// detectando los cambios, solo que con hasta un segundo de retraso.
    pub fn watch<F>(on_change: F) -> bool
    where
        F: Fn() + Send + Sync + 'static,
    {
        use windows::Foundation::TypedEventHandler;

        let Ok(manager) = manager() else {
            return false;
        };

        let handler = TypedEventHandler::<Manager, CurrentSessionChangedEventArgs>::new(
            move |_manager, _args| {
                on_change();
                Ok(())
            },
        );
        if manager.CurrentSessionChanged(&handler).is_err() {
            return false;
        }

        // El manager debe seguir vivo o los eventos dejan de llegar.
        std::mem::forget(manager);
        true
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::MediaState;

    pub enum Command {
        PlayPause,
        Next,
        Previous,
        Seek(f64),
    }

    pub fn read_state() -> MediaState {
        MediaState::default()
    }

    pub fn control(_command: Command) -> bool {
        false
    }

    pub fn watch<F>(_on_change: F) -> bool
    where
        F: Fn() + Send + Sync + 'static,
    {
        false
    }
}

pub use imp::{control, read_state, watch, Command};
