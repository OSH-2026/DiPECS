//! 动作总线 — 事件派发与订阅
//!
//! 基于 tokio mpsc channel 的多生产者-单消费者事件总线。
//! 用于 adapter → core 的事件传输和 core → kernel 的动作派发。

use aios_spec::{IntentBatch, RawEvent};
use tokio::sync::mpsc;

/// 发送到云端前的上下文请求
#[derive(Debug)]
pub struct ContextRequest {
    /// 请求 ID
    pub request_id: String,
    /// 触发该请求的原始事件批次
    pub raw_events: Vec<RawEvent>,
    /// 请求时间 (epoch ms)
    pub requested_at_ms: i64,
    /// 回调: 将云端返回的 IntentBatch 发送回 core
    pub reply_tx: tokio::sync::oneshot::Sender<IntentBatch>,
}

/// 动作总线持有者
///
/// 包含两个独立通道:
/// - `raw_events_tx`: adapter 向 core 推送原始事件
/// - `intent_tx`: agent 向 core 推送云端返回的意图
pub struct ActionBus {
    /// 原始事件接收端 (adapter → core)
    pub raw_events_rx: mpsc::Receiver<RawEvent>,
    /// 原始事件发送端
    pub raw_events_tx: mpsc::Sender<RawEvent>,
    /// 云端意图接收端 (agent → core)
    pub intent_rx: mpsc::Receiver<IntentBatch>,
    /// 云端意图发送端
    pub intent_tx: mpsc::Sender<IntentBatch>,
}

impl ActionBus {
    /// 创建新的动作总线
    ///
    /// `capacity` 为事件通道的缓冲大小。
    pub fn new(capacity: usize) -> Self {
        let (raw_events_tx, raw_events_rx) = mpsc::channel(capacity);
        let (intent_tx, intent_rx) = mpsc::channel(capacity);
        Self { raw_events_rx, raw_events_tx, intent_rx, intent_tx }
    }

    /// 推送原始事件 (adapter 调用)
    pub async fn push_raw_event(&self, event: RawEvent) -> Result<(), PushError> {
        self.raw_events_tx.send(event).await.map_err(|_| PushError::ChannelClosed)
    }

    /// 推送云端意图 (agent 调用)
    pub async fn push_intent(&self, batch: IntentBatch) -> Result<(), PushError> {
        self.intent_tx.send(batch).await.map_err(|_| PushError::ChannelClosed)
    }
}

/// 推送错误
#[derive(Debug, thiserror::Error)]
pub enum PushError {
    /// 通道已关闭
    #[error("channel closed")]
    ChannelClosed,
}

impl Default for ActionBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
