use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");

    // exists npm and vite command
    if !command_exists("npm") || !command_exists("vite") {
        println!("cargo:warning=npm or vite command not found");
        return;
    }

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("frontend")
        .status()
        .expect("Failed to execute Vite build");

    if !status.success() {
        panic!("Vite build failed");
    }
}

fn command_exists(cmd: &str) -> bool {
    if let Ok(paths) = env::var("PATH") {
        for path in env::split_paths(&paths) {
            let full_path = path.join(cmd);
            if full_path.is_file() && is_executable(&full_path) {
                return true;
            }
        }
    }
    false
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
