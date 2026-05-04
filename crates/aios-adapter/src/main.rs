//! # dipecsd — DiPECS 系统守护进程
//!
//! 部署路径: `/system/bin/dipecsd`
//! 启动方式: `dipecsd [--no-daemon] [--verbose]`
//!
//! ## 运行模式
//!
//! - **daemon 模式** (默认): fork 到后台, 持续采集系统事件
//! - **--no-daemon**: 前台运行, 用于调试和开发
//!
//! ## 数据流
//!
//! ```
//! [Kernel] ──eBPF──→ BinderProbe ──→ RawEvent ──→ ActionBus──→PrivacyAirGap
//! [ /proc] ──read──→ ProcReader  ──→ RawEvent ──→            ↓
//! [  sys ] ──read──→ SysCollector──→ RawEvent ──→        SanitizedEvent
//!                                                              ↓
//!                                                       StructuredContext
//!                                                              ↓
//!                                                       Cloud LLM (HTTPS)
//!                                                              ↓
//!   [ LMK  ] ←──── ActionExecutor ←── PolicyDecision ←── IntentBatch
//! ```
//!
//! ## 当前实现状态 (2026-05-04)
//!
//! - ProcReader: 可用 (Linux/Android 均可)
//! - SystemStateCollector: 可用 (Linux/Android 均可, 电池/网络 fallback)
//! - BinderProbe: 接口完成, eBPF attach 待真机验证
//! - Cloud LLM 通信: 占位 (aios-agent)
//! - Action 执行: 占位

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aios_adapter::{
    binder_probe::BinderProbe,
    daemon,
    proc_reader::{self, ProcReader},
    system_collector::SystemStateCollector,
};
use aios_core::action_bus::ActionBus;
use aios_core::privacy_airgap::DefaultPrivacyAirGap;
use aios_core::policy_engine::PolicyEngine;
use aios_spec::RawEvent;

/// 系统状态采集间隔
const SYS_POLL_INTERVAL_SECS: u64 = 30;
/// Binder 事件轮询间隔
const BINDER_POLL_INTERVAL_MS: u64 = 100;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("dipecs=info".parse()?)
        )
        .init();

    // 2. 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let no_daemon = args.iter().any(|a| a == "--no-daemon");

    if !no_daemon {
        daemon::daemonize();
    }
    tracing::info!("dipecsd starting (no-daemon={})", no_daemon);

    // 3. 安装信号处理器
    let mut shutdown_rx = daemon::install_signal_handlers();

    // 4. 初始化核心组件
    let bus = ActionBus::new(4096);
    let _sanitizer = DefaultPrivacyAirGap;
    let _policy = PolicyEngine::default();

    // 5. 初始化 Binder 探针
    let mut binder_probe = BinderProbe::new();
    match binder_probe.try_init() {
        Ok(true) => tracing::info!("Binder probe initialized with eBPF"),
        Ok(false) => tracing::warn!("Binder probe unavailable — running without IPC monitoring"),
        Err(e) => tracing::error!("Binder probe init failed: {}", e),
    }

    // 6. 前一次 /proc 快照缓存 (用于 diff)
    let mut prev_proc_snapshots: HashMap<u32, proc_reader::ProcSnapshot> = HashMap::new();

    // ===== 主循环 =====
    tracing::info!("entering main event loop");

    let mut last_sys_poll = SystemTime::now() - Duration::from_secs(SYS_POLL_INTERVAL_SECS);

    loop {
        let now = timestamp_ms();

        // ---- /proc 轮询 ----
        {
            let snapshots = ProcReader::scan_all();
            let curr_map: HashMap<u32, proc_reader::ProcSnapshot> = snapshots
                .iter()
                .map(|s| (s.pid, s.clone()))
                .collect();

            let changed = proc_reader::diff_snapshots(&prev_proc_snapshots, &curr_map);
            for snap in &changed {
                let event = RawEvent::ProcStateChange(ProcReader::to_event(snap, now));
                if let Err(e) = bus.push_raw_event(event).await {
                    tracing::warn!("failed to push proc event: {}", e);
                }
            }

            if !changed.is_empty() {
                tracing::debug!("proc poll: {} processes changed", changed.len());
            }
            prev_proc_snapshots = curr_map;
        }

        // ---- 系统状态采集 ----
        {
            let elapsed = SystemTime::now()
                .duration_since(last_sys_poll)
                .unwrap_or_default();
            if elapsed.as_secs() >= SYS_POLL_INTERVAL_SECS {
                let sys_event = SystemStateCollector::snapshot(now);
                let event = RawEvent::SystemState(sys_event);
                if let Err(e) = bus.push_raw_event(event).await {
                    tracing::warn!("failed to push system state event: {}", e);
                }
                last_sys_poll = SystemTime::now();
                tracing::debug!("system state polled");
            }
        }

        // ---- Binder 事件轮询 ----
        {
            let binder_events = binder_probe.poll();
            for tx in &binder_events {
                let event = RawEvent::BinderTransaction(tx.to_event());
                if let Err(e) = bus.push_raw_event(event).await {
                    tracing::warn!("failed to push binder event: {}", e);
                }
            }
            if !binder_events.is_empty() {
                tracing::debug!("binder poll: {} transactions", binder_events.len());
            }
        }

        // ---- 检查退出信号 ----
        if shutdown_rx.try_recv().is_ok() {
            tracing::info!("shutting down");
            break;
        }

        tokio::time::sleep(Duration::from_millis(BINDER_POLL_INTERVAL_MS)).await;
    }

    tracing::info!("dipecsd stopped");
    Ok(())
}

fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
