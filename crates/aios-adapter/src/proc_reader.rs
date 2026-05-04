//! `/proc` 文件系统读取器
//!
//! "how" — 如何从 Linux /proc 获取进程级资源信息。
//!
//! 读取:
//! - `/proc/[pid]/stat` — 进程状态、线程数
//! - `/proc/[pid]/status` — VmRSS, VmSwap
//! - `/proc/[pid]/oom_score` — LMK 打分
//! - `/proc/[pid]/io` — 磁盘 I/O 累计
//! - `/proc/[pid]/cmdline` — 进程命令行 (用于推断包名)

use aios_spec::{ProcState, ProcStateEvent};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// `/proc` 读取器
pub struct ProcReader;

/// 一次 /proc 轮询的结果
#[derive(Debug, Clone)]
pub struct ProcSnapshot {
    pub pid: u32,
    pub uid: u32,
    pub package_name: Option<String>,
    pub vm_rss_kb: u64,
    pub vm_swap_kb: u64,
    pub threads: u32,
    pub oom_score: i32,
    pub io_read_mb: u64,
    pub io_write_mb: u64,
    pub state: ProcState,
}

impl ProcReader {
    /// 扫描所有进程, 返回当前快照列表
    ///
    /// 遍历 `/proc/[0-9]*/` 目录, 读取每个进程的状态。
    pub fn scan_all() -> Vec<ProcSnapshot> {
        let mut snapshots = Vec::new();
        let proc_dir = Path::new("/proc");

        let entries = match fs::read_dir(proc_dir) {
            Ok(e) => e,
            Err(_) => return snapshots,
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // 只处理数字目录 (PID)
            let pid: u32 = match name_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            if let Some(snap) = Self::read_process(pid) {
                snapshots.push(snap);
            }
        }

        snapshots
    }

    /// 读取单个进程的信息
    fn read_process(pid: u32) -> Option<ProcSnapshot> {
        let base = format!("/proc/{}", pid);

        // 读取 stat — 第3字段为状态, 第20字段为线程数
        let (state, threads) = Self::read_stat(&base)?;

        // 读取 VmRSS, VmSwap, Uid
        let (uid, vm_rss_kb, vm_swap_kb) = Self::read_status(&base)?;

        // 读取 OOM score
        let oom_score = Self::read_oom_score(&base).unwrap_or(0);

        // 读取 I/O
        let (io_read_mb, io_write_mb) = Self::read_io(&base).unwrap_or((0, 0));

        // 读取 cmdline (包名推断)
        let package_name = Self::read_cmdline(&base);

        Some(ProcSnapshot {
            pid,
            uid,
            package_name,
            vm_rss_kb,
            vm_swap_kb,
            threads,
            oom_score,
            io_read_mb,
            io_write_mb,
            state,
        })
    }

    fn read_stat(base: &str) -> Option<(ProcState, u32)> {
        let content = fs::read_to_string(format!("{}/stat", base)).ok()?;
        // /proc/[pid]/stat 格式: pid (comm) state ... threads ...
        // 第3字段是 state, 需要跳过 comm 中的括号
        let comm_end = content.rfind(')')?;
        let rest = &content[comm_end + 2..]; // skip ") "
        let mut parts = rest.split_whitespace();

        let state_char = parts.next()?;
        let state = match state_char {
            "R" => ProcState::Running,
            "S" | "D" | "I" => ProcState::Sleeping,
            "Z" | "X" => ProcState::Zombie,
            _ => ProcState::Unknown,
        };

        // 跳过接下来的 15 个字段到达 threads (第20字段, index 19 in parts)
        // 我们已经读了 state (fields[0]), 还需要跳过 fields[1..17]
        let threads_str = parts.nth(17)?;
        let threads: u32 = threads_str.parse().ok()?;

        Some((state, threads))
    }

    fn read_status(base: &str) -> Option<(u32, u64, u64)> {
        let content = fs::read_to_string(format!("{}/status", base)).ok()?;
        let mut uid = 0u32;
        let mut vm_rss = 0u64;
        let mut vm_swap = 0u64;

        for line in content.lines() {
            if line.starts_with("Uid:") {
                // Format: "Uid:\t1000\t1000\t1000\t1000"
                uid = line.split_whitespace().nth(1)?.parse().ok()?;
            } else if line.starts_with("VmRSS:") {
                // Format: "VmRSS:\t  123456 kB"
                let val: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
                vm_rss = val;
            } else if line.starts_with("VmSwap:") {
                let val: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
                vm_swap = val;
            }
        }

        Some((uid, vm_rss, vm_swap))
    }

    fn read_oom_score(base: &str) -> Option<i32> {
        let content = fs::read_to_string(format!("{}/oom_score", base)).ok()?;
        content.trim().parse().ok()
    }

    fn read_io(base: &str) -> Option<(u64, u64)> {
        let content = fs::read_to_string(format!("{}/io", base)).ok()?;
        let mut read_bytes = 0u64;
        let mut write_bytes = 0u64;

        for line in content.lines() {
            if line.starts_with("read_bytes:") {
                read_bytes = line.split_whitespace().nth(1)?.parse().ok()?;
            } else if line.starts_with("write_bytes:") {
                write_bytes = line.split_whitespace().nth(1)?.parse().ok()?;
            }
        }

        // 转换为 MB
        Some((read_bytes / (1024 * 1024), write_bytes / (1024 * 1024)))
    }

    fn read_cmdline(base: &str) -> Option<String> {
        let content = fs::read_to_string(format!("{}/cmdline", base)).ok()?;
        // cmdline 是 null-separated, 取第一个参数
        let first_arg = content.split('\0').next()?;
        if first_arg.is_empty() {
            return None;
        }

        // 尝试从命令行中提取 Android 包名
        // 格式通常为 "com.example.app" 或 "/system/bin/surfaceflinger"
        if first_arg.contains('.') && !first_arg.starts_with('/') {
            Some(first_arg.to_string())
        } else {
            // 对于 native 进程, 返回进程名
            let name = std::path::Path::new(first_arg)
                .file_name()
                .map(|n| n.to_string_lossy().to_string());
            name
        }
    }

    /// 将 ProcSnapshot 转换为 aios-spec 的 ProcStateEvent
    pub fn to_event(snapshot: &ProcSnapshot, timestamp_ms: i64) -> ProcStateEvent {
        ProcStateEvent {
            timestamp_ms,
            pid: snapshot.pid,
            uid: snapshot.uid,
            package_name: snapshot.package_name.clone(),
            vm_rss_kb: snapshot.vm_rss_kb,
            vm_swap_kb: snapshot.vm_swap_kb,
            threads: snapshot.threads,
            oom_score: snapshot.oom_score,
            io_read_mb: snapshot.io_read_mb,
            io_write_mb: snapshot.io_write_mb,
            state: snapshot.state.clone(),
        }
    }
}

/// 计算两次快照之间的增量变化
pub fn diff_snapshots(prev: &HashMap<u32, ProcSnapshot>, curr: &HashMap<u32, ProcSnapshot>) -> Vec<ProcSnapshot> {
    // 对于新出现的或者状态变化的进程, 返回当前快照
    curr.values()
        .filter(|c| {
            match prev.get(&c.pid) {
                Some(p) => {
                    p.vm_rss_kb != c.vm_rss_kb
                        || p.vm_swap_kb != c.vm_swap_kb
                        || p.oom_score != c.oom_score
                        || !matches!(p.state, ProcState::Unknown)
                }
                None => true, // 新进程
            }
        })
        .cloned()
        .collect()
}
