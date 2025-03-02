use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("frontend")
        .status()
        .expect("Failed to execute Vite build");

    if !status.success() {
        panic!("Vite build failed");
    }
}
