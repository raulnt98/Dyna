//! Uso de CPU y memoria, para la vista de inicio.
//!
//! No hay sondeo propio ni evento: la vista de inicio pide los datos con
//! `system_stats` mientras está en pantalla, de modo que nada se mide (ni se
//! despierta) cuando no se ve.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    /// Uso del procesador desde la consulta anterior, de 0 a 100.
    pub cpu: u8,
    /// Memoria en uso, de 0 a 100.
    pub memory: u8,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
}

/// Tiempos de CPU de la consulta anterior: (inactivo, núcleo, usuario).
#[cfg(target_os = "windows")]
static LAST_CPU: std::sync::Mutex<Option<(u64, u64, u64)>> = std::sync::Mutex::new(None);

/// Porcentaje de uso entre dos lecturas de `GetSystemTimes`.
///
/// El tiempo de núcleo ya incluye el inactivo, de ahí que el total sea
/// núcleo + usuario y el ocupado, total − inactivo.
#[cfg(any(target_os = "windows", test))]
fn cpu_percent(before: (u64, u64, u64), after: (u64, u64, u64)) -> u8 {
    let idle = after.0.saturating_sub(before.0);
    let total = after.1.saturating_sub(before.1) + after.2.saturating_sub(before.2);
    if total == 0 {
        return 0;
    }
    (total.saturating_sub(idle) * 100 / total).min(100) as u8
}

#[cfg(target_os = "windows")]
fn read() -> SystemStats {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::GetSystemTimes;

    let ticks = |t: FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;

    let mut stats = SystemStats::default();

    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_ok() {
        let now = (ticks(idle), ticks(kernel), ticks(user));
        if let Ok(mut last) = LAST_CPU.lock() {
            // La primera consulta no tiene con qué compararse: 0 %.
            if let Some(before) = *last {
                stats.cpu = cpu_percent(before, now);
            }
            *last = Some(now);
        }
    }

    let mut memory = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut memory) }.is_ok() {
        const GB: f64 = 1024.0 * 1024.0 * 1024.0;
        stats.memory = memory.dwMemoryLoad.min(100) as u8;
        stats.memory_total_gb = (memory.ullTotalPhys as f64 / GB) as f32;
        stats.memory_used_gb = ((memory.ullTotalPhys - memory.ullAvailPhys) as f64 / GB) as f32;
    }
    stats
}

#[cfg(not(target_os = "windows"))]
fn read() -> SystemStats {
    SystemStats::default()
}

/// Uso de CPU y memoria. Lo llama la vista de inicio cada pocos segundos.
#[tauri::command(async)]
pub fn system_stats() -> SystemStats {
    read()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calcula_el_uso_entre_dos_lecturas() {
        // Pasan 100 ticks de núcleo (con 40 inactivos) y 50 de usuario.
        assert_eq!(cpu_percent((0, 0, 0), (40, 100, 50)), 73);
        assert_eq!(cpu_percent((10, 10, 10), (10, 10, 10)), 0);
    }
}
