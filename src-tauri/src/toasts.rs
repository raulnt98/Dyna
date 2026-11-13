//! Notificaciones de otras aplicaciones (Teams, Outlook…), leídas de Windows.
//!
//! Usa `UserNotificationListener`, que funciona también sin empaquetar la app
//! (se verificó con una prueba de acceso). El usuario debe haber permitido el
//! acceso a notificaciones en Windows; si no, simplemente no llega nada.
//!
//! **El contenido es privado**: nada de lo que se lee aquí se escribe en logs ni
//! en disco, y solo viaja de este hilo a la ventana de la isla.

use serde::Serialize;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter};

/// Evento con una notificación nueva de otra aplicación.
pub const TOAST_EVENT: &str = "toast://received";

/// Lo que la isla necesita para dibujar una notificación.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppToast {
    pub id: u32,
    /// Nombre de la aplicación que la emite.
    pub app: String,
    /// Primera línea: en un chat suele ser el remitente.
    pub title: String,
    /// El resto del texto, en una sola cadena.
    pub body: String,
    /// Foto de perfil del remitente como data URI, si el ajuste está activo y el
    /// aviso la trae (ver `avatars.rs`). **Contenido privado.**
    pub avatar: Option<String>,
}

/// Evento con la lista de aplicaciones detectadas, para el panel de ajustes.
pub const APPS_EVENT: &str = "toasts://apps";

/// Identificadores de aplicación de los avisos recientes, para abrirla al hacer
/// clic. Se guardan aquí y no se mandan al frontend: el comando `toast_open`
/// recibe solo el id del aviso, de modo que la ventana no puede pedir que se
/// lance una aplicación cualquiera, únicamente la de un aviso que ya vio.
static LAUNCH: std::sync::Mutex<Vec<(u32, String)>> = std::sync::Mutex::new(Vec::new());
const LAUNCH_LEN: usize = 32;

fn remember_launch(id: u32, aumid: String) {
    if aumid.is_empty() {
        return;
    }
    if let Ok(mut list) = LAUNCH.lock() {
        list.retain(|(known, _)| *known != id);
        if list.len() >= LAUNCH_LEN {
            list.remove(0);
        }
        list.push((id, aumid));
    }
}

/// Abre (o trae al frente) la aplicación que emitió el aviso `id`.
///
/// No abre el chat ni el mensaje concreto: Windows no permite reenviar la
/// activación de un aviso desde fuera de la aplicación que lo recibe. Se lanza la
/// aplicación por su identificador, igual que desde el menú Inicio.
#[cfg(windows)]
pub fn open(id: u32) -> bool {
    use std::os::windows::process::CommandExt;

    let Some(aumid) = LAUNCH
        .lock()
        .ok()
        .and_then(|list| list.iter().find(|(known, _)| *known == id).map(|(_, a)| a.clone()))
    else {
        return false;
    };
    // El identificador va tal cual a una ruta de `shell:AppsFolder`: solo se
    // admiten los caracteres de un AUMID normal, por si llegara algo extraño.
    let valid = aumid
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | '!' | '{' | '}' | ' ' | '('  | ')'));
    if !valid {
        return false;
    }
    // CREATE_NO_WINDOW: `explorer.exe` no debe abrir consola.
    std::process::Command::new("explorer.exe")
        .arg(format!("shell:AppsFolder\\{aumid}"))
        .creation_flags(0x0800_0000)
        .spawn()
        .is_ok()
}

#[cfg(not(windows))]
pub fn open(_id: u32) -> bool {
    false
}

/// Indica si el usuario activó las notificaciones de esa aplicación.
///
/// Una aplicación que no está en la lista (aún sin detectar) no se muestra: la
/// lista se rellena sola al verla, siempre desactivada, y es el usuario quien
/// la enciende.
fn allowed(app: &str, apps: &[crate::settings::NotificationApp]) -> bool {
    apps.iter()
        .any(|entry| entry.enabled && entry.name.eq_ignore_ascii_case(app))
}

/// Añade a los ajustes las aplicaciones nuevas, desactivadas, y avisa al panel.
///
/// Solo se guarda el nombre de la aplicación, nunca el contenido de lo que
/// envía.
fn remember_apps(app: &AppHandle, names: &HashSet<String>) {
    let mut config = crate::settings::current();
    let before = config.notification_apps.len();
    for name in names {
        if name.is_empty()
            || config
                .notification_apps
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(name))
        {
            continue;
        }
        config.notification_apps.push(crate::settings::NotificationApp {
            name: name.clone(),
            enabled: false,
        });
    }
    if config.notification_apps.len() == before {
        return;
    }
    config
        .notification_apps
        .sort_by_key(|entry| entry.name.to_lowercase());
    crate::settings::store(&config);
    let _ = app.emit(APPS_EVENT, &config.notification_apps);
}

/// Lee las notificaciones actuales.
///
/// Devuelve los identificadores de todas y las que no están en `known`, estas
/// con el nombre de su aplicación y, si `details` es cierto, también su texto.
/// Sobre las ya conocidas no se lee nada: se llama una vez por segundo y la
/// lista del centro de notificaciones puede ser larga.
#[cfg(windows)]
fn fetch(known: &HashSet<u32>, details: bool) -> Option<(Vec<u32>, Vec<AppToast>)> {
    use windows::UI::Notifications::Management::{
        UserNotificationListener, UserNotificationListenerAccessStatus,
    };
    use windows::UI::Notifications::NotificationKinds;

    let listener = UserNotificationListener::Current().ok()?;

    let mut status = listener.GetAccessStatus().ok()?;
    // Solo se pide permiso la primera vez: con «Denegado» volver a pedirlo cada
    // segundo no serviría de nada.
    if status == UserNotificationListenerAccessStatus::Unspecified {
        status = listener.RequestAccessAsync().ok()?.join().ok()?;
    }
    if status != UserNotificationListenerAccessStatus::Allowed {
        return None;
    }

    let list = listener
        .GetNotificationsAsync(NotificationKinds::Toast)
        .ok()?
        .join()
        .ok()?;

    let mut ids = Vec::new();
    let mut fresh = Vec::new();
    for notification in list {
        let Ok(id) = notification.Id() else { continue };
        ids.push(id);
        if known.contains(&id) {
            continue;
        }

        let app = notification
            .AppInfo()
            .and_then(|info| info.DisplayInfo())
            .and_then(|display| display.DisplayName())
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        if let Ok(aumid) = notification.AppInfo().and_then(|info| info.AppUserModelId()) {
            remember_launch(id, aumid.to_string_lossy());
        }

        // Sin `details` solo interesa descubrir la aplicación: el texto no se lee.
        if !details {
            fresh.push(AppToast {
                id,
                app,
                title: String::new(),
                body: String::new(),
                avatar: None,
            });
            continue;
        }

        // El texto de un toast son sus elementos de texto, en orden: título y
        // luego las líneas del cuerpo.
        let mut lines: Vec<String> = Vec::new();
        if let Ok(binding) = notification
            .Notification()
            .and_then(|n| n.Visual())
            .and_then(|visual| visual.Bindings())
            .and_then(|bindings| bindings.GetAt(0))
        {
            if let Ok(texts) = binding.GetTextElements() {
                for text in texts {
                    if let Ok(value) = text.Text() {
                        let value = value.to_string_lossy();
                        if !value.trim().is_empty() {
                            lines.push(value);
                        }
                    }
                }
            }
        }
        if lines.is_empty() {
            continue;
        }
        let title = lines.remove(0);
        fresh.push(AppToast {
            id,
            app,
            title,
            body: lines.join(" "),
            avatar: None,
        });
    }
    Some((ids, fresh))
}

#[cfg(not(windows))]
fn fetch(_known: &HashSet<u32>, _details: bool) -> Option<(Vec<u32>, Vec<AppToast>)> {
    None
}

/// Arranca el sondeo de notificaciones.
///
/// Windows ofrece un evento (`NotificationChanged`), pero exige una tarea en
/// segundo plano de la que una app sin empaquetar no dispone: se sondea. Un
/// segundo basta, y mientras el ajuste está apagado no se lee nada.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        // Notificaciones ya vistas: las que había antes de empezar no cuentan,
        // o al activar el ajuste caería de golpe todo el historial.
        let mut seen: HashSet<u32> = HashSet::new();
        let mut primed = false;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));

            // Se consulta en cada vuelta, como en el resto de sondeos, para que
            // apagar el ajuste surta efecto en caliente. Apagado, se olvida el
            // estado: al reactivarlo se parte de cero.
            let config = crate::settings::current();
            if !config.app_notifications {
                primed = false;
                seen.clear();
                continue;
            }

            let Some((ids, fresh)) = fetch(&seen, primed) else {
                continue;
            };
            // Las aplicaciones se descubren también en la primera pasada, con
            // las notificaciones que ya había: así el panel de ajustes puede
            // ofrecer Teams o Outlook sin esperar a que lleguen mensajes.
            let names: HashSet<String> = fresh.iter().map(|toast| toast.app.clone()).collect();
            remember_apps(&app, &names);
            if primed {
                // Los ajustes pudieron cambiar al descubrir aplicaciones.
                let apps = crate::settings::current().notification_apps;
                for mut toast in fresh {
                    if allowed(&toast.app, &apps) {
                        // La foto va aparte: lee la base de datos de Windows y
                        // descarga una imagen. No tiene interruptor propio: la
                        // activación de la aplicación en ajustes es el permiso,
                        // y solo prospera si el aviso trae foto de un servidor
                        // de la lista de `avatars.rs`.
                        toast.avatar = crate::avatars::for_notification(toast.id);
                        let _ = app.emit(TOAST_EVENT, toast);
                    }
                }
            }
            seen = ids.into_iter().collect();
            primed = true;
        }
    });
}
