// Diagnostic inserted after MTLCompileOptions.setLanguageVersion in WGSL/MSL path.
if std::env::var_os("VOXY_DIAGNOSTIC_METAL_SAFE_MATH").is_some() {
    #[allow(deprecated)]
    options.setFastMathEnabled(false);
}
