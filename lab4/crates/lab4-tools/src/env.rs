//! Local system environment capture for Lab4 reports.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Host environment summary used in deployment reports.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SystemEnvironment {
    /// Hostname reported by the kernel.
    pub hostname: Option<String>,
    /// Operating system name from `/etc/os-release`.
    pub os_pretty_name: Option<String>,
    /// Kernel release from `/proc/sys/kernel/osrelease`.
    pub kernel_release: Option<String>,
    /// Rust target architecture, such as `x86_64` or `aarch64`.
    pub architecture: String,
    /// First CPU model name found in `/proc/cpuinfo`.
    pub cpu_model: Option<String>,
    /// Number of logical processors found in `/proc/cpuinfo`.
    pub cpu_logical_cores: Option<usize>,
    /// Total memory in KiB from `/proc/meminfo`.
    pub mem_total_kib: Option<u64>,
}

/// Captures a best-effort system environment snapshot.
///
/// Missing Linux procfs files are represented as [`None`] instead of hard
/// errors so that the tool remains usable in containers and CI.
#[must_use]
pub fn capture_environment() -> SystemEnvironment {
    let cpuinfo = read_optional(Path::new("/proc/cpuinfo"));
    let meminfo = read_optional(Path::new("/proc/meminfo"));
    let os_release = read_optional(Path::new("/etc/os-release"));

    SystemEnvironment {
        hostname: read_trimmed(Path::new("/proc/sys/kernel/hostname")),
        os_pretty_name: os_release.as_deref().and_then(parse_os_pretty_name),
        kernel_release: read_trimmed(Path::new("/proc/sys/kernel/osrelease")),
        architecture: std::env::consts::ARCH.to_owned(),
        cpu_model: cpuinfo.as_deref().and_then(parse_cpu_model),
        cpu_logical_cores: cpuinfo.as_deref().and_then(parse_logical_cores),
        mem_total_kib: meminfo.as_deref().and_then(parse_mem_total_kib),
    }
}

fn read_optional(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_trimmed(path: &Path) -> Option<String> {
    read_optional(path).map(|value| value.trim().to_owned())
}

fn parse_os_pretty_name(os_release: &str) -> Option<String> {
    for line in os_release.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(strip_quotes(value).to_owned());
        }
    }
    None
}

fn parse_cpu_model(cpuinfo: &str) -> Option<String> {
    for line in cpuinfo.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "model name" {
                return Some(value.trim().to_owned());
            }
        }
    }
    None
}

fn parse_logical_cores(cpuinfo: &str) -> Option<usize> {
    let count = cpuinfo
        .lines()
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(key, _)| key.trim() == "processor")
        })
        .count();
    (count > 0).then_some(count)
}

fn parse_mem_total_kib(meminfo: &str) -> Option<u64> {
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let number = rest.split_whitespace().next()?;
            return number.parse::<u64>().ok();
        }
    }
    None
}

fn strip_quotes(value: &str) -> &str {
    value.trim().trim_matches('"')
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_os_pretty_name__strips_quotes() {
        let input = "NAME=Example\nPRETTY_NAME=\"Example Linux 1.0\"\n";
        assert_eq!(
            parse_os_pretty_name(input),
            Some("Example Linux 1.0".to_owned())
        );
    }

    #[test]
    fn test_parse_cpu_model__reads_first_model_name() {
        let input = "processor\t: 0\nmodel name\t: Test CPU\nprocessor\t: 1\n";
        assert_eq!(parse_cpu_model(input), Some("Test CPU".to_owned()));
    }

    #[test]
    fn test_parse_logical_cores__counts_processors() {
        let input = "processor\t: 0\nmodel name\t: Test CPU\nprocessor\t: 1\n";
        assert_eq!(parse_logical_cores(input), Some(2));
    }

    #[test]
    fn test_parse_mem_total_kib__reads_number() {
        let input = "MemTotal:       16384000 kB\nMemFree:         1000 kB\n";
        assert_eq!(parse_mem_total_kib(input), Some(16_384_000));
    }
}
