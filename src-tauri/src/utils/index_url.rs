pub(crate) fn normalize_default_index(pypi_mirror: &str) -> String {
    if pypi_mirror.trim().is_empty() {
        return "https://pypi.org/simple".to_string();
    }

    let mirror = pypi_mirror.trim().trim_end_matches('/');
    if mirror.ends_with("/simple") {
        mirror.to_string()
    } else {
        format!("{}/simple", mirror)
    }
}

/// Wrap a URL with a proxy prefix.
/// If proxy is empty, returns the original URL unchanged.
pub(crate) fn wrap_with_proxy(proxy: &str, url: &str) -> String {
    if proxy.is_empty() {
        url.to_string()
    } else {
        let base = proxy.trim_end_matches('/');
        format!("{}/{}", base, url)
    }
}
