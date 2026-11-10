//! Estado de la batería y la corriente, para avisar al conectar el cargador.

use serde::Serialize;

/// Evento con el estado eléctrico del equipo.
pub const POWER_EVENT: &str = "power://changed";

/// Lo que la isla necesita saber de la alimentación del equipo.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerState {
    /// El equipo está enchufado a la corriente.
    pub plugged: bool,
    /// Carga restante, de 0 a 100. Vale 0 si Windows no sabe decirlo.
    pub percent: u8,
    /// Hay batería: un equipo de sobremesa no tiene nada que anunciar.
    pub has_battery: bool,
}

#[cfg(target_os = "windows")]
pub fn read_state() -> PowerState {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status = SYSTEM_POWER_STATUS::default();
    // Un fallo aquí deja la struct a cero, que se interpreta como «sin
    // batería»: es el estado que no dispara ningún aviso.
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err() {
        return PowerState::default();
    }

    // Windows codifica la ausencia de batería en un bit de banderas, y usa
    // 255 tanto para «desconocido» como para el porcentaje de un equipo sin
    // batería. Ambos casos deben quedar fuera.
    const NO_BATTERY: u8 = 128;
    const UNKNOWN_PERCENT: u8 = 255;

    let has_battery = status.BatteryFlag != NO_BATTERY && status.BatteryLifePercent != UNKNOWN_PERCENT;

    PowerState {
        // `ACLineStatus` vale 1 con corriente, 0 sin ella y 255 si se ignora.
        plugged: status.ACLineStatus == 1,
        percent: if status.BatteryLifePercent == UNKNOWN_PERCENT {
            0
        } else {
            status.BatteryLifePercent.min(100)
        },
        has_battery,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn read_state() -> PowerState {
    PowerState::default()
}
