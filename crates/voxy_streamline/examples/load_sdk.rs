use std::{path::PathBuf, process::ExitCode};
use voxy_streamline::StreamlineRuntime;
fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage: load_sdk <absolute path to signed sl.interposer.dll>");
        return ExitCode::FAILURE;
    };
    match StreamlineRuntime::load(&PathBuf::from(path)) {
        Ok(_runtime) => {
            println!("SDK DLL loaded; SDK initialization/rendering have not been performed");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SDK load failed: {error:?}");
            ExitCode::FAILURE
        }
    }
}
