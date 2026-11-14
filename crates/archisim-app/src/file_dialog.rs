//! The system's file dialogs.
//!
//! On the desktop, the dialogs of `rfd`. On the web there is no folder to
//! open or save in yet (T2 of `docs/IPAD.md` brings the iPad's file
//! picker and downloads): the same calls answer «nothing chosen», so the
//! window carries on as if the person had cancelled, and says why.

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

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

        pub fn set_file_name(self, _name: impl Into<String>) -> Self {
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

        pub fn save_file(self) -> Option<PathBuf> {
            self.not_yet()
        }

        fn not_yet<T>(&self) -> Option<T> {
            crate::web::note(&format!(
                "«{}»: abrir y guardar archivos llega a la versión web en T2.",
                self.title
            ));
            None
        }
    }
}
