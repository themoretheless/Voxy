fn main() {
    println!("cargo:rerun-if-env-changed=VOXY_STREAMLINE_SDK");
    if std::env::var_os("CARGO_FEATURE_NATIVE").is_none()
        || std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
    {
        return;
    }
    let sdk = std::env::var_os("VOXY_STREAMLINE_SDK")
        .expect("native Windows Streamline requires VOXY_STREAMLINE_SDK");
    let native = std::path::Path::new("../../native/streamline");
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false);
    build.include(std::path::Path::new(&sdk).join("include"));
    for source in [
        "frame_generation",
        "reflex",
        "frame_token",
        "session",
        "dlss",
        "windows_module",
        "windows_runtime",
        "c_api",
    ] {
        let file = native.join(format!("{source}.cpp"));
        println!("cargo:rerun-if-changed={}", file.display());
        build.file(file);
    }
    println!("cargo:rerun-if-changed={}", native.display());
    build.compile("voxy_streamline_native");
}
