mod cpu_sampler;
mod process_tree;
mod session_root;

use crate::process_stat;

use super::ResourceUsage;

pub(super) fn collect(session_id: &str) -> Option<ResourceUsage> {
    if session_id.is_empty() || session_id.len() > 1024 {
        return None;
    }

    let root = session_root::resolve(session_id)?;
    let aggregate = process_tree::collect(root)?;
    let page_size = process_stat::page_size()?;
    let memory_bytes = aggregate.rss_pages.saturating_mul(page_size);
    let cpu_percent = cpu_sampler::sample(session_id, root, aggregate.cpu_ticks);

    Some(ResourceUsage {
        cpu_percent,
        memory_bytes,
    })
}
