mod component_python;
mod config_manifest;

pub fn run_startup_migrations() {
    config_manifest::migrate_config_manifest_if_needed();
    component_python::migrate_legacy_python_dirs();

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    component_python::migrate_windows_arm_python_component_if_needed();
}
