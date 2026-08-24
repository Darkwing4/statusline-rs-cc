mod cpu_sampler;
mod process_tree;
mod session_root;

use std::fs::OpenOptions;
use std::io::Read;
use std::os::raw::c_int;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use crate::process_stat;

use super::ResourceUsage;

const O_NOFOLLOW: c_int = 0o400000;

pub(super) fn collect(session_id: &str) -> Option<ResourceUsage> {
    if session_id.len() > 1024 {
        return None;
    }

    let root = session_root::resolve(session_id)?;
    let aggregate = process_tree::collect(root)?;
    let page_size = process_stat::page_size()?;
    let memory_bytes = aggregate.rss_pages.saturating_mul(page_size);
    let cpu_percent = cpu_sampler::sample(root, aggregate.cpu_ticks);

    Some(ResourceUsage {
        cpu_percent,
        memory_bytes,
    })
}

fn read_regular_file(path: &Path, max_bytes: u64, owner: Option<u32>) -> Option<String> {
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(O_NOFOLLOW);
    let file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;

    if !metadata.file_type().is_file() {
        return None;
    }

    if let Some(owner) = owner {
        if metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
            return None;
        }
    }

    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;

    if bytes.len() as u64 > max_bytes {
        return None;
    }

    String::from_utf8(bytes).ok()
}
