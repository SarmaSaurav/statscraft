// Embed the icon and the version information in the executable, so
// the app has a proper icon in Explorer and on the taskbar, and the
// file's properties show the version rather than "unknown".
fn main() {
    println!("cargo:rerun-if-changed=packaging/assets/app.ico");
    println!("cargo:rerun-if-changed=packaging/assets/statscraft.manifest");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/assets/app.ico");
        res.set_manifest_file("packaging/assets/statscraft.manifest");
        res.set("FileDescription", "StatsCraft Statistics");
        res.set("ProductName", "StatsCraft");
        res.set("CompanyName", "The StatsCraft Garage");
        res.set("LegalCopyright", "MIT licensed");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the resources: {}", e);
        }
    }
}
