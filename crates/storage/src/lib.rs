//! 持久化层：写 SQL，并把数据库行翻译成 `domain` 的类型。
//!
//! 依赖方向（由编译器强制）：`storage -> domain`。
//! 本 crate **不依赖** `server`/`protocol`，**不做规则、不开事务**——
//! 事务边界归 `server`，规则归 `domain`，这里只负责"存取"。
//!
//! 表结构见 [`docs/data-model.md`]，实现约定见 `crates/storage/docs/design.md`。
//!
//! [`docs/data-model.md`]: ../../../docs/data-model.md

#[path = "pool.rs"]
mod 连接;

pub use 连接::{建立连接池, 迁移, 连接池};

/// 持久化层的错误。
///
/// 只表达"哪一步失败了"；玩家可见文案由 `server` 的 `文案` 模块负责。
#[derive(Debug, thiserror::Error)]
pub enum 错误 {
    /// 建池或建连失败。
    #[error("连接数据库失败：{来源}")]
    连接失败 {
        /// `sqlx` 的原始错误。
        #[source]
        来源: sqlx::Error,
    },

    /// 迁移执行失败。
    #[error("执行迁移失败：{来源}")]
    迁移失败 {
        /// `sqlx` 的迁移错误。
        #[source]
        来源: sqlx::migrate::MigrateError,
    },
}
