use std::cmp::Ordering;
use std::path::Path;
use std::process::Command;

use reqwest::Client;
use crate::runtime::{AppHandle, AppState};

use crate::backup;
use crate::component;
use crate::component::ComponentsSnapshot;
use crate::config::{
    load_config, load_manifest, reload_config, reload_manifest, with_config_mut, AppConfig,
    AppManifest, BackupInfo, InstalledVersion, ThemePreference,
};
use crate::download;
use crate::error::{AppError, Result};
use crate::github::{self, GitHubRelease};
use crate::instance::{self, InstanceStatus};
use crate::network_config;
use crate::platform;
use crate::process::ProcessManager;
use crate::utils::index_url::normalize_default_index;
use crate::utils::lock_check::{collect_files_for_lock_check, ensure_target_not_locked};
use crate::utils::net::{build_http_client_with_proxy, check_url};
use crate::utils::paths::get_instance_core_dir;
use crate::utils::proxy::{
    build_single_url_proxy_settings, ProxyFields, ProxySource, DEFAULT_NO_PROXY_VALUE,
};
use crate::utils::validation::validate_instance_id;

fn sort_installed_versions_semver(versions: &mut [InstalledVersion]) {
    versions.sort_by(|a, b| {
        let av = semver::Version::parse(a.version.trim_start_matches('v')).ok();
        let bv = semver::Version::parse(b.version.trim_start_matches('v')).ok();

        match (av, bv) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.version.cmp(&a.version),
        }
    });
}

macro_rules! define_save_config_command {
    ($fn_name:ident, $param:ident : $ty:ty, $field:ident) => {
                pub async fn $fn_name($param: $ty) -> Result<()> {
            with_config_mut(move |config| {
                config.$field = $param;
                Ok(())
            })
        }
    };
}

fn apply_uv_fallback(config: &mut AppConfig) {
    if config.use_uv_for_deps && !component::is_uv_installed() {
        config.use_uv_for_deps = false;
        if let Err(e) = with_config_mut(|cfg| {
            if cfg.use_uv_for_deps {
                cfg.use_uv_for_deps = false;
            }
            Ok(())
        }) {
            log::warn!("Failed to persist uv fallback to pip: {}", e);
        }
    }
}

pub(crate) fn build_app_snapshot_with(
    process_manager: &ProcessManager,
    load_config_fn: fn() -> Result<std::sync::Arc<AppConfig>>,
    load_manifest_fn: fn() -> Result<std::sync::Arc<AppManifest>>,
) -> Result<AppSnapshot> {
    let config = load_config_fn()?;
    let manifest = load_manifest_fn()?;
    let instances = instance::list_instances(process_manager, manifest.as_ref());
    let backups = backup::list_backups()?;
    let mut config_for_snapshot = (*config).clone();
    apply_uv_fallback(&mut config_for_snapshot);
    let mut versions = manifest.installed_versions.clone();
    sort_installed_versions_semver(&mut versions);

    Ok(AppSnapshot {
        instances,
        versions,
        backups,
        components: component::build_components_snapshot(),
        config: config_for_snapshot,
    })
}

pub async fn get_app_snapshot(state: &AppState) -> Result<AppSnapshot> {
    let pm = state.process_manager.clone();
    tokio::task::spawn_blocking(move || build_app_snapshot_with(&pm, load_config, load_manifest))
        .await
        .map_err(|e| AppError::process(format!("Snapshot task panicked: {}", e)))?
}

pub async fn rebuild_app_snapshot(state: &AppState) -> Result<AppSnapshot> {
    let pm = state.process_manager.clone();
    tokio::task::spawn_blocking(move || {
        build_app_snapshot_with(&pm, reload_config, reload_manifest)
    })
    .await
    .map_err(|e| AppError::process(format!("Snapshot task panicked: {}", e)))?
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppSnapshot {
    pub instances: Vec<InstanceStatus>,
    pub versions: Vec<InstalledVersion>,
    pub backups: Vec<BackupInfo>,
    pub components: ComponentsSnapshot,
    pub config: AppConfig,
}

// === Config ===

pub async fn save_github_proxy(github_proxy: String, state: &AppState) -> Result<()> {
    // Test connectivity first
    let url = github::build_api_url(&github_proxy);
    let client = state.client();
    check_url(&client, &url).await?;
    // Test passed, save
    with_config_mut(move |config| {
        config.github_proxy = github_proxy;
        Ok(())
    })
}

pub async fn save_proxy(
    proxy_url: String,
    proxy_port: String,
    proxy_username: String,
    proxy_password: String,
    state: &AppState,
) -> Result<()> {
    let proxy_fields = ProxyFields::new(proxy_url, proxy_port, proxy_username, proxy_password);

    // Use a raw client (no system-proxy fallback) purely for connectivity tests.
    let configured_proxy = build_single_url_proxy_settings(
        ProxySource::AppConfig,
        &proxy_fields,
        Some(DEFAULT_NO_PROXY_VALUE.to_string()),
    )?;
    let test_proxy = configured_proxy.clone();
    let test_client = build_http_client_with_proxy(test_proxy)?;

    if !proxy_fields.url.is_empty()
        && check_url(&test_client, "https://cloudflare.com/cdn-cgi/trace")
            .await
            .is_err()
        && check_url(&test_client, "https://baidu.com").await.is_err()
    {
        return Err(AppError::network(
            "代理配置错误，无法连接 cloudflare.com 或 baidu.com",
        ));
    }

    let ProxyFields {
        url,
        port,
        username,
        password,
    } = proxy_fields;

    let next_client = with_config_mut(move |config| {
        config.proxy_url = url;
        config.proxy_port = port;
        config.proxy_username = username;
        config.proxy_password = password;
        network_config::build_http_client_from_config(config)
    })?;

    state.replace_client(next_client);

    Ok(())
}

pub async fn save_pypi_mirror(pypi_mirror: String, state: &AppState) -> Result<()> {
    let normalized_default_index = normalize_default_index(&pypi_mirror);
    let check_url_value = format!("{}/", normalized_default_index);

    // Test connectivity first
    let client = state.client();
    check_url(&client, &check_url_value).await?;

    // Test passed, save
    let normalized_for_save = if pypi_mirror.trim().is_empty() {
        String::new()
    } else {
        normalized_default_index
    };
    with_config_mut(move |config| {
        config.pypi_mirror = normalized_for_save;
        Ok(())
    })
}

define_save_config_command!(save_close_to_tray, close_to_tray: bool, close_to_tray);
define_save_config_command!(
    save_autostart_minimize_to_tray,
    autostart_minimize_to_tray: bool,
    autostart_minimize_to_tray
);
define_save_config_command!(save_nodejs_mirror, nodejs_mirror: String, nodejs_mirror);
define_save_config_command!(save_npm_registry, npm_registry: String, npm_registry);

pub async fn save_mainland_acceleration(
    mainland_acceleration: bool,
    state: &AppState,
) -> Result<()> {
    let next_client = with_config_mut(move |config| {
        config.mainland_acceleration = mainland_acceleration;
        network_config::build_http_client_from_config(config)
    })?;

    state.replace_client(next_client);
    Ok(())
}

pub async fn save_use_uv_for_deps(use_uv_for_deps: bool) -> Result<()> {
    if use_uv_for_deps && !component::is_uv_installed() {
        return Err(AppError::other("uv 组件未安装，无法启用 uv 安装依赖"));
    }

    with_config_mut(move |config| {
        config.use_uv_for_deps = use_uv_for_deps;
        Ok(())
    })
}

pub fn compare_versions(a: String, b: String) -> i32 {
    match (
        semver::Version::parse(a.trim_start_matches('v')),
        semver::Version::parse(b.trim_start_matches('v')),
    ) {
        (Ok(va), Ok(vb)) => va.cmp(&vb) as i32,
        _ => 0,
    }
}

define_save_config_command!(
    save_check_instance_update,
    check_instance_update: bool,
    check_instance_update
);
define_save_config_command!(
    save_persist_instance_state,
    persist_instance_state: bool,
    persist_instance_state
);
define_save_config_command!(
    save_ignore_external_path,
    ignore_external_path: bool,
    ignore_external_path
);
define_save_config_command!(
    save_lock_check_extension_whitelist,
    lock_check_extension_whitelist: bool,
    lock_check_extension_whitelist
);
define_save_config_command!(
    save_theme_preference,
    theme_preference: ThemePreference,
    theme_preference
);

// === Components ===

enum ComponentCommandAction {
    Install,
    Reinstall,
    Uninstall,
}

async fn run_component_command(
    app_handle: &AppHandle,
    state: &AppState,
    component_id: &str,
    action: ComponentCommandAction,
) -> Result<String> {
    let client = state.client();
    let id = component::ComponentId::from_str_id(component_id)
        .ok_or_else(|| AppError::other(format!("Unknown component: {}", component_id)))?;

    match action {
        ComponentCommandAction::Install => {
            component::install_component(&client, id, Some(app_handle)).await
        }
        ComponentCommandAction::Reinstall => {
            component::reinstall_component(&client, id, Some(app_handle)).await
        }
        ComponentCommandAction::Uninstall => {
            tokio::task::spawn_blocking(move || component::uninstall_component(id))
                .await
                .map_err(|e| AppError::process(format!("Uninstall task panicked: {}", e)))?
        }
    }
}

pub async fn install_component(
    app_handle: AppHandle,
    state: &AppState,
    component_id: String,
) -> Result<String> {
    run_component_command(
        &app_handle,
        state,
        &component_id,
        ComponentCommandAction::Install,
    )
    .await
}

pub async fn reinstall_component(
    app_handle: AppHandle,
    state: &AppState,
    component_id: String,
) -> Result<String> {
    run_component_command(
        &app_handle,
        state,
        &component_id,
        ComponentCommandAction::Reinstall,
    )
    .await
}

pub async fn uninstall_component(
    app_handle: AppHandle,
    state: &AppState,
    component_id: String,
) -> Result<String> {
    run_component_command(
        &app_handle,
        state,
        &component_id,
        ComponentCommandAction::Uninstall,
    )
    .await
}

// === GitHub ===

pub async fn fetch_releases(
    state: &AppState,
    force_refresh: Option<bool>,
) -> Result<Vec<GitHubRelease>> {
    let client = state.client();
    github::fetch_releases(&client, force_refresh.unwrap_or(false)).await
}

pub async fn fetch_launcher_release_notes(
    state: &AppState,
    version: String,
) -> Result<Option<String>> {
    let client = state.client();
    github::fetch_launcher_release_notes(&client, &version).await
}

// === Version Management ===

pub async fn install_version(
    app_handle: AppHandle,
    state: &AppState,
    release: GitHubRelease,
) -> Result<()> {
    let client = state.client();
    download::download_version(&client, &release, Some(&app_handle)).await
}

pub async fn uninstall_version(version: String) -> Result<()> {
    download::remove_version(&version)
}

// === Troubleshooting ===

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockCheckTarget {
    InstanceData,
    BackupCreate,
    BackupRestore,
    InstanceUpgrade,
}

fn run_instance_data_lock_check(instance_id: &str) -> Result<()> {
    validate_instance_id(instance_id)?;
    let data_dir = get_instance_core_dir(instance_id).join("data");
    if !data_dir.exists() {
        return Ok(());
    }
    let target_files = collect_files_for_lock_check(&data_dir)?;
    ensure_target_not_locked(&target_files)
}

pub async fn check_lock(
    target: LockCheckTarget,
    instance_id: Option<String>,
    backup_path: Option<String>,
    state: &AppState,
) -> Result<()> {
    match target {
        LockCheckTarget::InstanceData
        | LockCheckTarget::BackupCreate
        | LockCheckTarget::InstanceUpgrade => {
            let instance_id = instance_id
                .as_deref()
                .ok_or_else(|| AppError::other("check_lock 缺少 instance_id"))?;
            let _guard = state.process_manager.acquire_guard(instance_id)?;
            run_instance_data_lock_check(instance_id)
        }
        LockCheckTarget::BackupRestore => {
            let backup_path = backup_path
                .as_deref()
                .ok_or_else(|| AppError::other("check_lock 缺少 backup_path"))?;
            let (_, metadata) = backup::resolve_restore_backup_input(backup_path)?;
            let _guard = state.process_manager.acquire_guard(&metadata.instance_id)?;
            run_instance_data_lock_check(&metadata.instance_id)
        }
    }
}

pub async fn clear_instance_data(instance_id: String, state: &AppState) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::clear_instance_data(&instance_id)
}

pub async fn clear_instance_venv(instance_id: String, state: &AppState) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::clear_instance_venv(&instance_id)
}

pub async fn clear_pycache(instance_id: String, state: &AppState) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::clear_pycache(&instance_id)
}

pub async fn repair_instance(
    app_handle: AppHandle,
    instance_id: String,
    preserve_scope: instance::RepairPreserveScope,
    state: &AppState,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::repair_instance(&instance_id, preserve_scope, &app_handle).await
}

pub async fn rebuild_instance_manifest(
    state: &AppState,
) -> Result<instance::RebuildInstanceManifestResult> {
    if !state.process_manager.get_active_ids().is_empty() {
        return Err(AppError::instance_running());
    }

    let result = tokio::task::spawn_blocking(instance::rebuild_instance_manifest_from_disk)
        .await
        .map_err(|e| {
            AppError::process(format!("Rebuild instance manifest task panicked: {}", e))
        })??;
    Ok(result)
}

// === Instance Management ===

pub async fn open_instance_core_folder(instance_id: String) -> Result<()> {
    validate_instance_id(&instance_id)?;
    // 桌面专属功能：headless 环境无文件管理器，前端该按钮已灰显占位。
    Ok(())
}

pub async fn create_instance(name: String, version: String, port: u16) -> Result<()> {
    instance::create_instance(&name, &version, port)
}

pub async fn delete_instance(instance_id: String, state: &AppState) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::delete_instance(&instance_id)
}

pub async fn update_instance(
    app_handle: AppHandle,
    instance_id: String,
    name: Option<String>,
    version: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    state: &AppState,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::update_instance(
        &instance_id,
        name.as_deref(),
        version.as_deref(),
        host.as_deref(),
        port,
        &app_handle,
    )
    .await
}

pub async fn start_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: &AppState,
) -> Result<u16> {
    state
        .process_manager
        .start_instance(&instance_id, app_handle)
        .await
}

pub async fn stop_instance(instance_id: String, state: &AppState) -> Result<()> {
    state.process_manager.stop_instance(&instance_id).await
}

pub async fn restart_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: &AppState,
) -> Result<u16> {
    state
        .process_manager
        .restart_instance(&instance_id, app_handle)
        .await
}

pub async fn get_instance_port(instance_id: String, state: &AppState) -> Result<u16> {
    state
        .process_manager
        .get_port(&instance_id)
        .ok_or_else(AppError::instance_not_running)
}

// === Backup ===

pub async fn create_backup(instance_id: String, state: &AppState) -> Result<String> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    backup::create_backup(&instance_id, false)
}

pub async fn restore_backup(backup_path: String, state: &AppState) -> Result<()> {
    let (resolved_path, metadata) = backup::resolve_restore_backup_input(&backup_path)?;
    let _guard = state.process_manager.acquire_guard(&metadata.instance_id)?;
    backup::restore_backup_with_input(resolved_path, metadata)
}

pub async fn delete_backup(backup_path: String) -> Result<()> {
    backup::delete_backup(&backup_path)
}

// === Web (HTTP mode) specific ===
// 注意：systemd 单元名与 Node.js Web Manager 的 astrbotmgr.service 区分，
// 避免两个 Manager 抢占同一 unit。

const SYSTEMD_UNIT: &str = "astrbot-launcher";

pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemdStatus {
    pub installed: bool,
    pub enabled: bool,
}

pub async fn get_systemd_status() -> Result<SystemdStatus> {
    let unit_path = Path::new("/etc/systemd/system")
        .join(format!("{SYSTEMD_UNIT}.service"));
    let installed = unit_path.exists();
    let output = Command::new("systemctl")
        .args(["is-enabled", SYSTEMD_UNIT])
        .output();
    let enabled = match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    };
    Ok(SystemdStatus { installed, enabled })
}

pub async fn set_systemd_enabled(enable: bool) -> Result<()> {
    let action = if enable { "enable" } else { "disable" };
    Command::new("systemctl")
        .args([action, SYSTEMD_UNIT])
        .status()
        .map_err(|e| AppError::other(format!("systemctl {action} 失败: {e}")))?;
    Ok(())
}

pub async fn restart_manager() -> Result<()> {
    Command::new("systemctl")
        .args(["restart", SYSTEMD_UNIT])
        .status()
        .map_err(|e| AppError::other(format!("restart 失败: {e}")))?;
    Ok(())
}

pub async fn stop_manager() -> Result<()> {
    Command::new("systemctl")
        .args(["stop", SYSTEMD_UNIT])
        .status()
        .map_err(|e| AppError::other(format!("stop 失败: {e}")))?;
    Ok(())
}
