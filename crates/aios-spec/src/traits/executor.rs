use crate::intent::SuggestedAction;

/// 动作执行器
///
/// 接收经 PolicyEngine 校验后的 SuggestedAction,
/// 执行真正的系统级操作 (调整 oom_score_adj, posix_fadvise 等)。
pub trait ActionExecutor {
    /// 执行单个动作
    fn execute(&self, action: &SuggestedAction) -> ActionResult;

    /// 批量执行
    fn execute_batch(&self, actions: &[SuggestedAction]) -> Vec<ActionResult> {
        actions.iter().map(|a| self.execute(a)).collect()
    }
}

/// 动作执行结果
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// 对应的动作
    pub action_type: String,
    /// 目标 (如有)
    pub target: Option<String>,
    /// 是否成功
    pub success: bool,
    /// 失败原因 (如有)
    pub error: Option<String>,
    /// 执行耗时 (微秒)
    pub latency_us: u64,
}
