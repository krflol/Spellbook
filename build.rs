#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=assets/spellbook.png");
    // Convert the generated PNG to the Windows resource format; keep its alpha.
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("spellbook.ico");
    image::open("assets/spellbook.png")
        .expect("Load application icon")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .save_with_format(&out, image::ImageFormat::Ico)
        .expect("Encode Windows icon");
    tauri_winres::WindowsResource::new()
        .set_icon(out.to_str().unwrap())
        .set("ProductName", "Spellbook")
        .set("FileDescription", "Spellbook — World of Warcraft macro IDE")
        .set("LegalCopyright", "Spellbook contributors. MIT license.")
        .compile()
        .expect("Embed Windows icon and version information");
}
#[cfg(not(windows))]
fn main() {}
