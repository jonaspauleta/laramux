use std::io::Write;
use std::process::Command;

use crate::error::{LaraMuxError, Result};

const REPO: &str = "jonaspauleta/laramux";

pub fn print_help() {
    println!("laramux {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("A TUI application for managing Laravel development processes.");
    println!();
    println!("USAGE:");
    println!("    laramux              Start the TUI in a Laravel project directory");
    println!("    laramux update       Update to the latest version");
    println!("    laramux --version    Print version");
    println!("    laramux --help       Print this help message");
}

pub async fn run_update() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {current_version}");
    println!("Checking for updates...");

    // Use redirect URL instead of API to avoid rate limits
    let output = Command::new("curl")
        .args([
            "-sIL",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            &format!("https://github.com/{REPO}/releases/latest"),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error: Failed to check for updates: {stderr}");
        std::process::exit(1);
    }

    let url = String::from_utf8_lossy(&output.stdout);
    let tag = url
        .rsplit('/')
        .next()
        .ok_or_else(|| LaraMuxError::Process("Could not parse release tag from URL".into()))?;

    let latest_version = tag.strip_prefix('v').unwrap_or(tag);

    if latest_version == current_version {
        println!("Already up to date (v{current_version}).");
        return Ok(());
    }

    println!("New version available: v{latest_version}");

    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        eprintln!("Error: Unsupported operating system");
        std::process::exit(1);
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        eprintln!("Error: Unsupported architecture");
        std::process::exit(1);
    };

    let asset = if os == "windows" {
        format!("laramux-{os}-{arch}.exe")
    } else {
        format!("laramux-{os}-{arch}")
    };

    let base_url = format!("https://github.com/{REPO}/releases/latest/download");

    let tmpdir = std::env::temp_dir().join("laramux-update");
    std::fs::create_dir_all(&tmpdir)?;
    let tmp_binary = tmpdir.join("laramux");
    let tmp_checksums = tmpdir.join("checksums.txt");

    println!("Downloading {asset}...");
    let status = Command::new("curl")
        .args(["-fSL", &format!("{base_url}/{asset}"), "-o"])
        .arg(&tmp_binary)
        .status()?;
    if !status.success() {
        eprintln!("Error: Failed to download binary");
        let _ = std::fs::remove_dir_all(&tmpdir);
        std::process::exit(1);
    }

    let status = Command::new("curl")
        .args(["-fSL", &format!("{base_url}/checksums.txt"), "-o"])
        .arg(&tmp_checksums)
        .status()?;
    if !status.success() {
        eprintln!("Error: Failed to download checksums");
        let _ = std::fs::remove_dir_all(&tmpdir);
        std::process::exit(1);
    }

    println!("Verifying checksum...");
    let checksums = std::fs::read_to_string(&tmp_checksums)?;
    let expected = checksums
        .lines()
        .find(|line| line.ends_with(&asset))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| LaraMuxError::Process(format!("Checksum not found for {asset}")))?;

    let binary_bytes = std::fs::read(&tmp_binary)?;
    let actual = sha256_hex(&binary_bytes);

    if expected != actual {
        eprintln!("Error: Checksum verification failed");
        eprintln!("  Expected: {expected}");
        eprintln!("  Actual:   {actual}");
        let _ = std::fs::remove_dir_all(&tmpdir);
        std::process::exit(1);
    }

    let current_exe = std::env::current_exe()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_binary, std::fs::Permissions::from_mode(0o755))?;
    }

    match std::fs::rename(&tmp_binary, &current_exe) {
        Ok(()) => {}
        Err(e) if e.raw_os_error() == Some(18) => {
            // EXDEV: cross-device link
            // Cross-device move: copy instead
            let bytes = std::fs::read(&tmp_binary)?;
            let mut f = std::fs::File::create(&current_exe)?;
            f.write_all(&bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                f.set_permissions(std::fs::Permissions::from_mode(0o755))?;
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!("Error: Permission denied. Try: sudo laramux update");
            } else {
                eprintln!("Error: Failed to replace binary: {e}");
            }
            let _ = std::fs::remove_dir_all(&tmpdir);
            std::process::exit(1);
        }
    }

    let _ = std::fs::remove_dir_all(&tmpdir);
    println!("Successfully updated to v{latest_version}!");
    Ok(())
}

fn sha256_hex(data: &[u8]) -> String {
    // Simple SHA-256 implementation using std Command to avoid adding a dependency
    use std::io::Write;
    let mut child = Command::new("sh")
        .args(["-c", "shasum -a 256 || sha256sum"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to run shasum/sha256sum");
    child.stdin.take().unwrap().write_all(data).unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}
