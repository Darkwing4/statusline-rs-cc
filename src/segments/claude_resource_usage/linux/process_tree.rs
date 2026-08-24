use std::collections::HashSet;
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
    let processes = traverse_process_tree(root, process_stat::read, read_process_children)?;

    Some(aggregate_tree(&processes))
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

    Some(
        fs::read_dir(path)
            .ok()?
            .flatten()
            .filter_map(|entry| entry.file_name().to_string_lossy().parse().ok())
            .collect(),
    )
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

fn aggregate_tree(processes: &[ProcessStat]) -> TreeAggregate {
    processes.iter().fold(
        TreeAggregate {
            cpu_ticks: 0,
            rss_pages: 0,
        },
        |aggregate, process| TreeAggregate {
            cpu_ticks: aggregate.cpu_ticks.saturating_add(process.cpu_ticks),
            rss_pages: aggregate.rss_pages.saturating_add(process.rss_pages),
        },
    )
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
    fn sums_cpu_ticks_and_rss_pages() {
        let processes = vec![
            process(10, 1, 100, 10, 100),
            process(11, 10, 110, 20, 200),
            process(12, 11, 120, 30, 300),
        ];

        assert_eq!(
            aggregate_tree(&processes),
            TreeAggregate {
                cpu_ticks: 60,
                rss_pages: 600,
            }
        );
        assert_eq!(
            aggregate_tree(&[]),
            TreeAggregate {
                cpu_ticks: 0,
                rss_pages: 0,
            }
        );
    }
}
