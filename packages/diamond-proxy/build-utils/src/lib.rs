use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Helper function for git fallback logic (extracted from original code)
fn get_workspace_dir_from_git() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("Failed to execute git command to find repository root");

    if !output.status.success() {
        // Consider making this less severe if git isn't expected, maybe return Result?
        // For now, keep the panic as it matches original behavior when git fails.
        panic!("Failed to determine git repository root using git rev-parse");
    }

    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("Git output is not valid UTF-8")
            .trim()
            .to_string(),
    )
    .canonicalize()
    .expect("Failed to canonicalize path")
    .to_string_lossy()
    .into_owned()
}

pub fn build(packages: Vec<&str>, relative_dir: &str) {
    // Determine workspace directory: prioritize env var, fallback to git
    let workspace_dir = match env::var("STELLAR_CONTRACTS_ROOT") {
        Ok(val) => {
            let path = PathBuf::from(val);
            if path.is_dir() {
                println!(
                    "Using STELLAR_CONTRACTS_ROOT environment variable: {}",
                    path.display()
                );
                path.to_string_lossy().into_owned() // Convert PathBuf to String
            } else {
                eprintln!("Warning: STELLAR_CONTRACTS_ROOT environment variable is set but '{}' is not a valid directory. Falling back to git.", path.display());
                get_workspace_dir_from_git()
            }
        }
        Err(_) => {
            println!("STELLAR_CONTRACTS_ROOT environment variable not set. Falling back to git rev-parse.");
            get_workspace_dir_from_git()
        }
    };

    // Use custom target directory to avoid locks
    let custom_target_dir = format!("{workspace_dir}/{relative_dir}/target");

    // Create custom target directory path
    std::fs::create_dir_all(format!(
        "{custom_target_dir}/wasm32-unknown-unknown/release"
    ))
    .expect("Failed to create custom target directory");

    // Build all required packages
    for package in packages {
        let wasm_path = format!(
            "{}/wasm32-unknown-unknown/release/{}.wasm",
            custom_target_dir,
            package.replace("-", "_")
        );

        // Build the package
        println!("Building {package} for wasm32-unknown-unknown target...");
        let build_status = Command::new("rustup")
            .args([
                "run",
                "stable",
                "cargo",
                "build",
                "--release",
                "--package",
                package,
                "--target",
                "wasm32-unknown-unknown",
                "--target-dir",
                &custom_target_dir,
            ])
            .env("RUSTFLAGS", "-C target-feature=+bulk-memory")
            .status()
            .unwrap_or_else(|_| panic!("Failed to build {package}"));

        if !build_status.success() {
            panic!("Failed to build {package}");
        }

        // Only try to optimize if the file exists
        if Path::new(&wasm_path).exists() {
            println!("Optimizing {package} WASM binary at {wasm_path}...");
            let optimize_status = Command::new("stellar")
                .args([
                    "contract",
                    "optimize",
                    "--wasm",
                    &wasm_path,
                    "--wasm-out",
                    &wasm_path,
                ])
                .status()
                .unwrap_or_else(|_| panic!("Failed to optimize {package} WASM binary"));

            if !optimize_status.success() {
                panic!("Failed to optimize {package} WASM binary: {optimize_status}");
            }
            println!("Successfully optimized {package}");
        } else {
            // Try to find the file in deps directory
            let deps_wasm_path = format!(
                "{}/wasm32-unknown-unknown/release/deps/{}.wasm",
                custom_target_dir,
                package.replace("-", "_")
            );

            if Path::new(&deps_wasm_path).exists() {
                println!("Found {package} WASM binary in deps directory, copying...");
                fs::copy(&deps_wasm_path, &wasm_path)
                    .unwrap_or_else(|_| panic!("Failed to copy {package} WASM from deps"));

                println!("Optimizing {package} WASM binary...");
                let optimize_status = Command::new("stellar")
                    .args([
                        "contract",
                        "optimize",
                        "--wasm",
                        &wasm_path,
                        "--wasm-out",
                        &wasm_path,
                    ])
                    .status()
                    .unwrap_or_else(|_| panic!("Failed to optimize {package} WASM binary"));

                if !optimize_status.success() {
                    panic!("Failed to optimize {package} WASM binary");
                }
                println!("Successfully optimized {package}");
            } else {
                panic!("Could not find WASM file for {package} in either location");
            }
        }
    }

    println!("Successfully built and optimized all required contracts");
}
