#![cfg_attr(target_os="windows", windows_subsystem="windows")]
fn main() {
    if std::env::args().nth(1).as_deref()==Some("--box-worker") {
        std::process::exit(desktop_app_lib::run_worker());
    }
    desktop_app_lib::run();
}
