//! Salidas de audio: listarlas y elegir la predeterminada (selector de la isla).
//!
//! Listar usa Core Audio (`IMMDeviceEnumerator`), igual que `volume.rs`. **Cambiar la salida predeterminada no
//! tiene API documentada**: Windows solo lo hace desde su propia interfaz, a través de `IPolicyConfig`, una interfaz
//! COM sin documentar que usan todos los conmutadores de audio (SoundSwitch, EarTrumpet…). Se declara aquí a mano.
//! Si en alguna versión de Windows cambiara, `audio_output_set` devuelve `false` y la lista no se altera.
//!
//! Solo se leen nombres de dispositivos de salida y se cambia cuál es el predeterminado; no se toca el audio.

use serde::Serialize;

/// Una salida de audio activa.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutput {
    /// Identificador del dispositivo para Core Audio.
    pub id: String,
    /// Nombre tal como lo muestra Windows («Altavoces (Realtek Audio)»).
    pub name: String,
    /// Tipo para elegir el icono: `headphones`, `speaker` o `monitor`.
    pub kind: String,
    /// `true` si es la salida predeterminada ahora.
    pub is_default: bool,
}

/// Adivina el tipo de dispositivo por su nombre: Windows no lo da de forma fiable. `laptop` dice si el equipo tiene batería,
/// que es lo que distingue los altavoces integrados de un portátil («Altavoces (Realtek Audio)») de unos altavoces de sobremesa.
///
/// Tipos: `headphones`, `laptop`, `tv`, `monitor` y `speaker` (el resto, también los altavoces Bluetooth).
pub fn guess_kind(name: &str, laptop: bool) -> &'static str {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    // Palabras sueltas (`tv`, `hp`): mirarlas como parte de otra («output», «chp») daría falsos positivos.
    let word = |w: &str| n.split(|c: char| !c.is_alphanumeric()).any(|part| part == w);

    if has(&["auricular", "headphone", "headset", "airpods", "buds", "pods", "casco", "p40i", "liberty", "q30", "wh-1000", "wf-1000"]) {
        "headphones"
    } else if word("tv") || has(&["televis", "samsung", "bravia", "hisense", "tcl", "chromecast", "roku"]) {
        "tv"
    } else if has(&["hdmi", "displayport", "display audio", "high definition audio", "monitor", "dell", "benq", "aoc", "viewsonic"]) || word("hp") || word("lg") {
        "monitor"
    } else if laptop && has(&["altavoces", "altavoz", "speakers", "speaker"]) && has(&["realtek", "intel", "conexant", "cirrus", "smart sound", "dolby", "waves", "amd audio", "integrad", "internal", "built-in"]) {
        "laptop"
    } else {
        "speaker"
    }
}

#[cfg(windows)]
mod imp {
    // Los métodos de la interfaz COM llevan los nombres de Windows (PascalCase).
    #![allow(non_snake_case)]

    use super::{guess_kind, AudioOutput};
    use windows::core::PCWSTR;
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{
        eCommunications, eConsole, eMultimedia, eRender, ERole, IMMDeviceEnumerator, MMDeviceEnumerator,
        DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
    use windows::Win32::System::Com::CoTaskMemFree;

    /// `IPolicyConfig` (Windows 7 en adelante). Solo se usa `SetDefaultEndpoint`; el resto de métodos están para que
    /// la tabla de funciones tenga el orden correcto, con punteros opacos en lugar de sus tipos reales.
    #[windows::core::interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: windows::core::IUnknown {
        fn GetMixFormat(&self, device: PCWSTR, format: *mut *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn GetDeviceFormat(&self, device: PCWSTR, default: i32, format: *mut *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn ResetDeviceFormat(&self, device: PCWSTR) -> windows::core::HRESULT;
        fn SetDeviceFormat(&self, device: PCWSTR, a: *mut core::ffi::c_void, b: *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn GetProcessingPeriod(&self, device: PCWSTR, default: i32, a: *mut i64, b: *mut i64) -> windows::core::HRESULT;
        fn SetProcessingPeriod(&self, device: PCWSTR, a: *mut i64) -> windows::core::HRESULT;
        fn GetShareMode(&self, device: PCWSTR, a: *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn SetShareMode(&self, device: PCWSTR, a: *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn GetPropertyValue(&self, device: PCWSTR, store: i32, key: *const core::ffi::c_void, value: *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn SetPropertyValue(&self, device: PCWSTR, store: i32, key: *const core::ffi::c_void, value: *mut core::ffi::c_void) -> windows::core::HRESULT;
        fn SetDefaultEndpoint(&self, device: PCWSTR, role: ERole) -> windows::core::HRESULT;
        fn SetEndpointVisibility(&self, device: PCWSTR, visible: i32) -> windows::core::HRESULT;
    }

    /// Clase COM de `IPolicyConfig`.
    const POLICY_CONFIG_CLIENT: windows::core::GUID = windows::core::GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    fn enumerator() -> Option<IMMDeviceEnumerator> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()
        }
    }

    pub fn list() -> Vec<AudioOutput> {
        let Some(enumerator) = enumerator() else {
            return Vec::new();
        };
        let laptop = crate::power::read_state().has_battery;
        unsafe {
            let default_id = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .ok()
                .and_then(|d| d.GetId().ok())
                .and_then(|id| id.to_string().ok());
            let Ok(collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
                return Vec::new();
            };
            let count = collection.GetCount().unwrap_or(0);
            let mut outputs = Vec::new();
            for index in 0..count {
                let Ok(device) = collection.Item(index) else { continue };
                let Some(id) = device.GetId().ok().and_then(|id| id.to_string().ok()) else { continue };
                let name = device
                    .OpenPropertyStore(STGM_READ)
                    .and_then(|store| store.GetValue(&PKEY_Device_FriendlyName))
                    .ok()
                    .and_then(|value| PropVariantToStringAlloc(&value).ok())
                    .and_then(|text| {
                        // La cadena la reserva COM: se copia y se libera.
                        let copy = text.to_string().ok();
                        CoTaskMemFree(Some(text.0 as *const core::ffi::c_void));
                        copy
                    })
                    .unwrap_or_default();
                if name.trim().is_empty() {
                    continue;
                }
                outputs.push(AudioOutput {
                    is_default: default_id.as_deref() == Some(id.as_str()),
                    kind: guess_kind(&name, laptop).to_string(),
                    id,
                    name,
                });
            }
            outputs
        }
    }

    pub fn set_default(id: &str) -> bool {
        // El identificador viene del frontend: solo se acepta uno que esté en la lista de salidas activas.
        if !list().iter().any(|o| o.id == id) {
            return false;
        }
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let Ok(policy) = CoCreateInstance::<_, IPolicyConfig>(&POLICY_CONFIG_CLIENT, None, CLSCTX_ALL) else {
                return false;
            };
            let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
            // Las tres funciones, o las aplicaciones de comunicaciones seguirían en la salida anterior.
            [eConsole, eMultimedia, eCommunications]
                .into_iter()
                .all(|role| policy.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role).is_ok())
        }
    }
}

/// Salidas de audio activas, la predeterminada marcada. Va fuera del hilo principal: toca COM.
#[tauri::command(async)]
pub fn audio_outputs() -> Vec<AudioOutput> {
    #[cfg(windows)]
    {
        imp::list()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Cambia la salida de audio predeterminada. `false` si el identificador no es de una salida activa o falla.
#[tauri::command(async)]
pub fn audio_output_set(id: String) -> bool {
    #[cfg(windows)]
    {
        imp::set_default(&id)
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adivina_el_tipo_por_el_nombre() {
        assert_eq!(guess_kind("Auriculares (soundcore P40i Hands-Free)", true), "headphones");
        assert_eq!(guess_kind("LG TV (NVIDIA High Definition Audio)", false), "tv");
        assert_eq!(guess_kind("SAMSUNG (AMD High Definition Audio Device)", false), "tv");
        assert_eq!(guess_kind("DELL U2720Q (NVIDIA High Definition Audio)", false), "monitor");
        assert_eq!(guess_kind("EDIFIER M60", false), "speaker");
        assert_eq!(guess_kind("Soundcore Motion B", true), "speaker");
        // Los altavoces integrados cuentan como portátil solo si el equipo tiene batería.
        assert_eq!(guess_kind("Altavoces (2- Realtek(R) Audio)", true), "laptop");
        assert_eq!(guess_kind("Altavoces (Realtek(R) Audio)", false), "speaker");
        // «output» o «chp» no son un televisor ni un HP.
        assert_eq!(guess_kind("Line output (USB Audio)", false), "speaker");
    }

    /// Fija como predeterminada la que ya lo es: comprueba que `IPolicyConfig` responde sin cambiar nada.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn fija_la_actual() {
        let current = imp::list().into_iter().find(|o| o.is_default).expect("sin salida predeterminada");
        assert!(imp::set_default(&current.id));
    }

    /// Lista las salidas de esta máquina: `cargo test --lib outputs -- --ignored --nocapture`.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn lista_real() {
        for output in imp::list() {
            println!("{} · {} · {} · {}", output.name, output.kind, output.is_default, output.id);
        }
    }
}
