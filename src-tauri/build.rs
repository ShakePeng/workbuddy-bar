fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=src/macos_tray.m");
        cc::Build::new()
            .file("src/macos_tray.m")
            .flag("-fobjc-arc")
            .compile("workbuddy_tray");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    tauri_build::build()
}
