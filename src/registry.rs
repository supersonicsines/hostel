use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AppData;
use crate::service::LocalService;

pub fn apply_metadata(
    data: &mut AppData,
    services: &mut [LocalService],
    prune_stale: bool,
) -> bool {
    let mut changed = false;
    let live_metadata_keys = services
        .iter()
        .map(LocalService::metadata_key)
        .collect::<HashSet<_>>();
    let live_memo_keys = services
        .iter()
        .map(LocalService::memo_key)
        .collect::<HashSet<_>>();

    for service in services {
        let metadata_key = service.metadata_key();
        let legacy_key = service.memo_key();
        let mut metadata = data
            .metadata
            .get(&metadata_key)
            .cloned()
            .unwrap_or_default();

        if let Some(memo) = data.memos.remove(&legacy_key) {
            if metadata.memo.is_none() {
                metadata.memo = Some(memo);
            }
            changed = true;
        }

        if let Some(url_path) = data.url_overrides.remove(&legacy_key) {
            if metadata.url_path.is_none() {
                metadata.url_path = Some(url_path);
            }
            changed = true;
        }

        if metadata.is_empty() {
            data.metadata.remove(&metadata_key);
        } else {
            data.metadata.insert(metadata_key, metadata.clone());
        }

        service.metadata = metadata;
    }

    if prune_stale {
        let before = data.metadata.len();
        data.metadata
            .retain(|key, metadata| live_metadata_keys.contains(key) && !metadata.is_empty());
        changed |= data.metadata.len() != before;

        let before = data.memos.len();
        data.memos.retain(|key, _| live_memo_keys.contains(key));
        changed |= data.memos.len() != before;

        let before = data.url_overrides.len();
        data.url_overrides
            .retain(|key, _| live_memo_keys.contains(key));
        changed |= data.url_overrides.len() != before;
    }

    changed
}

pub fn clear_metadata(data: &mut AppData, service: &LocalService) -> bool {
    let changed = data.metadata.remove(&service.metadata_key()).is_some();
    let legacy_memo_removed = data.memos.remove(&service.memo_key()).is_some();
    let legacy_url_removed = data.url_overrides.remove(&service.memo_key()).is_some();
    changed || legacy_memo_removed || legacy_url_removed
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
