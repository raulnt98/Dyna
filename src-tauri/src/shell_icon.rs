//! Icono de una aplicación clásica, tal como la enseña el menú Inicio.
//!
//! `AppDisplayInfo.GetLogo` solo da logo a las aplicaciones empaquetadas. Las
//! clásicas (Outlook de escritorio, FortiClient, antivirus…) no tienen paquete,
//! pero el Shell de Windows sí sabe qué icono lleva cada una en `shell:AppsFolder`,
//! que es la misma lista del menú Inicio y se direcciona por su AUMID. Aquí se le
//! pide esa imagen y se codifica como PNG.
//!
//! Hay dos formas de AUMID. Las que registra la propia app (`Microsoft.Office.OUTLOOK.EXE.15`)
//! se buscan directamente en `AppsFolder`. Las apps que no registran ninguno reciben
//! uno derivado de su ruta: `{ID de carpeta conocida}\Fortinet\FortiClient\FortiTray.exe`,
//! con barras invertidas. Si ese no es una entrada del menú Inicio (un controlador de
//! bandeja, por ejemplo), se resuelve la carpeta conocida a su ruta real y se le pide
//! el icono al propio `.exe`.
//!
//! Un tercer caso son los avisos de globo de un icono de la bandeja (FortiClient,
//! clientes VPN, utilidades antiguas): Windows les inventa un AUMID,
//! `NotifyIconGeneratedAumid_<número>`, que no es una app del menú Inicio ni lleva ruta.
//! El número es el nombre de una clave de `HKCU\Control Panel\NotifyIconSettings`, que
//! guarda el `ExecutablePath` del programa que puso el icono.
//!
//! No usa la red ni lee contenido del usuario: es el icono de un programa.

/// `true` si el AUMID solo lleva caracteres normales y ninguna ruta sube de nivel.
///
/// Se comprueba antes de pasarlo al Shell como parte de una ruta. La barra invertida
/// vale: es el separador de los AUMID derivados de una ruta. `..` no.
pub fn is_plausible(aumid: &str) -> bool {
    !aumid.is_empty()
        && !aumid.contains("..")
        && aumid.chars().all(|c| {
            c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | '!' | '{' | '}' | ' ' | '(' | ')' | '\\')
        })
}

#[cfg(windows)]
pub fn for_aumid(aumid: &str) -> Option<String> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    if !is_plausible(aumid) {
        return None;
    }
    // Si COM ya estaba iniciado en este hilo, con otro modelo o con el mismo, el
    // error se ignora a propósito.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

    // Aviso de globo de un icono de bandeja: se busca el programa en el registro.
    if let Some(id) = aumid.strip_prefix(NOTIFY_ICON_PREFIX) {
        return image_of(&notify_icon_exe(id)?);
    }

    // 1) La entrada del menú Inicio.
    if let Some(icon) = image_of(&format!("shell:AppsFolder\\{aumid}")) {
        return Some(icon);
    }
    // 2) AUMID derivado de una ruta: el icono del propio ejecutable.
    image_of(&exe_path(aumid)?)
}

/// Prefijo de los AUMID que Windows genera para los avisos de globo de un icono de bandeja.
pub const NOTIFY_ICON_PREFIX: &str = "NotifyIconGeneratedAumid_";

/// `true` si es el número de una clave de `NotifyIconSettings`: solo dígitos, para no
/// poder colar otra ruta del registro.
fn is_notify_icon_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 24 && id.chars().all(|c| c.is_ascii_digit())
}

/// Ruta del programa que puso un icono en la bandeja, según el registro.
///
/// `ExecutablePath` viene como `{ID de carpeta}\resto` o como una ruta completa.
#[cfg(windows)]
fn notify_icon_exe(id: &str) -> Option<String> {
    use windows::core::{w, HSTRING};
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};

    if !is_notify_icon_id(id) {
        return None;
    }
    let key = HSTRING::from(format!("Control Panel\\NotifyIconSettings\\{id}"));
    let mut bytes = 0u32;
    unsafe {
        // Primera llamada: cuántos bytes ocupa el valor.
        let status = RegGetValueW(HKEY_CURRENT_USER, &key, w!("ExecutablePath"), RRF_RT_REG_SZ, None, None, Some(&mut bytes));
        if status.is_err() || bytes < 2 {
            return None;
        }
        let mut buffer = vec![0u16; (bytes as usize + 1) / 2];
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            w!("ExecutablePath"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as *mut _),
            Some(&mut bytes),
        );
        if status.is_err() {
            return None;
        }
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let raw = String::from_utf16_lossy(&buffer[..end]);
        // Con ID de carpeta conocida se resuelve a la ruta real; si no, ya es una ruta.
        if raw.starts_with('{') {
            exe_path(&raw)
        } else {
            std::fs::metadata(&raw).ok()?.is_file().then_some(raw)
        }
    }
}

/// Ruta real del ejecutable de un AUMID del tipo `{ID de carpeta conocida}\resto`.
#[cfg(windows)]
fn exe_path(aumid: &str) -> Option<String> {
    use windows::core::GUID;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let (id, rest) = aumid.split_once('\\')?;
    let id = id.strip_prefix('{')?.strip_suffix('}')?;
    let guid = GUID::try_from(id).ok()?;

    let folder = unsafe {
        let raw = SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, None).ok()?;
        let folder = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0 as *const _));
        folder?
    };
    let path = format!("{folder}\\{rest}");
    // Solo se pide el icono de un archivo que existe de verdad.
    std::fs::metadata(&path).ok()?.is_file().then_some(path)
}

/// Icono del elemento del Shell con ese nombre, como data URI PNG.
#[cfg(windows)]
fn image_of(parsing_name: &str) -> Option<String> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    };

    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(parsing_name), None).ok()?;
        let bitmap = factory
            .GetImage(SIZE { cx: 64, cy: 64 }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .ok()?;

        let mut info = BITMAP::default();
        let got = GetObjectW(
            HGDIOBJ(bitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut info as *mut _ as *mut _),
        );
        if got == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            return None;
        }
        let (width, height) = (info.bmWidth as usize, info.bmHeight as usize);

        // Se pide en BGRA de arriba abajo (altura negativa).
        let mut header = BITMAPINFO::default();
        header.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut bgra = vec![0u8; width * height * 4];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            bitmap,
            0,
            height as u32,
            Some(bgra.as_mut_ptr() as *mut _),
            &mut header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        if lines == 0 {
            return None;
        }

        // BGRA premultiplicado -> RGBA recto, que es lo que espera un PNG.
        let mut rgba = Vec::with_capacity(bgra.len());
        for px in bgra.chunks_exact(4) {
            let (b, g, r, a) = (px[0], px[1], px[2], px[3]);
            let un = |c: u8| -> u8 {
                if a == 0 || a == 255 {
                    c
                } else {
                    ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8
                }
            };
            rgba.extend_from_slice(&[un(r), un(g), un(b), a]);
        }
        // Si todo el canal alfa es 0 el icono no trae transparencia (algunos
        // iconos antiguos): se trata como opaco en vez de dejarlo invisible.
        if rgba.chunks_exact(4).all(|px| px[3] == 0) {
            for px in rgba.chunks_exact_mut(4) {
                px[3] = 255;
            }
        }

        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&rgba).ok()?;
        }
        Some(format!("data:image/png;base64,{}", BASE64.encode(&png)))
    }
}

#[cfg(not(windows))]
pub fn for_aumid(_aumid: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::is_plausible;

    #[test]
    fn admite_los_aumid_normales_y_los_derivados_de_una_ruta() {
        assert!(is_plausible("Microsoft.Office.OUTLOOK.EXE.15"));
        assert!(is_plausible("5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App"));
        assert!(is_plausible("{7B8C21DE-510F-48E4-A985-5F324D226ED1}.Bitdefender Endpoint Security Tools"));
        assert!(is_plausible("{6D809377-6AF0-444B-8957-A3773F02200E}\\Fortinet\\FortiClient\\FortiTray.exe"));
    }

    #[test]
    fn el_id_de_un_icono_de_bandeja_son_solo_digitos() {
        use super::is_notify_icon_id;
        assert!(is_notify_icon_id("12700883534930935995"));
        assert!(!is_notify_icon_id(""));
        assert!(!is_notify_icon_id("1270..\\0883"));
        assert!(!is_notify_icon_id("12a"));
        assert!(!is_notify_icon_id("1234567890123456789012345"));
    }

    #[test]
    fn rechaza_lo_que_no_es_un_aumid() {
        assert!(!is_plausible(""));
        assert!(!is_plausible("{6D809377-6AF0-444B-8957-A3773F02200E}\\..\\..\\Windows\\cmd.exe"));
        assert!(!is_plausible("C:\\Windows\\cmd.exe"));
        assert!(!is_plausible("a\"b"));
        assert!(!is_plausible("a&b"));
    }
}
