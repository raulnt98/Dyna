//! Nivel del audio que suena, para que la onda de la isla siga la música de verdad.
//!
//! Escucha la salida del sistema por «loopback» de WASAPI (lo mismo que hace un grabador de
//! lo que suena) y calcula cinco bandas de frecuencia con una FFT. **Solo se mide el nivel**:
//! las muestras se procesan en el momento y se descartan, nada se guarda ni sale del equipo, y
//! a la isla solo llegan cinco números entre 0 y 1.
//!
//! Es un ajuste propio y nace apagado (`audio_visualizer`): capturar el audio del sistema es
//! sensible aunque no se conserve. El ajuste se consulta en cada vuelta; apagado, el hilo
//! suelta el dispositivo y no captura nada.

use serde::Serialize;

/// Evento con los niveles de las cinco bandas, de grave a agudo, entre 0 y 1.
pub const AUDIO_EVENT: &str = "audio://levels";

/// Número de barras del visualizador, de grave a agudo.
pub const BAND_COUNT: usize = 8;

/// Límites del espectro que se dibuja, en hercios. Por debajo de 50 Hz no hay casi nada que se vea y
/// por encima de 16 kHz casi nadie oye; entre medias se reparten las barras en escala logarítmica,
/// que es como oímos (cada octava pesa lo mismo).
const LOW_HZ: f32 = 50.0;
const HIGH_HZ: f32 = 16_000.0;

/// Muestras por bloque de análisis. 2048 a 48 kHz son 43 ms y dan bins de 23 Hz: con 1024 las
/// barras de grave caían en un solo bin y el grave se movía a saltos.
pub const WINDOW: usize = 2048;
/// Cada cuántas muestras se analiza un bloque nuevo (el bloque se solapa en tres cuartas partes con el
/// anterior): 512 a 48 kHz son 10,7 ms, unas 94 actualizaciones por segundo. Más solape no cuesta casi
/// nada y hace el movimiento más continuo que analizar de 1024 en 1024.
pub const HOP: usize = WINDOW / 4;

/// Cuánto por debajo del pico reciente se dibuja todavía algo (rango dinámico de las barras, en dB).
const RANGE_DB: f32 = 36.0;
/// El pico de referencia nunca baja de aquí: una música muy flojita se amplifica hasta este punto, pero
/// no más allá.
const FLOOR_DB: f32 = -50.0;
/// Puerta de ruido, en dB absolutos: por debajo de esto una banda vale cero, sea cual sea el pico. Es el
/// ruido de fondo de una salida de audio; sin puerta, la ganancia automática lo amplificaría en las
/// pausas y las barras bailarían con el silencio.
const GATE_DB: f32 = -70.0;
/// Constante de tiempo con la que bajan las barras al cesar el sonido. Medida en segundos y no en bloques:
/// así no cambia si se cambia el solape o la tasa de muestreo.
const RELEASE_TAU_SECS: f32 = 0.06;
/// Cuánto «olvida» el pico de referencia por segundo: así, si baja el volumen, las barras se readaptan
/// en pocos segundos en vez de quedarse pequeñas para siempre.
const REFERENCE_DECAY_DB_PER_SEC: f32 = 4.0;

/// Niveles de las barras, de grave a agudo, entre 0 y 1.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Levels(pub [f32; BAND_COUNT]);

/// FFT de Cooley-Tukey in situ, con la longitud potencia de dos. Basta para 1024 muestras
/// 40 veces por segundo; no merece una dependencia.
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    // Reordenación por inversión de bits.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = next;
            }
        }
        len <<= 1;
    }
}

/// Límites (inferior y superior, en Hz) de cada barra: `BAND_COUNT` tramos de igual anchura en escala
/// logarítmica entre `LOW_HZ` y `HIGH_HZ` (o la frecuencia de Nyquist, si es menor).
fn band_edges(rate: f32) -> [(f32, f32); BAND_COUNT] {
    let top = HIGH_HZ.min(rate * 0.47);
    let ratio = (top / LOW_HZ).powf(1.0 / BAND_COUNT as f32);
    std::array::from_fn(|i| (LOW_HZ * ratio.powi(i as i32), LOW_HZ * ratio.powi(i as i32 + 1)))
}

/// Convierte bloques de audio en niveles de barra, recordando lo necesario entre un bloque y el
/// siguiente: la referencia de volumen (para normalizar) y el suavizado de cada barra.
pub struct Analyzer {
    /// Pico reciente en dB, al que se miden las barras (ganancia automática).
    reference_db: f32,
    /// Último nivel de cada barra, con el suavizado ya aplicado.
    smooth: [f32; BAND_COUNT],
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer { reference_db: FLOOR_DB, smooth: [0.0; BAND_COUNT] }
    }

    /// Niveles actuales, ya suavizados.
    pub fn levels(&self) -> [f32; BAND_COUNT] {
        self.smooth
    }

    /// Analiza un bloque de `WINDOW` muestras mono a `rate` Hz y devuelve los niveles suavizados.
    ///
    /// Cada barra mide la **energía** de su tramo (raíz de la suma de potencias de sus bins), no el pico
    /// de un bin: con tramos logarítmicos, las barras de agudo abarcan muchos más bins que las de grave,
    /// y como la música tiene aproximadamente la misma energía en cada octava, así todas se mueven de
    /// forma parecida sin tener que compensarlas a mano. Una senoide pura da igual su amplitud.
    ///
    /// Después se normaliza respecto al pico reciente (`RANGE_DB` por debajo de él es cero): el visualizador
    /// se ve igual de vivo con el volumen alto que con el bajo, que es lo que más fallaba antes.
    pub fn process(&mut self, block: &[f32], rate: f32) -> [f32; BAND_COUNT] {
        let n = block.len();
        debug_assert!(n.is_power_of_two());
        let mut re: Vec<f32> = block
            .iter()
            .enumerate()
            .map(|(i, s)| s * (0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n as f32 - 1.0)).cos()))
            .collect();
        let mut im = vec![0.0f32; n];
        fft(&mut re, &mut im);

        let bin = rate / n as f32;
        let mut db = [f32::NEG_INFINITY; BAND_COUNT];
        for (band, (low, high)) in band_edges(rate).iter().enumerate() {
            let first = ((low / bin).ceil() as usize).max(1);
            let last = ((high / bin).floor() as usize).min(n / 2 - 1).max(first);
            let energy: f32 = (first..=last).map(|k| re[k] * re[k] + im[k] * im[k]).sum();
            // Amplitud equivalente: con la ventana de Hann, una senoide de amplitud 1 da n/4 en su bin.
            let magnitude = energy.sqrt() * 4.0 / n as f32;
            db[band] = 20.0 * (magnitude + 1e-9).log10();
        }

        // La referencia sube de golpe con un pico nuevo y baja despacio.
        let peak = db.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let hop_secs = HOP as f32 / rate;
        self.reference_db = (self.reference_db - REFERENCE_DECAY_DB_PER_SEC * hop_secs).max(FLOOR_DB).max(peak);

        // Suelo de las barras: `RANGE_DB` por debajo del pico, pero nunca por debajo de la puerta de ruido.
        let lower = (self.reference_db - RANGE_DB).max(GATE_DB);
        let span = (self.reference_db - lower).max(1.0);
        for band in 0..BAND_COUNT {
            let raw = ((db[band] - lower) / span).clamp(0.0, 1.0);
            // Sube de golpe y baja con una constante de tiempo fija: sin eso las barras parpadean.
            let release = (-hop_secs / RELEASE_TAU_SECS).exp();
            self.smooth[band] = if raw > self.smooth[band] { raw } else { self.smooth[band] * release + raw * (1.0 - release) };
        }
        self.smooth
    }

    /// Sin audio nuevo (nada suena): las barras se relajan a cero y la referencia se olvida.
    pub fn relax(&mut self, elapsed_secs: f32) {
        let fade = (-elapsed_secs / (RELEASE_TAU_SECS * 1.4)).exp();
        for v in self.smooth.iter_mut() {
            *v *= fade;
        }
        self.reference_db = (self.reference_db - REFERENCE_DECAY_DB_PER_SEC * elapsed_secs).max(FLOOR_DB);
    }
}

#[cfg(target_os = "windows")]
mod capture {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
        AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};

    pub struct Loopback {
        client: IAudioClient,
        capture: IAudioCaptureClient,
        pub rate: f32,
        channels: usize,
        float: bool,
        bits: u16,
        pub device_id: String,
    }

    impl Loopback {
        /// Abre la salida predeterminada en modo loopback. `None` si no hay dispositivo o
        /// Windows no deja abrirlo: se reintenta más tarde.
        pub fn open() -> Option<Loopback> {
            unsafe {
                let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
                let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
                let id = device.GetId().ok()?;
                let device_id = id.to_string().unwrap_or_default();
                CoTaskMemFree(Some(id.0 as *const _));
                let client: IAudioClient = device.Activate(CLSCTX_ALL, None).ok()?;
                let format: *mut WAVEFORMATEX = client.GetMixFormat().ok()?;
                // `WAVEFORMATEX` está empaquetada: se copian los campos, no se toman referencias.
                let (tag, channels, rate, bits) = (
                    (*format).wFormatTag,
                    (*format).nChannels as usize,
                    (*format).nSamplesPerSec as f32,
                    (*format).wBitsPerSample,
                );
                // 3 = coma flotante; 0xFFFE = formato extensible, que en la mezcla del sistema
                // es de 32 bits en coma flotante.
                let float = tag == 3 || (tag == 0xFFFE && bits == 32);
                let init = client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 2_000_000, 0, format, None);
                CoTaskMemFree(Some(format as *const _));
                init.ok()?;
                let capture: IAudioCaptureClient = client.GetService().ok()?;
                client.Start().ok()?;
                Some(Loopback { client, capture, rate, channels: channels.max(1), float, bits, device_id })
            }
        }

        /// Añade a `out` las muestras (mono) disponibles. `Err` si el dispositivo falló.
        pub fn read(&self, out: &mut Vec<f32>) -> Result<(), ()> {
            unsafe {
                loop {
                    let packet = self.capture.GetNextPacketSize().map_err(|_| ())?;
                    if packet == 0 {
                        return Ok(());
                    }
                    let mut data: *mut u8 = std::ptr::null_mut();
                    let mut frames = 0u32;
                    let mut flags = 0u32;
                    self.capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None).map_err(|_| ())?;
                    // Bandera «silencio»: el búfer no trae datos válidos.
                    let silent = flags & 0x2 != 0;
                    for frame in 0..frames as usize {
                        if silent || data.is_null() {
                            out.push(0.0);
                            continue;
                        }
                        let mut sum = 0.0f32;
                        for c in 0..self.channels {
                            let at = frame * self.channels + c;
                            sum += if self.float {
                                (data as *const f32).add(at).read_unaligned()
                            } else if self.bits == 16 {
                                (data as *const i16).add(at).read_unaligned() as f32 / 32768.0
                            } else {
                                0.0
                            };
                        }
                        out.push(sum / self.channels as f32);
                    }
                    self.capture.ReleaseBuffer(frames).map_err(|_| ())?;
                }
            }
        }

        /// ¿Sigue siendo la salida predeterminada el dispositivo que se abrió?
        pub fn is_default(&self) -> bool {
            unsafe {
                let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
                    return true;
                };
                let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) else {
                    return true;
                };
                let Ok(id) = device.GetId() else { return true };
                let same = id.to_string().map(|s| s == self.device_id).unwrap_or(true);
                CoTaskMemFree(Some(id.0 as *const _));
                same
            }
        }
    }

    impl Drop for Loopback {
        fn drop(&mut self) {
            unsafe {
                let _ = self.client.Stop();
            }
        }
    }

    // Se usa `Interface` en la firma de `GetService`; se deja importado a propósito.
    #[allow(dead_code)]
    fn _interface<T: Interface>() {}
}

/// Arranca el hilo del visualizador. Sin el ajuste no captura nada.
#[cfg(target_os = "windows")]
pub fn start(app: tauri::AppHandle) {
    use tauri::Emitter;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut loopback: Option<capture::Loopback> = None;
        let mut samples: Vec<f32> = Vec::new();
        let mut analyzer = Analyzer::new();
        let mut last_sent = [f32::MAX; BAND_COUNT];
        let mut last_check = std::time::Instant::now();
        let mut last_loop = std::time::Instant::now();

        loop {
            let config = crate::settings::current();
            if !config.audio_visualizer {
                // Apagado: se suelta el dispositivo y se duerme largo.
                if loopback.take().is_some() {
                    analyzer = Analyzer::new();
                    let _ = app.emit(AUDIO_EVENT, Levels([0.0; BAND_COUNT]));
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }

            if loopback.is_none() {
                loopback = capture::Loopback::open();
                samples.clear();
                if loopback.is_none() {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    continue;
                }
            }
            let device = loopback.as_ref().unwrap();

            // Si cambia la salida predeterminada (se conectan auriculares), se reabre.
            if last_check.elapsed().as_secs() >= 3 {
                last_check = std::time::Instant::now();
                if !device.is_default() {
                    loopback = None;
                    continue;
                }
            }
            if device.read(&mut samples).is_err() {
                loopback = None;
                continue;
            }

            // Análisis: bloques de 2048 muestras con un paso de 512 (unas 94 veces por segundo a 48 kHz); a la
            // isla se le avisa una vez por vuelta, cada ~16 ms (60 por segundo).
            let elapsed = last_loop.elapsed().as_secs_f32();
            last_loop = std::time::Instant::now();
            let mut computed = false;
            while samples.len() >= WINDOW {
                let block: Vec<f32> = samples[..WINDOW].to_vec();
                samples.drain(..HOP);
                analyzer.process(&block, device.rate);
                computed = true;
            }
            // Sin muestras nuevas (nada suena) las barras se relajan a cero.
            if !computed {
                analyzer.relax(elapsed);
            }
            let rounded = analyzer.levels().map(|v| ((v * 100.0).round() / 100.0).clamp(0.0, 1.0));
            let moved = rounded.iter().zip(last_sent.iter()).any(|(a, b)| (a - b).abs() >= 0.01);
            if moved || (rounded == [0.0; BAND_COUNT] && last_sent != [0.0; BAND_COUNT]) {
                last_sent = rounded;
                let _ = app.emit(AUDIO_EVENT, Levels(rounded));
            }
            // Si se acumuló demasiado (la isla tardó), se descarta lo viejo.
            if samples.len() > WINDOW * 8 {
                samples.clear();
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start(_app: tauri::AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n).map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin()).collect()
    }

    /// Barra más alta de un resultado.
    fn loudest(levels: &[f32; BAND_COUNT]) -> usize {
        levels.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0
    }

    /// Resultado tras dar el mismo bloque varias veces, como en la vida real (el suavizado y la
    /// referencia necesitan unos bloques para asentarse).
    fn settled(block: &[f32], rate: f32) -> [f32; BAND_COUNT] {
        let mut analyzer = Analyzer::new();
        for _ in 0..6 {
            analyzer.process(block, rate);
        }
        analyzer.levels()
    }

    #[test]
    fn las_bandas_cubren_el_espectro_sin_huecos() {
        let edges = band_edges(48_000.0);
        assert!((edges[0].0 - LOW_HZ).abs() < 0.01);
        assert!((edges[BAND_COUNT - 1].1 - HIGH_HZ).abs() < 1.0);
        for pair in edges.windows(2) {
            assert!((pair[0].1 - pair[1].0).abs() < 0.01, "hueco entre bandas");
        }
        // Escala logarítmica: la razón entre el techo y el suelo de cada banda es la misma.
        let ratios: Vec<f32> = edges.iter().map(|(low, high)| high / low).collect();
        assert!(ratios.iter().all(|r| (r - ratios[0]).abs() < 0.001));
    }

    #[test]
    fn cada_barra_responde_a_su_tono() {
        let rate = 48_000.0;
        let edges = band_edges(rate);
        for (band, (low, high)) in edges.iter().enumerate() {
            let center = (low * high).sqrt();
            let levels = settled(&sine(center, rate, WINDOW, 0.4), rate);
            assert_eq!(loudest(&levels), band, "{center:.0} Hz → {levels:?}");
            assert!(levels[band] > 0.85, "{center:.0} Hz → {levels:?}");
        }
    }

    #[test]
    fn un_tono_no_enciende_las_barras_lejanas() {
        let rate = 48_000.0;
        let levels = settled(&sine(1000.0, rate, WINDOW, 0.4), rate);
        let band = loudest(&levels);
        for (i, v) in levels.iter().enumerate() {
            if i.abs_diff(band) >= 3 {
                assert!(*v < 0.35, "la barra {i} se enciende con un tono de la {band}: {levels:?}");
            }
        }
    }

    #[test]
    fn el_volumen_no_cambia_lo_vivo_que_se_ve() {
        // La misma señal a volumen normal y a volumen muy bajo: antes, con el bajo las barras casi
        // no se movían. Con la ganancia automática se ven igual.
        let rate = 48_000.0;
        let fuerte = settled(&sine(1000.0, rate, WINDOW, 0.5), rate);
        let flojo = settled(&sine(1000.0, rate, WINDOW, 0.01), rate);
        let band = loudest(&fuerte);
        assert!(fuerte[band] > 0.9 && flojo[band] > 0.9, "{fuerte:?} / {flojo:?}");
    }

    #[test]
    fn el_silencio_no_se_amplifica() {
        // Nada suena: ni ruido muy bajo levanta las barras, porque la referencia no baja del suelo.
        let rate = 48_000.0;
        assert_eq!(settled(&vec![0.0; WINDOW], rate), [0.0; BAND_COUNT]);
        // Ruido de fondo de una salida real: un par de bits de dither, unos -85 dB.
        let ruido: Vec<f32> = (0..WINDOW).map(|i| 0.00001 * ((i * 7919 % 13) as f32 - 6.0)).collect();
        let levels = settled(&ruido, rate);
        assert!(levels.iter().all(|v| *v < 0.05), "{levels:?}");
    }

    #[test]
    fn las_barras_bajan_despacio_al_parar_el_sonido() {
        let rate = 48_000.0;
        let mut analyzer = Analyzer::new();
        for _ in 0..4 {
            analyzer.process(&sine(1000.0, rate, WINDOW, 0.5), rate);
        }
        let band = loudest(&analyzer.levels());
        let alto = analyzer.levels()[band];
        analyzer.process(&vec![0.0; WINDOW], rate);
        let despues = analyzer.levels()[band];
        // Un bloque de silencio la baja, pero no de golpe a cero (es lo que evita el parpadeo).
        assert!(despues < alto && despues > alto * 0.5, "{alto} → {despues}");
    }

    /// Escucha la salida real unos segundos: `cargo test --lib audio -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn escucha_el_sistema() {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let device = capture::Loopback::open().expect("sin dispositivo de salida");
        let mut samples = Vec::new();
        let mut analyzer = Analyzer::new();
        let (mut windows, mut emissions) = (0u32, 0u32);
        let start = std::time::Instant::now();
        while start.elapsed().as_secs_f32() < 4.0 {
            std::thread::sleep(std::time::Duration::from_millis(16));
            device.read(&mut samples).expect("fallo de captura");
            while samples.len() >= WINDOW {
                let block: Vec<f32> = samples[..WINDOW].to_vec();
                samples.drain(..HOP);
                analyzer.process(&block, device.rate);
                windows += 1;
            }
            emissions += 1;
        }
        println!("tasa={} análisis/s={:.0} avisos a la isla/s={:.0} niveles={:?}", device.rate, windows as f32 / 4.0, emissions as f32 / 4.0, analyzer.levels());
    }
}
