pub const HOST_HEADER_NAME: &str = "x-bouncer-host";

fn split_path(path: &str) -> Option<(&str, &str)> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.split_once('/') {
        Some((alias, rest)) => Some((alias, rest)),
        None => Some((trimmed, "")),
    }
}

pub fn extract_alias<'a>(path: &'a str, host_header: Option<&'a str>) -> Option<(&'a str, &'a str)> {
    if let Some(alias) = host_header.map(str::trim).filter(|s| !s.is_empty()) {
        return Some((alias, path.trim_start_matches('/')));
    }
    split_path(path)
}
