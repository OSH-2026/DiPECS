//! 系统状态采集器
//!
//! "how" — 如何获取设备层状态信息。
//!
//! 采集: 电池、网络、铃声模式、位置、耳机/蓝牙。
//! Linux 桌面环境下使用 fallback 值。

use aios_spec::{LocationType, NetworkType, RingerMode, SystemStateEvent};

/// 系统状态采集器
pub struct SystemStateCollector;

impl SystemStateCollector {
    /// 获取当前系统状态快照
    ///
    /// 在 Android (daemon) 上:
    /// - 电池: 读 /sys/class/power_supply/battery/capacity
    /// - 网络: 读 /sys/class/net/*/operstate
    /// - 位置: 通过 LocationManager (需要 Kotlin 端配合)
    ///
    /// 在 Linux 桌面上: 返回 fallback 值
    pub fn snapshot(timestamp_ms: i64) -> SystemStateEvent {
        SystemStateEvent {
            timestamp_ms,
            battery_pct: Self::read_battery(),
            is_charging: Self::read_charging(),
            network: Self::detect_network(),
            ringer_mode: RingerMode::Normal,
            location_type: LocationType::Unknown,
            headphone_connected: false,
            bluetooth_connected: false,
        }
    }

    /// 读取电量百分比
    fn read_battery() -> Option<u8> {
        // Android: /sys/class/power_supply/battery/capacity
        let paths = [
            "/sys/class/power_supply/battery/capacity",
            "/sys/class/power_supply/BAT0/capacity",
            "/sys/class/power_supply/BAT1/capacity",
        ];

        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(pct) = content.trim().parse::<u8>() {
                    return Some(pct);
                }
            }
        }
        None
    }

    /// 读取充电状态
    fn read_charging() -> bool {
        let paths = [
            "/sys/class/power_supply/battery/status",
            "/sys/class/power_supply/BAT0/status",
        ];

        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                return content.trim().to_lowercase().contains("charging");
            }
        }
        false
    }

    /// 检测网络类型
    fn detect_network() -> NetworkType {
        // 检查 /sys/class/net/ 下各接口的状态
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let operstate_path = entry.path().join("operstate");

                if let Ok(state) = std::fs::read_to_string(&operstate_path) {
                    let up = state.trim() == "up";

                    if up && (name_str.starts_with("wlan") || name_str == "wlan0") {
                        return NetworkType::Wifi;
                    }
                    if up && (name_str.starts_with("rmnet") || name_str.starts_with("wwan")) {
                        return NetworkType::Cellular;
                    }
                }
            }

            // 如果以太网接口是 up 的
            if std::fs::read_to_string("/sys/class/net/eth0/operstate")
                .map(|s| s.trim() == "up")
                .unwrap_or(false)
            {
                return NetworkType::Wifi; // 有线也归类为 WiFi (有网络)
            }
        }

        NetworkType::Unknown
    }
}
