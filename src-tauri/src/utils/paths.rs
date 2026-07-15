//! Centralized path utilities for the application.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};

/// Get the root data directory for the application (~/.astrbot_launcher).
#[allow(clippy::expect_used)]
pub(crate) fn get_data_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".astrbot_launcher")
}

/// Get the path to the unified application data database.
pub(crate) fn data_db_path() -> PathBuf {
    get_data_dir().join("data.redb")
}

/// Get the path to the legacy config TOML file (migration only).
pub(crate) fn config_path() -> PathBuf {
    get_data_dir().join("config.toml")
}

/// Get the path to the legacy manifest TOML file (migration only).
pub(crate) fn manifest_path() -> PathBuf {
    get_data_dir().join("manifest.toml")
}

/// Get the path to the releases cache file.
pub(crate) fn version_list_cache_path() -> PathBuf {
    get_data_dir().join("version_list.json")
}

/// Ensure all required data directories exist.
pub(crate) fn ensure_data_dirs() -> Result<()> {
    let base = get_data_dir();
    fs::create_dir_all(&base).map_err(|e| AppError::io(e.to_string()))?;

    let dirs = [
        base.join("components"),
        base.join("versions"),
        base.join("instances"),
        base.join("backups"),
    ];
    for dir in &dirs {
        fs::create_dir_all(dir).map_err(|e| AppError::io(e.to_string()))?;
    }
    Ok(())
}

/// Get the root directory for an instance.
pub(crate) fn get_instance_dir(instance_id: &str) -> PathBuf {
    get_data_dir().join("instances").join(instance_id)
}

/// Get the core directory for an instance.
pub(crate) fn get_instance_core_dir(instance_id: &str) -> PathBuf {
    get_instance_dir(instance_id).join("core")
}

/// Get the virtual environment directory for an instance.
pub(crate) fn get_instance_venv_dir(instance_id: &str) -> PathBuf {
    get_instance_dir(instance_id).join("venv")
}

/// Get the versions directory.
pub(crate) fn get_versions_dir() -> PathBuf {
    get_data_dir().join("versions")
}

/// Get the zip file path for a specific version (e.g., versions/v4.14.8.zip).
pub(crate) fn get_version_zip_path(version: &str) -> PathBuf {
    get_versions_dir().join(format!("{}.zip", version))
}

/// Get the backups directory.
pub(crate) fn get_backups_dir() -> PathBuf {
    get_data_dir().join("backups")
}

/// Get the root components directory.
pub(crate) fn get_components_dir() -> PathBuf {
    get_data_dir().join("components")
}

/// Get a specific component's directory.
pub(crate) fn get_component_dir(dir_name: &str) -> PathBuf {
    get_components_dir().join(dir_name)
}

/// Get Python runtime directory under the unified python component.
pub(crate) fn get_python_runtime_dir(runtime: &str) -> PathBuf {
    get_component_dir("python").join(runtime)
}

fn join_segments(base: &Path, segments: &[&str]) -> PathBuf {
    let mut path = base.to_path_buf();
    path.extend(segments);
    path
}

#[cfg(target_os = "windows")]
fn platform_join(base: &Path, windows_segments: &[&str], _unix_segments: &[&str]) -> PathBuf {
    join_segments(base, windows_segments)
}

#[cfg(not(target_os = "windows"))]
fn platform_join(base: &Path, _windows_segments: &[&str], unix_segments: &[&str]) -> PathBuf {
    join_segments(base, unix_segments)
}

/// Get the path to the Python executable for a standalone Python directory.
pub(crate) fn get_python_exe_path(python_dir: &Path) -> PathBuf {
    platform_join(python_dir, &["python.exe"], &["bin", "python3"])
}

/// Get the path to the Node.js executable for a standalone Node directory.
pub(crate) fn get_node_exe_path(node_dir: &Path) -> PathBuf {
    platform_join(node_dir, &["node.exe"], &["bin", "node"])
}

/// Get the path to the npm executable for a standalone Node directory.
pub(crate) fn get_npm_exe_path(node_dir: &Path) -> PathBuf {
    platform_join(node_dir, &["npm.cmd"], &["bin", "npm"])
}

/// Get the path to the npx executable for a standalone Node directory.
pub(crate) fn get_npx_exe_path(node_dir: &Path) -> PathBuf {
    platform_join(node_dir, &["npx.cmd"], &["bin", "npx"])
}

/// Get the bin directory for a standalone Node directory.
pub(crate) fn get_node_bin_dir(node_dir: &Path) -> PathBuf {
    platform_join(node_dir, &[], &["bin"])
}

/// Get the npm global install prefix directory (component-level, shared by all instances).
pub(crate) fn get_nodejs_npm_prefix() -> PathBuf {
    get_component_dir("nodejs")
}

/// Get the npm cache directory (component-level, shared by all instances).
pub(crate) fn get_nodejs_npm_cache() -> PathBuf {
    get_component_dir("nodejs").join(".npm_cache")
}

/// Get the shim scripts directory for Node.js.
pub(crate) fn get_nodejs_shim_dir() -> PathBuf {
    get_component_dir("nodejs").join("shims")
}

/// Get the bin directory under an npm prefix (where globally installed binaries go).
pub(crate) fn get_npm_prefix_bin_dir(npm_prefix: &Path) -> PathBuf {
    platform_join(npm_prefix, &[], &["bin"])
}

/// Get the node_modules directory under an npm prefix.
pub(crate) fn get_npm_prefix_modules_dir(npm_prefix: &Path) -> PathBuf {
    platform_join(npm_prefix, &["node_modules"], &["lib", "node_modules"])
}

/// Get the Python executable path within a virtual environment.
pub(crate) fn get_venv_python(venv_dir: &Path) -> PathBuf {
    platform_join(venv_dir, &["Scripts", "python.exe"], &["bin", "python"])
}

/// Get uv executable path within uv component directory.
pub(crate) fn get_uv_exe_path(uv_dir: &Path) -> PathBuf {
    platform_join(uv_dir, &["uv.exe"], &["uv"])
}

/// Get uvx executable path within uv component directory.
pub(crate) fn get_uvx_exe_path(uv_dir: &Path) -> PathBuf {
    platform_join(uv_dir, &["uvx.exe"], &["uvx"])
}

/// Get uv cache directory (component-level, shared by all instances).
pub(crate) fn get_uv_cache_dir() -> PathBuf {
    get_component_dir("uv").join("cache")
}
