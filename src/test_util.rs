//! Shared helpers for tests (serialized env access).

#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};
#[cfg(test)]
use tempfile::TempDir;

#[cfg(test)]
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Run a test closure with an exclusive `TRANSCRIBE_HOME` temp directory.
#[cfg(test)]
pub fn with_home<F, R>(f: F) -> R
where
    F: FnOnce(&Path) -> R,
{
    let _guard = env_lock();
    let tmp = TempDir::new().expect("temp home");
    std::env::set_var("TRANSCRIBE_HOME", tmp.path());
    let result = f(tmp.path());
    std::env::remove_var("TRANSCRIBE_HOME");
    result
}

#[cfg(test)]
pub fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
