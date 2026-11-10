//! Volumen del dispositivo de audio predeterminado de Windows.
//!
//! Usa la API Core Audio (`IAudioEndpointVolume`), que controla el volumen
//! maestro del sistema: el mismo que mueve la tecla de volumen del teclado.

#[cfg(target_os = "windows")]
mod imp {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };

    /// Obtiene el control de volumen del dispositivo de salida activo.
    ///
    /// COM se inicializa en cada llamada: si ya lo estaba en este hilo, el
    /// error se ignora a propósito.
    fn endpoint() -> Option<IAudioEndpointVolume> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
        }
    }

    /// Volumen maestro actual, de 0 a 100.
    pub fn get() -> u8 {
        let Some(endpoint) = endpoint() else {
            return 0;
        };
        unsafe {
            endpoint
                .GetMasterVolumeLevelScalar()
                .map(|level| (level * 100.0).round().clamp(0.0, 100.0) as u8)
                .unwrap_or(0)
        }
    }

    /// Ajusta el volumen maestro. `value` va de 0 a 100.
    pub fn set(value: u8) -> bool {
        let Some(endpoint) = endpoint() else {
            return false;
        };
        let level = (value.min(100) as f32) / 100.0;
        unsafe { endpoint.SetMasterVolumeLevelScalar(level, std::ptr::null()).is_ok() }
    }

    /// Indica si el audio está silenciado.
    pub fn muted() -> bool {
        let Some(endpoint) = endpoint() else {
            return false;
        };
        unsafe { endpoint.GetMute().map(|m| m.as_bool()).unwrap_or(false) }
    }

    /// Silencia o restaura el audio.
    pub fn set_muted(muted: bool) -> bool {
        let Some(endpoint) = endpoint() else {
            return false;
        };
        unsafe { endpoint.SetMute(muted, std::ptr::null()).is_ok() }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn get() -> u8 {
        0
    }
    pub fn set(_value: u8) -> bool {
        false
    }
    pub fn muted() -> bool {
        false
    }
    pub fn set_muted(_muted: bool) -> bool {
        false
    }
}

pub use imp::{get, muted, set, set_muted};
