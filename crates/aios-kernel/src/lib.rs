//! # aios-kernel — 资源生命周期管理与 IPC 协调
//!
//! 职责: 接收 PolicyEngine 校验通过的动作, 执行系统级操作。
//!
//! 当前阶段提供骨架实现, 所有操作通过 tracing 记录。
//! 后续在真机/模拟器上实现真实的 syscall 调用。

use aios_spec::traits::ActionExecutor;
use aios_spec::traits::ActionResult;
use aios_spec::ActionType;
use aios_spec::SuggestedAction;
use std::time::Instant;

/// 默认动作执行器
///
/// 执行经 PolicyEngine 校验后的 SuggestedAction。
/// 当前为骨架实现: 记录操作日志, 返回占位结果。
/// 后续将对接真实 syscall:
/// - PreWarmProcess → fork zygote, 调整 cgroup
/// - PrefetchFile → posix_fadvise(POSIX_FADV_WILLNEED)
/// - KeepAlive → /proc/pid/oom_score_adj 调整
/// - ReleaseMemory → /proc/pid/reclaim 或 process_madvise(MADV_COLD)
pub struct DefaultActionExecutor;

impl ActionExecutor for DefaultActionExecutor {
    fn execute(&self, action: &SuggestedAction) -> ActionResult {
        let start = Instant::now();
        let action_name = format!("{:?}", action.action_type);

        let (success, error) = match action.action_type {
            ActionType::PreWarmProcess => {
                if let Some(ref target) = action.target {
                    tracing::info!(
                        target = %target,
                        urgency = ?action.urgency,
                        "PreWarmProcess: stub (zygote fork not yet implemented)"
                    );
                    (true, None)
                } else {
                    (false, Some("PreWarmProcess requires a target app".into()))
                }
            },
            ActionType::PrefetchFile => {
                tracing::info!(
                    target = ?action.target,
                    urgency = ?action.urgency,
                    "PrefetchFile: stub (posix_fadvise not yet implemented)"
                );
                (true, None)
            },
            ActionType::KeepAlive => {
                if let Some(ref target) = action.target {
                    tracing::info!(
                        target = %target,
                        urgency = ?action.urgency,
                        "KeepAlive: stub (oom_score_adj write not yet implemented)"
                    );
                    (true, None)
                } else {
                    tracing::info!("KeepAlive: no target specified, skipping");
                    (true, None)
                }
            },
            ActionType::ReleaseMemory => {
                tracing::info!(
                    target = ?action.target,
                    urgency = ?action.urgency,
                    "ReleaseMemory: stub (/proc/pid/reclaim not yet implemented)"
                );
                (true, None)
            },
            ActionType::NoOp => {
                tracing::debug!("NoOp executed");
                (true, None)
            },
        };

        ActionResult {
            action_type: action_name,
            target: action.target.clone(),
            success,
            error,
            latency_us: start.elapsed().as_micros() as u64,
        }
    }
}

impl Default for DefaultActionExecutor {
    fn default() -> Self {
        Self
    }
}
