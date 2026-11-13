//! Estado de la conexión Wi-Fi, para avisar cuando el equipo se conecta.

use serde::Serialize;

/// Evento que se emite al conectarse a una red Wi-Fi.
pub const WIFI_EVENT: &str = "network://wifi";

/// Lo que la isla necesita saber de la conexión inalámbrica.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiState {
    /// Nombre de la red a la que está conectado, o `None` si no hay conexión.
    pub ssid: Option<String>,
    /// Calidad de la señal, de 0 a 100.
    pub signal: u8,
}

#[cfg(target_os = "windows")]
pub fn read_state() -> WifiState {
    use std::ffi::c_void;
    use windows::Win32::Foundation::{HANDLE, WIN32_ERROR};
    use windows::Win32::NetworkManagement::WiFi::{
        wlan_interface_state_connected, wlan_intf_opcode_current_connection, WlanCloseHandle,
        WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface,
        WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO_LIST,
    };

    // Sin servicio WLAN (equipo de sobremesa sin tarjeta, servicio detenido) las
    // llamadas fallan: se interpreta como «sin Wi-Fi», que no avisa de nada.
    let mut state = WifiState::default();

    unsafe {
        let mut negotiated = 0u32;
        let mut client = HANDLE::default();
        if WIN32_ERROR(WlanOpenHandle(2, None, &mut negotiated, &mut client)).is_err() {
            return state;
        }

        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WIN32_ERROR(WlanEnumInterfaces(client, None, &mut list)).is_ok() && !list.is_null() {
            let interfaces = std::slice::from_raw_parts(
                (*list).InterfaceInfo.as_ptr(),
                (*list).dwNumberOfItems as usize,
            );

            for interface in interfaces {
                if interface.isState != wlan_interface_state_connected {
                    continue;
                }

                let mut size = 0u32;
                let mut data: *mut c_void = std::ptr::null_mut();
                let status = WlanQueryInterface(
                    client,
                    &interface.InterfaceGuid,
                    wlan_intf_opcode_current_connection,
                    None,
                    &mut size,
                    &mut data,
                    None,
                );
                if WIN32_ERROR(status).is_ok() && !data.is_null() {
                    let attrs = &*(data as *const WLAN_CONNECTION_ATTRIBUTES);
                    let ssid = &attrs.wlanAssociationAttributes.dot11Ssid;
                    let len = (ssid.uSSIDLength as usize).min(ssid.ucSSID.len());
                    // Un SSID no tiene por qué ser UTF-8 válido; lo inválido se
                    // sustituye en vez de descartar la conexión.
                    state.ssid = Some(String::from_utf8_lossy(&ssid.ucSSID[..len]).into_owned());
                    state.signal = attrs.wlanAssociationAttributes.wlanSignalQuality.min(100) as u8;
                    WlanFreeMemory(data);
                    break;
                }
            }
            WlanFreeMemory(list as *const c_void);
        }

        WlanCloseHandle(client, None);
    }

    state
}

#[cfg(not(target_os = "windows"))]
pub fn read_state() -> WifiState {
    WifiState::default()
}

/// Evento que se emite al perder o recuperar la salida a internet.
pub const INTERNET_EVENT: &str = "network://internet";

/// Si el equipo tiene salida a internet, no solo red local.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternetState {
    pub online: bool,
}

/// ¿Hay salida a internet? Pregunta al Administrador de redes de Windows,
/// que ya sondea por su cuenta (NCSI): no se envía tráfico desde aquí. Estar
/// conectado al router no basta, y es justo el caso que interesa avisar.
///
/// Ante un fallo se devuelve `true`: no avisar de una pérdida que no se ha
/// podido comprobar es preferible a un aviso falso.
#[cfg(target_os = "windows")]
pub fn has_internet() -> bool {
    use windows::Win32::Networking::NetworkListManager::{INetworkListManager, NetworkListManager};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };

    // Banderas NLM_CONNECTIVITY_IPV4_INTERNET (0x40) e IPV6_INTERNET (0x400).
    const INTERNET: i32 = 0x40 | 0x400;

    unsafe {
        // Si COM ya estaba iniciado en este hilo el error se ignora a propósito.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(manager) = CoCreateInstance::<_, INetworkListManager>(&NetworkListManager, None, CLSCTX_ALL) else {
            return true;
        };
        match manager.GetConnectivity() {
            Ok(flags) => flags.0 & INTERNET != 0,
            Err(_) => true,
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn has_internet() -> bool {
    true
}
