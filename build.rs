//! Embeds the TakeUI app icon and version details into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/brand/takeshot.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/brand/takeshot.ico")
            .set("ProductName", "Takeshot")
            .set("FileDescription", "Takeshot")
            .set("CompanyName", "Takeflow Sistemas")
            .set(
                "LegalCopyright",
                "Copyright (c) 2026-present Takeflow Sistemas",
            );
        res.compile().expect("embed Windows resources");
    }
}
