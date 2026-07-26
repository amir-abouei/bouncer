use arc_swap::ArcSwap;
use notify::{RecursiveMode, Watcher};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct RawConfig {
    targets: HashMap<String, String>,
}

pub type SharedAllowlist = Arc<ArcSwap<HashMap<String, String>>>;

fn load_from_disk(path: &Path) -> anyhow::Result<HashMap<String, String>> {
    let contents = std::fs::read_to_string(path)?;
    let raw: RawConfig = toml::from_str(&contents)?;
    Ok(raw.targets)
}

pub fn load_and_watch(path: &str) -> anyhow::Result<SharedAllowlist> {
    let path_buf = std::path::PathBuf::from(path);
    let initial = load_from_disk(&path_buf)?;
    tracing::info!(targets = ?initial, "loaded allowlist");

    let shared: SharedAllowlist = Arc::new(ArcSwap::from_pointee(initial));
    let watched = shared.clone();
    let watch_path = path_buf.clone();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&watch_path, RecursiveMode::NonRecursive)?;

    std::thread::spawn(move || {
        let _watcher = watcher;
        for res in rx {
            match res {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                    ) {
                        match load_from_disk(&watch_path) {
                            Ok(new_targets) => {
                                tracing::info!(targets = ?new_targets, "allowlist reloaded");
                                watched.store(Arc::new(new_targets));
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "failed to reload allowlist, keeping previous config");
                            }
                        }
                    }
                }
                Err(e) => tracing::error!(error = %e, "watch error"),
            }
        }
    });

    Ok(shared)
}

pub fn resolve(allowlist: &SharedAllowlist, alias: &str) -> Option<String> {
    allowlist.load().get(alias).cloned()
}
