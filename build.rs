//! Version and architecture strings the C build used to pass as -D defines
//! (`LITE_PROJECT_VERSION_STR`, `LITE_ARCH_TUPLE`).
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LITE_ARCH_TUPLE");
    // A new commit changes logs/HEAD, so the "(git-xxxxxxxx)" suffix stays current.
    println!("cargo:rerun-if-changed=.git/logs/HEAD");

    let mut version = std::env::var("CARGO_PKG_VERSION").unwrap();
    // Only non-release builds carry the commit, as upstream's meson.build did.
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        if let Ok(out) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
            let commit = String::from_utf8_lossy(&out.stdout);
            let commit = commit.trim();
            if out.status.success() && commit.len() >= 8 {
                version = format!("{version} (git-{})", &commit[..8]);
            }
        }
    }
    println!("cargo:rustc-env=LITE_PROJECT_VERSION_STR={version}");

    // '<arch>-<os>', the tuple upstream's Meson build produced (its cpu families equal Rust's
    // arch names for x86_64, x86, aarch64, arm).
    let arch = std::env::var("LITE_ARCH_TUPLE").unwrap_or_else(|_| {
        format!(
            "{}-{}",
            std::env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
            std::env::var("CARGO_CFG_TARGET_OS").unwrap()
        )
    });
    println!("cargo:rustc-env=LITE_ARCH_TUPLE={arch}");

    window_icon();
    // Cargo binaries need the same dlopen providers that Nix retains in postFixup.
    for variable in ["PKG_CONFIG_PATH", "PKG_CONFIG_LIBDIR", "PKG_CONFIG_SYSROOT_DIR"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    let mut providers = vec!["xkbcommon-x11", "x11", "xcb", "xcursor", "xi"];
    if std::env::var_os("CARGO_FEATURE_WAYLAND").is_some() {
        providers.push("wayland-client");
    }
    let libraries = Command::new("pkg-config")
        .arg("--variable=libdir")
        .args(&providers)
        .output().expect("pkg-config for native runtime providers");
    assert!(libraries.status.success(), "missing native runtime providers: {}", String::from_utf8_lossy(&libraries.stderr));
    let paths = String::from_utf8(libraries.stdout).expect("native library directories");
    for path in paths.split_whitespace() {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{path}");
    }
}

/// The window icon lives in `icon.inl` (a C byte array kept from upstream); extract its RGBA bytes.
fn window_icon() {
    let path = "resources/icons/icon.inl";
    println!("cargo:rerun-if-changed={path}");
    let text = std::fs::read_to_string(path).expect("read icon.inl");
    let body = text
        .split('{')
        .nth(1)
        .and_then(|s| s.split('}').next())
        .expect("icon.inl array");
    let bytes: Vec<u8> = body
        .split(',')
        .filter_map(|t| t.trim().strip_prefix("0x"))
        .map(|h| u8::from_str_radix(h, 16).expect("hex byte"))
        .collect();
    assert_eq!(bytes.len(), 64 * 64 * 4, "icon.inl must be 64x64 RGBA");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("icon.rgba");
    std::fs::write(out, bytes).unwrap();
}
