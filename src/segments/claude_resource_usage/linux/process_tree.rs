use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::session_root::ResolvedRoot;
use crate::process_stat::{self, ProcessStat};

const PROC_ROOT: &str = "/proc";

#[derive(Debug, Eq, PartialEq)]
pub(super) struct TreeAggregate {
    pub(super) cpu_ticks: u64,
    pub(super) rss_pages: u64,
}

pub(super) fn collect(root: ResolvedRoot) -> Option<TreeAggregate> {
    let processes = read_process_tree(root)?;
    aggregate_tree(&processes, root)
}

fn read_process_tree(root: ResolvedRoot) -> Option<Vec<ProcessStat>> {
    traverse_process_tree(root, process_stat::read, read_process_children)
}

fn traverse_process_tree<ReadStat, ReadChildren>(
    root: ResolvedRoot,
    mut read_stat: ReadStat,
    mut read_children: ReadChildren,
) -> Option<Vec<ProcessStat>>
where
    ReadStat: FnMut(u32) -> Option<ProcessStat>,
    ReadChildren: FnMut(u32) -> Option<Vec<u32>>,
{
    let root_stat = read_stat(root.pid)?;
    if root_stat.pid != root.pid || root_stat.start_time != root.start_time {
        return None;
    }

    let mut processes = Vec::new();
    let mut stack = vec![root_stat];
    let mut visited = HashSet::new();

    while let Some(process) = stack.pop() {
        if !visited.insert(process.pid) {
            continue;
        }

        let mut validated_children = HashSet::new();
        for _ in 0..2 {
            let children = read_children(process.pid).unwrap_or_default();
            for child_pid in children {
                if visited.contains(&child_pid) || validated_children.contains(&child_pid) {
                    continue;
                }
                let Some(child) = read_stat(child_pid) else {
                    continue;
                };
                if child.pid == child_pid && child.ppid == process.pid {
                    validated_children.insert(child_pid);
                    stack.push(child);
                }
            }
        }

        processes.push(process);
    }

    let final_root = read_stat(root.pid)?;
    if final_root.pid != root.pid || final_root.start_time != root.start_time {
        return None;
    }

    Some(processes)
}

fn read_process_children(pid: u32) -> Option<Vec<u32>> {
    collect_task_children(pid, read_task_ids, read_task_children)
}

fn collect_task_children<ReadTasks, ReadChildren>(
    pid: u32,
    mut read_tasks: ReadTasks,
    mut read_children: ReadChildren,
) -> Option<Vec<u32>>
where
    ReadTasks: FnMut(u32) -> Option<Vec<u32>>,
    ReadChildren: FnMut(u32, u32) -> Option<Vec<u32>>,
{
    let mut children = HashSet::new();

    for tid in read_tasks(pid)? {
        if let Some(task_children) = read_children(pid, tid) {
            children.extend(task_children);
        }
    }

    let mut children: Vec<u32> = children.into_iter().collect();
    children.sort_unstable();
    Some(children)
}

fn read_task_ids(pid: u32) -> Option<Vec<u32>> {
    let path = Path::new(PROC_ROOT).join(pid.to_string()).join("task");
    let entries = fs::read_dir(path).ok()?;
    let mut tids = Vec::new();

    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if let Ok(tid) = name.parse() {
            tids.push(tid);
        }
    }

    Some(tids)
}

fn read_task_children(pid: u32, tid: u32) -> Option<Vec<u32>> {
    let path = Path::new(PROC_ROOT)
        .join(pid.to_string())
        .join("task")
        .join(tid.to_string())
        .join("children");
    let body = fs::read_to_string(path).ok()?;
    parse_process_children(&body)
}

fn parse_process_children(body: &str) -> Option<Vec<u32>> {
    body.split_whitespace()
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn aggregate_tree(processes: &[ProcessStat], root: ResolvedRoot) -> Option<TreeAggregate> {
    let by_pid: HashMap<u32, &ProcessStat> = processes
        .iter()
        .map(|process| (process.pid, process))
        .collect();
    let root_stat = by_pid.get(&root.pid)?;
    if root_stat.start_time != root.start_time {
        return None;
    }

    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for process in processes {
        if process.pid != root.pid {
            children.entry(process.ppid).or_default().push(process.pid);
        }
    }

    let mut aggregate = TreeAggregate {
        cpu_ticks: 0,
        rss_pages: 0,
    };
    let mut stack = vec![root.pid];
    let mut visited = HashSet::new();

    while let Some(pid) = stack.pop() {
        if !visited.insert(pid) {
            continue;
        }
        let Some(process) = by_pid.get(&pid) else {
            continue;
        };

        aggregate.cpu_ticks = aggregate.cpu_ticks.saturating_add(process.cpu_ticks);
        aggregate.rss_pages = aggregate.rss_pages.saturating_add(process.rss_pages);

        if let Some(process_children) = children.get(&pid) {
            stack.extend(process_children.iter().copied());
        }
    }

    Some(aggregate)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        aggregate_tree, collect_task_children, parse_process_children, traverse_process_tree,
        TreeAggregate,
    };
    use crate::process_stat::ProcessStat;
    use crate::segments::claude_resource_usage::linux::session_root::ResolvedRoot;

    fn process(
        pid: u32,
        ppid: u32,
        start_time: u64,
        cpu_ticks: u64,
        rss_pages: u64,
    ) -> ProcessStat {
        ProcessStat {
            pid,
            ppid,
            start_time,
            cpu_ticks,
            rss_pages,
        }
    }

    #[test]
    fn parses_direct_process_children() {
        assert_eq!(
            parse_process_children("11  12 13\n"),
            Some(vec![11, 12, 13])
        );
        assert_eq!(parse_process_children(""), Some(Vec::new()));
        assert_eq!(parse_process_children("11 invalid"), None);
    }

    #[test]
    fn collects_and_deduplicates_children_from_all_tasks() {
        let task_children = HashMap::from([
            ((10, 10), vec![11, 12]),
            ((10, 101), vec![12, 13]),
            ((10, 102), vec![14]),
        ]);

        assert_eq!(
            collect_task_children(
                10,
                |_| Some(vec![10, 101, 102]),
                |pid, tid| task_children.get(&(pid, tid)).cloned(),
            ),
            Some(vec![11, 12, 13, 14])
        );
    }

    #[test]
    fn traverses_only_valid_injected_descendants() {
        let root = ResolvedRoot {
            pid: 10,
            start_time: 100,
        };
        let stats = HashMap::from([
            (10, process(10, 1, 100, 10, 100)),
            (11, process(11, 10, 110, 20, 200)),
            (12, process(12, 99, 120, 30, 300)),
            (13, process(13, 11, 130, 40, 400)),
            (20, process(20, 1, 140, 50, 500)),
        ]);
        let children = HashMap::from([(10, vec![11, 12, 99]), (11, vec![13])]);
        let mut tree = traverse_process_tree(
            root,
            |pid| stats.get(&pid).cloned(),
            |pid| children.get(&pid).cloned(),
        )
        .unwrap();
        tree.sort_by_key(|entry| entry.pid);

        assert_eq!(
            tree,
            vec![
                process(10, 1, 100, 10, 100),
                process(11, 10, 110, 20, 200),
                process(13, 11, 130, 40, 400),
            ]
        );
    }

    #[test]
    fn rejects_root_reuse_during_tree_traversal() {
        let root = ResolvedRoot {
            pid: 10,
            start_time: 100,
        };
        let mut root_reads = 0;
        let tree = traverse_process_tree(
            root,
            |pid| {
                root_reads += 1;
                let start_time = if root_reads == 1 { 100 } else { 101 };
                Some(process(pid, 1, start_time, 10, 100))
            },
            |_| Some(Vec::new()),
        );

        assert_eq!(tree, None);
    }

    #[test]
    fn second_immediate_listing_adds_new_valid_child() {
        let root = ResolvedRoot {
            pid: 10,
            start_time: 100,
        };
        let stats = HashMap::from([
            (10, process(10, 1, 100, 10, 100)),
            (11, process(11, 10, 110, 20, 200)),
            (12, process(12, 10, 120, 30, 300)),
        ]);
        let mut root_listings = 0;
        let mut tree = traverse_process_tree(
            root,
            |pid| stats.get(&pid).cloned(),
            |pid| {
                if pid != 10 {
                    return Some(Vec::new());
                }
                root_listings += 1;
                if root_listings == 1 {
                    Some(vec![11])
                } else {
                    Some(vec![11, 12])
                }
            },
        )
        .unwrap();
        tree.sort_by_key(|entry| entry.pid);

        assert_eq!(
            tree,
            vec![
                process(10, 1, 100, 10, 100),
                process(11, 10, 110, 20, 200),
                process(12, 10, 120, 30, 300),
            ]
        );
    }

    #[test]
    fn aggregates_only_transitive_process_tree() {
        let processes = vec![
            process(10, 1, 100, 10, 100),
            process(11, 10, 110, 20, 200),
            process(12, 11, 120, 30, 300),
            process(13, 1, 130, 40, 400),
        ];

        assert_eq!(
            aggregate_tree(
                &processes,
                ResolvedRoot {
                    pid: 10,
                    start_time: 100,
                }
            ),
            Some(TreeAggregate {
                cpu_ticks: 60,
                rss_pages: 600,
            })
        );
        assert_eq!(
            aggregate_tree(
                &processes,
                ResolvedRoot {
                    pid: 10,
                    start_time: 101,
                }
            ),
            None
        );
    }

    #[test]
    fn keeps_cpu_ticks_when_live_child_is_reaped() {
        let root = ResolvedRoot {
            pid: 10,
            start_time: 100,
        };
        let before = vec![process(10, 1, 100, 100, 100), process(11, 10, 110, 50, 200)];
        let after = vec![process(10, 1, 100, 150, 100)];

        assert_eq!(
            aggregate_tree(&before, root).map(|aggregate| aggregate.cpu_ticks),
            Some(150)
        );
        assert_eq!(
            aggregate_tree(&after, root).map(|aggregate| aggregate.cpu_ticks),
            Some(150)
        );
    }
}
