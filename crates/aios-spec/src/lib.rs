//! # aios-spec — DiPECS 宪法层
//!
//! 零内部依赖。定义全系统的核心数据结构、Trait 和 IPC 协议。
//! 所有跨模块通信必须依赖此层的抽象。

#![deny(unsafe_op_in_unsafe_fn)]

mod event;
mod context;
mod intent;
mod trace;

pub use event::*;
pub use context::*;
pub use intent::*;
pub use trace::*;

/// aios-spec 定义的公共 trait
pub mod traits {
    mod privacy;
    mod executor;
    mod trace_validator;

    pub use privacy::PrivacySanitizer;
    pub use executor::ActionExecutor;
    pub use trace_validator::TraceValidator;
}
