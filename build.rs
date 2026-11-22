//! Puts PrintCraft's icon and version details into Windows builds (x86_64
//! and arm64). Needs the Windows resource compiler (rc.exe with MSVC, or
//! windres with MinGW); without it the build still works, just without the
//! icon on the .exe.

fn main() {
    println!("cargo:rerun-if-changed=packaging/icons/printcraft.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("packaging/icons/printcraft.ico")
        .set("ProductName", "PrintCraft")
        .set("FileDescription", "PrintCraft")
        .set("CompanyName", "PrintCraft contributors")
        .set("LegalCopyright", "GPL-3.0-or-later");
    if let Err(e) = res.compile() {
        println!("cargo:warning=no icon in the Windows program (resource compiler not found: {e})");
    }
}
