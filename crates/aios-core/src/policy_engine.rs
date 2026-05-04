//! 策略引擎 — 校验 LLM 返回的意图是否合法
//!
//! 职责:
//! 1. 检查风险等级是否允许自动执行
//! 2. 检查推荐的 action 是否在白名单内
//! 3. 检查目标 app 是否可操作
//! 4. 输出经过滤的可执行动作列表

use aios_spec::{ActionUrgency, Intent, IntentBatch, RiskLevel, SuggestedAction};

/// 策略校验结果
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// 原始意图 ID
    pub intent_id: String,
    /// 原始意图是否通过校验
    pub approved: bool,
    /// 被拒绝的原因 (如有)
    pub rejection_reason: Option<String>,
    /// 通过校验的动作列表 (可能少于原始列表)
    pub approved_actions: Vec<SuggestedAction>,
}

/// 策略引擎配置
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    /// 允许自动执行的最大风险等级
    pub max_auto_risk: RiskLevel,
    /// 禁止的 action 类型
    pub blocked_actions: Vec<String>,
    /// 单次最多执行的动作数
    pub max_actions_per_batch: usize,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            // 默认只允许低风险自动执行
            max_auto_risk: RiskLevel::Low,
            // 默认不禁止任何 action 类型
            blocked_actions: vec![],
            // 单次最多 5 个动作
            max_actions_per_batch: 5,
        }
    }
}

/// 策略引擎
pub struct PolicyEngine {
    config: PolicyConfig,
}

impl PolicyEngine {
    /// 使用给定配置创建策略引擎
    pub fn new(config: PolicyConfig) -> Self {
        Self { config }
    }

    /// 校验整个 IntentBatch, 返回每个意图的决策
    pub fn evaluate_batch(&self, batch: &IntentBatch) -> Vec<PolicyDecision> {
        batch
            .intents
            .iter()
            .map(|intent| self.evaluate_intent(intent))
            .collect()
    }

    /// 校验单个意图
    fn evaluate_intent(&self, intent: &Intent) -> PolicyDecision {
        // 1. 风险等级检查
        if intent.risk_level as u8 > self.config.max_auto_risk as u8 {
            return PolicyDecision {
                intent_id: intent.intent_id.clone(),
                approved: false,
                rejection_reason: Some(format!(
                    "risk level {:?} exceeds max allowed {:?}",
                    intent.risk_level, self.config.max_auto_risk
                )),
                approved_actions: vec![],
            };
        }

        // 2. 置信度检查 — 低于 0.3 的意图直接拒绝
        if intent.confidence < 0.3 {
            return PolicyDecision {
                intent_id: intent.intent_id.clone(),
                approved: false,
                rejection_reason: Some(format!("confidence {} too low", intent.confidence)),
                approved_actions: vec![],
            };
        }

        // 3. 过滤动作
        let approved_actions: Vec<SuggestedAction> = intent
            .suggested_actions
            .iter()
            .filter(|action| {
                let action_name = format!("{:?}", action.action_type);
                // 检查是否在禁止列表中
                if self
                    .config
                    .blocked_actions
                    .iter()
                    .any(|blocked| action_name.contains(blocked))
                {
                    return false;
                }
                // 检查紧迫度 — Deferred 的动作不在此次执行
                if matches!(action.urgency, ActionUrgency::Deferred) {
                    return false;
                }
                true
            })
            .take(self.config.max_actions_per_batch)
            .cloned()
            .collect();

        PolicyDecision {
            intent_id: intent.intent_id.clone(),
            approved: true,
            rejection_reason: None,
            approved_actions,
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new(PolicyConfig::default())
    }
}
