//! Baterías de los dispositivos, para la página «Baterías» de la isla abierta.
//!
//! El portátil sale de `power.rs`. Los dispositivos Bluetooth conectados dan su carga por una propiedad que Windows
//! rellena cuando el dispositivo la informa (`{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2`, la que lee el panel de
//! Bluetooth de Configuración), pero en el nodo de uno de sus servicios, no en el dispositivo principal (ver
//! `bluetooth_batteries`). No todos los dispositivos la informan, y los que no, simplemente no salen. Solo se leen
//! nombres y porcentajes de dispositivos ya emparejados; no se escanea ni se envía nada fuera del equipo.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBattery {
    /// Identificador estable para la lista de la página.
    pub id: String,
    pub name: String,
    /// `laptop`, `headphones`, `mouse`, `keyboard`, `speaker` o `device`: elige el icono.
    pub kind: String,
    /// Carga, 0–100.
    pub percent: u8,
    /// Enchufado (solo el portátil lo sabe).
    pub charging: bool,
}

/// Tipo de dispositivo según su nombre: Windows da la clase del dispositivo, pero los nombres son más fiables en la
/// práctica (los auriculares con micrófono se anuncian como «manos libres» o «audio»).
pub fn guess_kind(name: &str) -> &'static str {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["mouse", "ratón", "raton", "mx master", "mx anywhere"]) {
        "mouse"
    } else if has(&["keyboard", "teclado", "keychron", "mx keys"]) {
        "keyboard"
    } else if has(&["buds", "airpods", "auricular", "headphone", "headset", "earbuds", "p40i", "liberty", "wh-", "wf-", "jabra", "p20i"]) {
        "headphones"
    } else if has(&["speaker", "altavoz", "motion", "edifier", "soundbar", "boombox", "flip", "charge ", "jbl", "blik"]) {
        "speaker"
    } else {
        "device"
    }
}

/// Dispositivos con batería, leídos de los nodos de dispositivo de Windows.
///
/// La carga **no está en el dispositivo Bluetooth principal** sino en uno de sus servicios: en unos auriculares
/// clásicos, el nodo «Hands-Free AG» (`BTHENUM\{0000111E-…}`); en uno de bajo consumo, el del perfil HID o el de
/// batería. Windows la pone ahí, solo mientras el dispositivo está conectado, en la propiedad
/// `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` (un byte, 0–100). Por eso se recorren **todos los nodos presentes** y
/// se leen los que la tengan, en lugar de preguntar por los dispositivos Bluetooth conectados (que daba vacío).
/// Medido con unos auriculares reales: el nodo «Hands-Free AG» daba 90 y el dispositivo principal, nada.
#[cfg(target_os = "windows")]
fn bluetooth_batteries() -> Vec<DeviceBattery> {
    use windows::core::PCWSTR;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_DevNode_PropertyW, CM_Get_Device_ID_ListW, CM_Get_Device_ID_List_SizeW,
        CM_Get_Parent, CM_Locate_DevNodeW, CM_GETIDLIST_FILTER_PRESENT, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
    };
    use windows::Win32::Devices::Properties::DEVPROPTYPE;
    use windows::Win32::Foundation::DEVPROPKEY;
    use windows::core::GUID;

    const BATTERY: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5),
        pid: 2,
    };
    const FRIENDLY_NAME: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0xA45C254E_DF1C_4EFD_8020_67D146A850E0),
        pid: 14,
    };
    /// `DEVPKEY_Bluetooth_...` de conexión: verdadero solo mientras el dispositivo está conectado. Windows conserva la
    /// última carga conocida en los nodos de dispositivos que ya no están conectados (se vio con un altavoz apagado), y
    /// sin este filtro saldrían con un porcentaje viejo.
    const CONNECTED: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0x83DA6326_97A6_4088_9453_A1923F573B29),
        pid: 15,
    };
    const DEVICE_DESC: DEVPROPKEY = DEVPROPKEY {
        fmtid: GUID::from_u128(0xA45C254E_DF1C_4EFD_8020_67D146A850E0),
        pid: 2,
    };

    fn property(devinst: u32, key: &DEVPROPKEY) -> Option<Vec<u8>> {
        let mut kind = DEVPROPTYPE(0);
        let mut size = 0u32;
        // Primero se pregunta el tamaño (devuelve BUFFER_SMALL) y luego se lee.
        unsafe {
            let _ = CM_Get_DevNode_PropertyW(devinst, key, &mut kind, None, &mut size, 0);
        }
        if size == 0 {
            return None;
        }
        let mut buffer = vec![0u8; size as usize];
        let result = unsafe {
            CM_Get_DevNode_PropertyW(devinst, key, &mut kind, Some(buffer.as_mut_ptr()), &mut size, 0)
        };
        (result == CR_SUCCESS).then(|| {
            buffer.truncate(size as usize);
            buffer
        })
    }
    fn text(devinst: u32, key: &DEVPROPKEY) -> Option<String> {
        let bytes = property(devinst, key)?;
        let wide: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let value = String::from_utf16_lossy(&wide).trim_end_matches('\0').trim().to_string();
        (!value.is_empty()).then_some(value)
    }
    /// Nombre legible del nodo: el suyo, o el de su padre si el nodo es un servicio con nombre genérico.
    fn name_of(devinst: u32) -> Option<String> {
        let own = |node: u32| text(node, &FRIENDLY_NAME).or_else(|| text(node, &DEVICE_DESC));
        let mut name = own(devinst)?;
        for suffix in [" Hands-Free AG", " Hands-Free", " Manos libres AG", " Manos libres"] {
            if let Some(stripped) = name.strip_suffix(suffix) {
                name = stripped.to_string();
            }
        }
        // Los servicios sin nombre propio (HID, batería) lo toman del dispositivo del que cuelgan.
        let generic = ["hid-compliant", "dispositivo de entrada", "bluetooth le", "dispositivo compatible", "battery"];
        if generic.iter().any(|g| name.to_lowercase().contains(g)) {
            let mut parent = 0u32;
            if unsafe { CM_Get_Parent(&mut parent, devinst, 0) } == CR_SUCCESS {
                if let Some(parent_name) = own(parent) {
                    name = parent_name;
                }
            }
        }
        Some(name)
    }

    let mut length = 0u32;
    if unsafe { CM_Get_Device_ID_List_SizeW(&mut length, PCWSTR::null(), CM_GETIDLIST_FILTER_PRESENT) } != CR_SUCCESS
        || length == 0
    {
        return Vec::new();
    }
    let mut ids = vec![0u16; length as usize];
    if unsafe { CM_Get_Device_ID_ListW(PCWSTR::null(), &mut ids, CM_GETIDLIST_FILTER_PRESENT) } != CR_SUCCESS {
        return Vec::new();
    }

    let all: Vec<String> = ids
        .split(|c| *c == 0)
        .filter(|id| !id.is_empty())
        .map(|id| String::from_utf16_lossy(id))
        .collect();
    let locate = |id: &str| -> Option<u32> {
        let mut wide: Vec<u16> = id.encode_utf16().collect();
        wide.push(0);
        let mut devinst = 0u32;
        (unsafe { CM_Locate_DevNodeW(&mut devinst, PCWSTR(wide.as_ptr()), CM_LOCATE_DEVNODE_NORMAL) } == CR_SUCCESS)
            .then_some(devinst)
    };
    // Direcciones de los dispositivos Bluetooth clásicos conectados ahora: el nodo principal `BTHENUM\DEV_<dirección>`.
    let connected: Vec<String> = all
        .iter()
        .filter_map(|id| {
            let upper = id.to_uppercase();
            let address = upper.strip_prefix("BTHENUM\\DEV_")?.split('\\').next()?.to_string();
            let node = locate(id)?;
            let on = property(node, &CONNECTED).is_some_and(|bytes| bytes.first().is_some_and(|b| *b != 0));
            on.then_some(address)
        })
        .collect();

    let mut out = Vec::new();
    for id in ids.split(|c| *c == 0).filter(|id| !id.is_empty()) {
        // Solo Bluetooth (clásico y de bajo consumo): los demás nodos con esta propiedad no son baterías de periféricos.
        let upper = String::from_utf16_lossy(id).to_uppercase();
        if !(upper.starts_with("BTHENUM\\") || upper.starts_with("BTHLE\\") || upper.starts_with("HID\\{00001812")) {
            continue;
        }
        // Un servicio de un dispositivo clásico acaba en `…&<dirección>_C00000000`: solo cuenta si está conectado.
        if upper.starts_with("BTHENUM\\{") {
            let address = upper.rsplit('&').next().and_then(|tail| tail.split('_').next()).unwrap_or("");
            if !connected.iter().any(|c| c == address) {
                continue;
            }
        }
        let Some(devinst) = locate(&String::from_utf16_lossy(id)) else { continue };
        let Some(percent) = property(devinst, &BATTERY).and_then(|bytes| bytes.first().copied()) else {
            continue;
        };
        if percent > 100 {
            continue;
        }
        let Some(name) = name_of(devinst) else { continue };
        out.push(DeviceBattery { id: upper, kind: guess_kind(&name).to_string(), name, percent, charging: false });
    }
    // Un mismo dispositivo puede aparecer por varios nodos: se cuenta una vez, con la última lectura.
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

#[cfg(not(target_os = "windows"))]
fn bluetooth_batteries() -> Vec<DeviceBattery> {
    Vec::new()
}

/// Portátil primero y luego los dispositivos, con su carga. Vacío en un equipo sin batería y sin dispositivos que la
/// informen. Hace consultas de Windows que pueden tardar: va en un hilo aparte.
#[tauri::command(async)]
pub fn batteries_list() -> Vec<DeviceBattery> {
    let mut list = Vec::new();
    let power = crate::power::read_state();
    if power.has_battery {
        list.push(DeviceBattery {
            id: "laptop".into(),
            name: "Notebook".into(),
            kind: "laptop".into(),
            percent: power.percent,
            charging: power.plugged,
        });
    }
    list.extend(bluetooth_batteries());
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Consulta real: `cargo test --lib batteries -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn consulta_real() {
        println!("{:?}", batteries_list());
    }

    #[test]
    fn adivina_el_tipo_por_el_nombre() {
        assert_eq!(guess_kind("Logitech MX Master 3"), "mouse");
        assert_eq!(guess_kind("Teclado Bluetooth"), "keyboard");
        assert_eq!(guess_kind("soundcore P40i"), "headphones");
        assert_eq!(guess_kind("EDIFIER M60"), "speaker");
        assert_eq!(guess_kind("Cosa rara"), "device");
    }
}
