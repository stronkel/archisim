//! Writing what is exported (`docs/IPAD.md` §12).
//!
//! On the desktop, a file where the dialog said. In the browser there are
//! no folders: the dialogs answer a place under [`WEB_OUT`], and what is
//! written there goes to the person as a file to keep (the iPad's «Guardar
//! en Archivos» or a download, `web::deliver`), one after the other if
//! there are several.

use std::path::Path;

/// Where the browser's save dialogs «put» the files: not a folder, a mark.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const WEB_OUT: &str = "/Descargas";

/// Writes `bytes` in `path`, or, in the browser, hands them over.
pub fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    if let Ok(rest) = path.as_ref().strip_prefix(WEB_OUT) {
        let name = rest.to_string_lossy().replace('/', "-");
        crate::web::deliver(&name, bytes.as_ref(), mime_of(&name));
        return Ok(());
    }
    std::fs::write(path, bytes)
}

/// Makes a folder for what is exported (nothing to make in the browser).
pub fn create_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    if path.as_ref().starts_with(WEB_OUT) {
        return Ok(());
    }
    std::fs::create_dir_all(path)
}

/// The kind of a file by its name, for the browser.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn mime_of(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "dxf" => "image/vnd.dxf",
        "csv" => "text/csv",
        "txt" | "bc3" => "text/plain",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "glb" => "model/gltf-binary",
        "ifc" => "application/x-step",
        "json" | "archisim" => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_on_the_desktop_as_always() {
        let dir = std::env::temp_dir().join(format!("archisim-files-{}", std::process::id()));
        create_dir_all(&dir).unwrap();
        write(dir.join("a.txt"), "hola").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "hola");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(mime_of("Planos.PDF"), "application/pdf");
    }
}
