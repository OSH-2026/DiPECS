//! # aios-core — DiPECS 核心引擎
//!
//! 职责: 事件聚合、隐私脱敏、策略校验、Trace 回放。
//! 内部逻辑保持同步 (Sync), 只在系统边界使用 async。

#![deny(unsafe_op_in_unsafe_fn)]

pub mod privacy_airgap;
pub mod action_bus;
pub mod policy_engine;
pub mod trace_engine;
