fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Ok(target_arch) = std::env::var("CARGO_CFG_TARGET_ARCH") {
        if target_arch == "bpfel" || target_arch == "bpfeb" {
            println!("cargo:rustc-cfg=getrandom_backend=\"custom\"");
        }
    }
}
