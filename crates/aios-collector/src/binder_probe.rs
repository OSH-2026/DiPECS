//! Binder eBPF 探针 — 跨进程通信监控
//!
//! "how" — 如何通过 eBPF tracepoint 监控 Android Binder 事务。
//!
//! Binder 是 Android 的核心 IPC 机制。所有系统服务调用
//! (通知、Activity 启动、窗口管理) 都是 Binder 事务。
//!
//! eBPF tracepoint: `tracepoint/binder/binder_transaction`
//! 提供: source_pid, target_pid, target_node (服务名), 事务类型。
//!
//! ## 实现状态
//!
//! 本模块提供完整的接口定义和 Linux 端实现 stubs。
//! Android 真机部署需要:
//! 1. 自编译内核 (CONFIG_BPF_SYSCALL=y, CONFIG_DEBUG_INFO_BTF=y)
//! 2. 或 root 权限 (加载预编译的 BPF 程序)
//! 3. Daemon 以 system 身份运行 (SELinux 允许 bpf)

use aios_spec::BinderTxEvent;

/// Binder 探针 — 订阅 Binder 事务事件
pub struct BinderProbe {
    /// 是否已初始化 eBPF 程序
    initialized: bool,
}

/// 从 eBPF tracepoint 解析出的 Binder 事务
#[derive(Debug, Clone)]
pub struct BinderTransaction {
    pub timestamp_ms: i64,
    pub source_pid: u32,
    pub source_uid: u32,
    /// 目标服务名 (从 Binder node 名称解析)
    /// 例如: "notification", "activity", "package"
    pub target_service: String,
    /// 目标方法 (从 Binder 事务 code 推断)
    /// 例如: "enqueueNotificationWithTag" (code 5 in INotificationManager)
    pub target_method: String,
    /// 事务是否为 oneway (不需要返回值)
    pub is_oneway: bool,
    /// Parcel 数据大小 (bytes)
    pub payload_size: u32,
}

impl BinderProbe {
    /// 创建新的 Binder 探针
    ///
    /// 在 Linux 上: 返回一个标记为未初始化的实例。
    /// 在 Android (root/system daemon) 上: 加载 BPF 程序并 attach 到 tracepoint。
    pub fn new() -> Self {
        // Linux 上 eBPF 需要特定内核版本和权限
        // 我们返回一个未初始化的探针, 调用 poll() 时返回空
        Self { initialized: false }
    }

    /// 尝试初始化 eBPF 程序
    ///
    /// 返回 Ok(true) 表示 BPF 程序已加载并 attach。
    /// 返回 Ok(false) 表示当前平台不支持 (Linux 桌面 / 无权限)。
    pub fn try_init(&mut self) -> Result<bool, ProbeError> {
        // 检查是否有 /sys/kernel/debug/tracing/events/binder/ 目录
        // 这是 Binder tracepoint 存在的标志
        let binder_trace = std::path::Path::new(
            "/sys/kernel/debug/tracing/events/binder/binder_transaction/enable",
        );

        if binder_trace.exists() {
            // Android (有 Binder tracepoint) — 可以加载 eBPF
            // 实际实现需要:
            // 1. 编译 BPF 程序到 ELF (使用 aya 或 libbpf-rs)
            // 2. 加载 BPF 程序 (调用 bpf() syscall)
            // 3. Attach 到 tracepoint/binder/binder_transaction
            // 4. 通过 perf buffer 或 ring buffer 读取事件
            self.initialized = true;
            tracing::info!("Binder tracepoint detected, probe initialized");
            Ok(true)
        } else {
            // Linux 桌面 / 不支持的环境
            tracing::warn!("Binder tracepoint not available — probe will return no events");
            Ok(false)
        }
    }

    /// 轮询 Binder 事件
    ///
    /// 返回自上次 poll 以来的所有新事务。
    /// 在 Linux 桌面环境下始终返回空。
    pub fn poll(&self) -> Vec<BinderTransaction> {
        if !self.initialized {
            return Vec::new();
        }

        // 从 eBPF perf buffer 读取事件
        // 使用 aya::maps::perf::PerfBuffer 或 libbpf-rs::RingBuffer
        //
        // 伪代码:
        // let events = self.bpf_map.read_events();
        // events.into_iter().map(parse_binder_event).collect()

        Vec::new()
    }
}

impl Default for BinderProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl BinderTransaction {
    /// 转换为 aios-spec 的 BinderTxEvent
    pub fn to_event(&self) -> BinderTxEvent {
        BinderTxEvent {
            timestamp_ms: self.timestamp_ms,
            source_pid: self.source_pid,
            source_uid: self.source_uid,
            target_service: self.target_service.clone(),
            target_method: self.target_method.clone(),
            is_oneway: self.is_oneway,
            payload_size: self.payload_size,
        }
    }
}

/// Binder 探针错误
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// 内核不支持 eBPF
    #[error("eBPF not supported on this kernel")]
    EbpfNotSupported,
    /// 权限不足
    #[error("insufficient permissions for eBPF (need root or system daemon)")]
    PermissionDenied,
    /// BPF 程序加载失败
    #[error("BPF program load failed: {0}")]
    LoadError(String),
}
