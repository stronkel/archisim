//! Windows: the program's icon (Explorer, taskbar, "Abrir con") and its
//! version information, built into the executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("ProductName", "ArchiSim")
            .set("FileDescription", "ArchiSim · herramienta BIM")
            .set("CompanyName", "AD5")
            .set("ProductVersion", &version)
            .set("FileVersion", &version)
            .set("OriginalFilename", "ArchiSim.exe");
        if let Err(e) = res.compile() {
            // Without the resource compiler the program still builds, with
            // no icon: say so in the build output.
            println!("cargo:warning=sin icono de Windows: {e}");
        }
    }
}
