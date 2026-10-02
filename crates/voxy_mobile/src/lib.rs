//! Native mobile ABI boundary; the engine and shared application forbid unsafe.

#[cfg(any(target_os = "android", target_os = "ios"))]
fn run(event_loop: winit::event_loop::EventLoop<()>) {
    let result = voxy_app::SceneApp::new(false).and_then(|app| app.run(event_loop));
    if let Err(error) = result {
        eprintln!("Voxy mobile error: {error}");
    }
}

/// `NativeActivity` glue entrypoint. Owns the Android event-loop thread.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "Rust" fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    match winit::event_loop::EventLoop::builder()
        .with_android_app(app)
        .build()
    {
        Ok(event_loop) => run(event_loop),
        Err(error) => eprintln!("Voxy Android event-loop error: {error}"),
    }
}

/// Called exactly once by the iOS executable's main function, on the main thread.
#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn voxy_ios_main() {
    match winit::event_loop::EventLoop::new() {
        Ok(event_loop) => run(event_loop),
        Err(error) => eprintln!("Voxy iOS event-loop error: {error}"),
    }
}
