//! The system's file dialogs.
//!
//! On the desktop, the dialogs of `rfd`. On the web there is no folder:
//! opening goes through the iPad's file picker (`web::pick`), so these
//! answer «nothing chosen» and say why; saving answers a place under
//! `files::WEB_OUT`, and what is written there is handed over as a file
//! (`files::write`).

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

/// A folder to export several files to (in the browser, handed over one
/// after the other).
pub fn pick_export_folder(d: FileDialog) -> Option<std::path::PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    return d.pick_folder();
    #[cfg(target_arch = "wasm32")]
    return d.pick_export_folder();
}

#[cfg(target_arch = "wasm32")]
pub use web::FileDialog;

#[cfg(target_arch = "wasm32")]
mod web {
    use std::path::{Path, PathBuf};

    /// A file dialog that, in the browser, never opens: the same builder as
    /// `crate::file_dialog::FileDialog`, whose answers are always «cancelled».
    #[derive(Default, Clone, Debug)]
    pub struct FileDialog {
        title: String,
        name: String,
    }

    impl FileDialog {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add_filter(self, _name: impl Into<String>, _extensions: &[impl ToString]) -> Self {
            self
        }

        pub fn set_directory<P: AsRef<Path>>(self, _path: P) -> Self {
            self
        }

        pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
            self.name = name.into();
            self
        }

        pub fn set_title(mut self, title: impl Into<String>) -> Self {
            self.title = title.into();
            self
        }

        pub fn pick_file(self) -> Option<PathBuf> {
            self.not_yet()
        }

        pub fn pick_files(self) -> Option<Vec<PathBuf>> {
            self.not_yet()
        }

        pub fn pick_folder(self) -> Option<PathBuf> {
            self.not_yet()
        }

        /// The browser has no folders: a file to hand over, by the name
        /// given (`crate::files::write`).
        pub fn save_file(self) -> Option<PathBuf> {
            let name = if self.name.is_empty() {
                "archisim".to_string()
            } else {
                self.name.clone()
            };
            Some(Path::new(crate::files::WEB_OUT).join(name))
        }

        /// A folder to export several files to: they are handed over one
        /// after the other.
        pub fn pick_export_folder(self) -> Option<PathBuf> {
            Some(PathBuf::from(crate::files::WEB_OUT))
        }

        fn not_yet<T>(&self) -> Option<T> {
            crate::web::note(&format!(
                "«{}» todavía no está en la versión web (sí abrir, guardar e importar y exportar IFC).",
                self.title
            ));
            None
        }
    }
}
