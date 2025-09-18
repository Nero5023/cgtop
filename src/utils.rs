use std::fs;
use std::path::Path;

/// Recursively remove a directory and all its contents
/// Logs errors but doesn't fail if some operations can't be completed
pub fn remove_dir_recursive_safe<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let path = path.as_ref();

    log::info!("Attempting to remove directory: {}", path.display());

    if !path.exists() {
        let msg = format!("Directory does not exist: {}", path.display());
        log::warn!("{}", msg);
        return Err(msg);
    }

    if !path.is_dir() {
        let msg = format!("Path is not a directory: {}", path.display());
        log::warn!("{}", msg);
        return Err(msg);
    }

    // Try to remove the directory recursively
    match remove_dir_contents_recursive(path) {
        Ok(_) => {
            // Try to remove the directory itself
            match fs::remove_dir(path) {
                Ok(_) => {
                    log::info!("Successfully removed directory: {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    let msg = format!("Failed to remove directory {}: {}", path.display(), e);
                    log::error!("{}", msg);
                    Err(msg)
                }
            }
        }
        Err(e) => {
            let msg = format!(
                "Failed to remove directory contents {}: {}",
                path.display(),
                e
            );
            log::error!("{}", msg);
            Err(msg)
        }
    }
}

fn remove_dir_contents_recursive<P: AsRef<Path>>(dir: P) -> Result<(), std::io::Error> {
    let dir = dir.as_ref();

    // Read directory entries
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Failed to read directory {}: {}", dir.display(), e);
            return Err(e);
        }
    };

    // Process each entry
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Failed to read directory entry in {}: {}", dir.display(), e);
                continue; // Skip this entry but continue with others
            }
        };

        let path = entry.path();

        if path.is_dir() {
            // Recursively remove subdirectory
            if let Err(e) = remove_dir_contents_recursive(&path) {
                log::warn!(
                    "Failed to remove subdirectory contents {}: {}",
                    path.display(),
                    e
                );
                // Continue with other entries
            }

            // Try to remove the empty subdirectory
            if let Err(e) = fs::remove_dir(&path) {
                log::warn!("Failed to remove subdirectory {}: {}", path.display(), e);
                // Continue with other entries
            } else {
                log::debug!("Removed subdirectory: {}", path.display());
            }
        } else {
            // Remove file
            if let Err(e) = fs::remove_file(&path) {
                log::warn!("Failed to remove file {}: {}", path.display(), e);
                // Continue with other entries
            } else {
                log::debug!("Removed file: {}", path.display());
            }
        }
    }

    Ok(())
}

/// Check if a path is safe to remove (basic safety checks)
pub fn is_safe_to_remove<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    // Don't allow removing system critical directories
    let forbidden_paths = [
        "/", "/sys", "/proc", "/dev", "/boot", "/etc", "/bin", "/sbin", "/usr", "/var", "/home",
        "/root",
    ];

    for forbidden in &forbidden_paths {
        if path_str == *forbidden || path_str.starts_with(&format!("{}/", forbidden)) {
            if !path_str.starts_with("/sys/fs/cgroup") {
                return false;
            }
        }
    }

    // Only allow removal under /sys/fs/cgroup
    if !path_str.starts_with("/sys/fs/cgroup/") {
        return false;
    }

    // Don't allow removing the root cgroup directory itself
    if path_str == "/sys/fs/cgroup" {
        return false;
    }

    true
}
