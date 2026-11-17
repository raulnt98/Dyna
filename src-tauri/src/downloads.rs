//! Descargas del navegador en curso: velocidad y aviso al terminar.
//!
//! Windows no tiene una API general de descargas, y cada navegador lleva sus cuentas. Lo que
//! todos hacen es escribir la descarga en un archivo temporal de la carpeta Descargas que va
//! creciendo (`.crdownload` en Chrome, Edge y Brave, `.part` en Firefox, `.opdownload` en
//! Opera) y renombrarlo al terminar. Eso es lo que se vigila: **el tamaño de esos archivos**.
//!
//! Por eso no hay porcentaje: el navegador no dice cuánto falta, y de fuera no se sabe. Se
//! enseña la velocidad y cuánto se ha bajado. Tampoco sirve para Edge/Chrome en perfiles con
//! la carpeta de descargas cambiada, ni para gestores que no usen esos nombres.
//!
//! Solo se leen nombres, tamaños y fechas de la carpeta: **nunca el contenido** de los
//! archivos. El nombre del archivo terminado va a la isla y no se guarda ni se registra.

use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// Evento con el estado de las descargas en curso, una vez por segundo mientras haya alguna.
pub const PROGRESS_EVENT: &str = "downloads://progress";
/// Evento al terminar una descarga.
pub const DONE_EVENT: &str = "downloads://done";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// Hay al menos una descarga creciendo.
    pub active: bool,
    pub count: u32,
    /// Bytes bajados entre todas las descargas activas.
    pub bytes: u64,
    /// Bytes por segundo, sumando todas.
    pub speed: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadDone {
    /// Nombre del archivo terminado. **Contenido privado**: solo para la isla.
    pub name: String,
    pub size: u64,
}

/// Extensiones temporales de los navegadores mientras descargan.
const TEMP_EXTENSIONS: [&str; 4] = ["crdownload", "part", "download", "opdownload"];

/// Un archivo temporal que no ha crecido en tanto tiempo es una descarga abandonada o
/// pausada, no una en curso.
const STALL: Duration = Duration::from_secs(4);

fn is_temp(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| TEMP_EXTENSIONS.iter().any(|t| e.eq_ignore_ascii_case(t)))
}

/// Carpeta Descargas del usuario, sea cual sea su ubicación.
#[cfg(target_os = "windows")]
fn downloads_dir() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    unsafe {
        let raw = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None).ok()?;
        let path = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0 as *const _));
        path.map(PathBuf::from)
    }
}

#[cfg(not(target_os = "windows"))]
fn downloads_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads"))
}

/// Lo que se recuerda de cada archivo temporal entre una vuelta y la siguiente.
struct Tracked {
    size: u64,
    /// Cuándo se vio crecer por última vez.
    grew_at: Instant,
}

/// Estado del vigilante, aparte del hilo para poder probarlo sin disco.
#[derive(Default)]
struct Watch {
    tracked: HashMap<PathBuf, Tracked>,
}

/// Resultado de una vuelta.
struct Step {
    progress: DownloadProgress,
    /// Archivos temporales que desaparecieron: (último tamaño visto).
    vanished: Vec<(PathBuf, u64)>,
}

impl Watch {
    /// Una vuelta con la lista actual de archivos temporales `(ruta, tamaño)`.
    fn step(&mut self, now: Instant, files: &[(PathBuf, u64)], elapsed: Duration) -> Step {
        let mut bytes = 0u64;
        let mut grown = 0u64;
        let mut count = 0u32;

        for (path, size) in files {
            let entry = self.tracked.entry(path.clone()).or_insert(Tracked { size: *size, grew_at: now });
            if *size > entry.size {
                grown += *size - entry.size;
                entry.size = *size;
                entry.grew_at = now;
            }
            if now.duration_since(entry.grew_at) < STALL {
                count += 1;
                bytes += *size;
            }
        }

        // Los que ya no están: la descarga terminó (renombrado) o se canceló (borrado).
        let present: Vec<&PathBuf> = files.iter().map(|(p, _)| p).collect();
        let gone: Vec<PathBuf> = self.tracked.keys().filter(|p| !present.contains(p)).cloned().collect();
        let mut vanished = Vec::new();
        for path in gone {
            if let Some(t) = self.tracked.remove(&path) {
                vanished.push((path, t.size));
            }
        }

        let secs = elapsed.as_secs_f64().max(0.001);
        Step {
            progress: DownloadProgress { active: count > 0, count, bytes, speed: (grown as f64 / secs) as u64 },
            vanished,
        }
    }
}

/// Archivos temporales de la carpeta, con su tamaño. Solo metadatos.
fn scan(dir: &std::path::Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| is_temp(&e.path()))
        .filter_map(|e| e.metadata().ok().map(|m| (e.path(), m.len())))
        .collect()
}

/// El archivo terminado que corresponde a una descarga que acaba de desaparecer: el
/// no temporal más reciente de la carpeta, modificado hace muy poco y de tamaño parecido.
/// `None` si no hay (la descarga se canceló).
fn finished_file(dir: &std::path::Path, last_size: u64) -> Option<(String, u64)> {
    let now = SystemTime::now();
    let mut best: Option<(SystemTime, String, u64)> = None;
    for entry in std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if is_temp(&path) || path.is_dir() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        if now.duration_since(modified).unwrap_or(Duration::MAX) > Duration::from_secs(8) {
            continue;
        }
        // Tamaño parecido: el temporal puede quedarse algo corto de lo que acaba pesando.
        if meta.len() + meta.len() / 8 + 4096 < last_size {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if best.as_ref().is_none_or(|(t, _, _)| modified > *t) {
            best = Some((modified, name, meta.len()));
        }
    }
    best.map(|(_, name, size)| (name, size))
}

/// Arranca el vigilante. El ajuste se consulta en cada vuelta: apagarlo surte efecto en caliente.
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || {
        let mut watch = Watch::default();
        let mut was_active = false;
        let mut last = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(1000));
            let config = crate::settings::current();
            let now = Instant::now();
            let elapsed = now.duration_since(last);
            last = now;

            if !(config.notifications && config.notify_downloads) {
                if was_active {
                    was_active = false;
                    let _ = app.emit(PROGRESS_EVENT, DownloadProgress { active: false, count: 0, bytes: 0, speed: 0 });
                }
                watch.tracked.clear();
                continue;
            }
            let Some(dir) = downloads_dir() else { continue };
            let files = scan(&dir);
            let step = watch.step(now, &files, elapsed);

            if step.progress.active || was_active {
                let _ = app.emit(PROGRESS_EVENT, step.progress.clone());
            }
            was_active = step.progress.active;

            for (_, last_size) in step.vanished {
                // Se da un instante al navegador para renombrar antes de buscar el resultado.
                std::thread::sleep(Duration::from_millis(400));
                if let Some((name, size)) = finished_file(&dir, last_size) {
                    let _ = app.emit(DONE_EVENT, DownloadDone { name, size });
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: u64) -> (PathBuf, u64) {
        (PathBuf::from(name), size)
    }

    #[test]
    fn reconoce_los_temporales_de_los_navegadores() {
        assert!(is_temp(std::path::Path::new("C:/d/Unconfirmed 123.crdownload")));
        assert!(is_temp(std::path::Path::new("video.mp4.part")));
        assert!(is_temp(std::path::Path::new("X.CRDOWNLOAD")));
        assert!(!is_temp(std::path::Path::new("informe.pdf")));
    }

    #[test]
    fn mide_la_velocidad_con_lo_que_crece() {
        let mut watch = Watch::default();
        let t0 = Instant::now();
        watch.step(t0, &[file("a.crdownload", 1_000)], Duration::from_secs(1));
        let step = watch.step(t0 + Duration::from_secs(1), &[file("a.crdownload", 3_001_000)], Duration::from_secs(1));
        assert!(step.progress.active);
        assert_eq!(step.progress.speed, 3_000_000);
        assert_eq!(step.progress.count, 1);
    }

    #[test]
    fn un_temporal_que_no_crece_deja_de_contar() {
        let mut watch = Watch::default();
        let t0 = Instant::now();
        watch.step(t0, &[file("a.part", 500)], Duration::from_secs(1));
        // Pasan 10 s sin crecer: es una descarga abandonada, no una en curso.
        let step = watch.step(t0 + Duration::from_secs(10), &[file("a.part", 500)], Duration::from_secs(10));
        assert!(!step.progress.active);
        assert!(step.vanished.is_empty());
    }

    #[test]
    fn avisa_cuando_desaparece_el_temporal() {
        let mut watch = Watch::default();
        let t0 = Instant::now();
        watch.step(t0, &[file("a.crdownload", 7_000)], Duration::from_secs(1));
        let step = watch.step(t0 + Duration::from_secs(1), &[], Duration::from_secs(1));
        assert_eq!(step.vanished.len(), 1);
        assert_eq!(step.vanished[0].1, 7_000);
        assert!(!step.progress.active);
    }
}
