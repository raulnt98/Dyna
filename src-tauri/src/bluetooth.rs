//! Dispositivos Bluetooth conectados, para avisar al conectar o desconectar uno.
//!
//! Se usa la enumeración de dispositivos de Windows con el filtro de «conectado»
//! (clásicos y de bajo consumo). Los eventos de `DeviceWatcher` exigen mantener
//! vivos objetos WinRT y manejadores; con una consulta cada pocos segundos basta
//! y no despierta al frontend salvo ante un cambio.
//!
//! Solo se leen los nombres de dispositivos ya emparejados: no se escanea ni se
//! envía nada fuera del equipo.

use serde::Serialize;

/// Evento que se emite al conectar o desconectar un dispositivo Bluetooth.
pub const BLUETOOTH_EVENT: &str = "bluetooth://changed";

/// Lo que la isla necesita para dibujar el aviso.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothChange {
    /// Nombre del dispositivo tal como lo muestra Windows.
    pub name: String,
    /// `true` al conectarse, `false` al desconectarse.
    pub connected: bool,
}

/// Nombres de los dispositivos conectados ahora mismo, ordenados y sin repetir.
///
/// `None` si la consulta falla (Bluetooth apagado o sin adaptador, servicio
/// detenido): un fallo no debe leerse como «se desconectó todo», así que quien
/// llama se salta esa vuelta en lugar de comparar contra una lista vacía.
#[cfg(target_os = "windows")]
pub fn connected_names() -> Option<Vec<String>> {
    use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice, BluetoothLEDevice};
    use windows::Devices::Enumeration::DeviceInformation;
    use windows::core::HSTRING;

    fn find(selector: windows::core::Result<HSTRING>) -> Option<Vec<String>> {
        let devices = DeviceInformation::FindAllAsyncAqsFilter(&selector.ok()?)
            .ok()?
            .join()
            .ok()?;
        Some(
            devices
                .into_iter()
                .filter_map(|device| device.Name().ok())
                .map(|name| name.to_string())
                .collect(),
        )
    }

    let classic = find(BluetoothDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    ));
    let low_energy = find(BluetoothLEDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    ));
    // Si fallan las dos, no hay lectura; con una basta.
    if classic.is_none() && low_energy.is_none() {
        return None;
    }

    // Unos auriculares pueden salir por las dos vías: se cuentan una vez. Los
    // nombres vacíos son enlaces internos sin nada que enseñar.
    let mut names: Vec<String> = classic
        .into_iter()
        .chain(low_energy)
        .flatten()
        .filter(|name| !name.trim().is_empty())
        .collect();
    names.sort();
    names.dedup();
    Some(names)
}

#[cfg(not(target_os = "windows"))]
pub fn connected_names() -> Option<Vec<String>> {
    None
}

/// Cambios entre dos lecturas: primero las desconexiones, luego las conexiones.
pub fn diff(before: &[String], after: &[String]) -> Vec<BluetoothChange> {
    let gone = before.iter().filter(|name| !after.contains(name)).map(|name| BluetoothChange {
        name: name.clone(),
        connected: false,
    });
    let new = after.iter().filter(|name| !before.contains(name)).map(|name| BluetoothChange {
        name: name.clone(),
        connected: true,
    });
    gone.chain(new).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn detecta_conexiones_y_desconexiones() {
        let changes = diff(&names(&["Ratón", "Auriculares"]), &names(&["Auriculares", "Teclado"]));
        assert_eq!(changes.len(), 2);
        assert_eq!((changes[0].name.as_str(), changes[0].connected), ("Ratón", false));
        assert_eq!((changes[1].name.as_str(), changes[1].connected), ("Teclado", true));
    }

    #[test]
    fn sin_cambios_no_avisa() {
        assert!(diff(&names(&["A"]), &names(&["A"])).is_empty());
    }
}
