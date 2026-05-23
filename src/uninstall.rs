use crate::db::transcribe_home;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const INSTALL_DIR_NAME: &str = "episode-transcribe";

pub fn run_uninstall(purge: bool, yes: bool) -> Result<()> {
    if !yes {
        eprintln!("This will remove the transcribe CLI from your user PATH and delete");
        eprintln!("the install directory.");
        if purge {
            eprintln!("--purge will also delete ~/.transcribe (projects, registry).");
        }
        eprintln!("Re-run with -y to confirm.");
        bail!("aborted (use -y to confirm)");
    }

    let mut removed_any = false;

    #[cfg(windows)]
    {
        if let Some(install_dir) = windows_install_dir() {
            if install_dir.is_dir() {
                fs::remove_dir_all(&install_dir)
                    .with_context(|| format!("remove {}", install_dir.display()))?;
                eprintln!("removed: {}", install_dir.display());
                removed_any = true;
            }
            remove_from_user_path(&install_dir)?;
        }
    }

    #[cfg(not(windows))]
    {
        let local_bin = unix_local_bin();
        let binary = local_bin.join("transcribe");
        if binary.is_file() {
            fs::remove_file(&binary).with_context(|| format!("remove {}", binary.display()))?;
            eprintln!("removed: {}", binary.display());
            removed_any = true;
        }
        if let Ok(which) = which_transcribe() {
            if which != binary && which.is_file() {
                eprintln!("note: another transcribe exists at {}", which.display());
                eprintln!("      (e.g. cargo install); remove manually if needed");
            }
        }
    }

    if purge {
        let home = transcribe_home().context("resolve transcribe home")?;
        if home.is_dir() {
            fs::remove_dir_all(&home).with_context(|| format!("remove {}", home.display()))?;
            eprintln!("removed registry: {}", home.display());
            removed_any = true;
        }
    }

    if !removed_any {
        eprintln!("nothing found to uninstall (already removed?)");
    } else {
        eprintln!("uninstall complete. Open a new terminal for PATH changes.");
    }

    Ok(())
}

#[cfg(windows)]
fn windows_install_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("Programs").join(INSTALL_DIR_NAME))
}

#[cfg(windows)]
fn remove_from_user_path(install_dir: &Path) -> Result<()> {
    let install_norm = install_dir.to_string_lossy().replace('/', "\\");
    let script = format!(
        r#"
$install = '{}'
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not $userPath) {{ exit 0 }}
$parts = $userPath -split ';' | Where-Object {{
    $_ -and ($_ -ne $install) -and ($_ -notlike '*episode-transcribe*')
}}
$new = ($parts | Where-Object {{ $_ }}) -join ';'
if ($new -ne $userPath) {{
    [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    Write-Output 'updated'
}}
"#,
        install_norm.replace('\'', "''")
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .context("update user PATH")?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("updated") {
            eprintln!("removed install dir from user PATH");
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn unix_local_bin() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".local").join("bin"))
        .unwrap_or_else(|| PathBuf::from(".local/bin"))
}

#[cfg(not(windows))]
fn which_transcribe() -> Result<PathBuf> {
    let output = std::process::Command::new("which")
        .arg("transcribe")
        .output()
        .context("which transcribe")?;
    if !output.status.success() {
        bail!("transcribe not on PATH");
    }
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn uninstall_without_yes_aborts() {
        assert!(run_uninstall(false, false).is_err());
    }

    #[test]
    fn uninstall_purge_temp_home() {
        crate::test_util::with_home(|home| {
            std::fs::create_dir_all(home).unwrap();
            run_uninstall(true, true).unwrap();
        });
    }
}
