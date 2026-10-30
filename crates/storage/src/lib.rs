//! 持久化层：写 SQL，并把数据库行翻译成 `domain` 的类型。
//!
//! 依赖方向（由编译器强制）：`storage -> domain`。
//! 本 crate **不依赖** `server`/`protocol`，**不做规则**——
//! 规则归 `domain`，事务的**边界**归 `server`，这里只负责"存取"。
//!
//! 但事务的**机制**在这里：`sqlx` 的类型不导出给 `server`，
//! 它拿到的是 [`事务`] 这个不透明句柄 + [`开事务`]/[`提交`]/[`回滚`] 三个动作（[D40]）。
//! 这样"只有 storage 懂数据库"是编译期的事实，不是纪律。
//!
//! 表结构见 [`docs/arch/data-model.md`]，实现约定见 `crates/storage/docs/impl.md`。
//!
//! [`docs/arch/data-model.md`]: ../../../docs/arch/data-model.md
//! [D40]: ../../../docs/decisions.md

#[path = "player.rs"]
mod 玩家;
#[path = "pool.rs"]
mod 连接;
#[path = "channel.rs"]
mod 频道;

pub use 玩家::{串行化注册, 插入玩家, 插入绑定, 查绑定, 绑定};
pub use 连接::{事务, 回滚, 建立连接池, 开事务, 提交, 迁移, 连接池};
pub use 频道::{查频道地区, 频道地区};

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

    /// 一次查询、写入或事务操作失败。
    ///
    /// `操作` 是中文短语（如 `查询平台绑定`），只进日志，不给玩家看。
    #[error("{操作}失败：{来源}")]
    数据库失败 {
        /// 正在做什么。
        操作: &'static str,
        /// `sqlx` 的原始错误。
        #[source]
        来源: sqlx::Error,
    },
}
