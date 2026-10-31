//! 持久化层：写 SQL，并把数据库行翻译成 `domain` 的类型。
//!
//! 依赖方向（由编译器强制）：`storage -> domain`。
//! 本 crate **不依赖** `server`/`protocol`，**不做规则**——
//! 规则归 `domain`，事务的**边界**归 `server`，这里只负责"存取"。
//!
//! 事务的**机制**也在这里（D40/D45）：连接池与事务都是 newtype，
//! `server` 拿到 [`事务`] 这个不透明句柄 + [`开事务`]/[`提交`]/[`回滚`] 三个动作，
//! 池与事务的任何 `sqlx` 固有方法它都够不着——想在那层写一条 SQL 会直接编译失败。
//! 读写分流（D46）：读查询收 [`执行者`]（池直查或事务内读），写函数只收 `&mut 事务`。
//!
//! 表结构见 [`docs/arch/data-model.md`]，实现约定见 `crates/storage/docs/impl.md`。
//!
//! [`docs/arch/data-model.md`]: ../../../docs/arch/data-model.md

// 本 crate 的实现模块全是私有的，于是 `pub(crate)` 在 clippy 眼里是"冗余"的
// （它会建议改成 `pub`；但那样又撞上 `unreachable_pub`——两边互斥，见 server/lib.rs）。
// 保留 `pub(crate)`：让"只在 crate 内可见"写在签名上。
#![allow(clippy::redundant_pub_crate)]

#[path = "region.rs"]
mod 地区;
#[path = "inventory.rs"]
mod 库存;
#[path = "player.rs"]
mod 玩家;
#[path = "system.rs"]
mod 系统;
#[path = "coalition.rs"]
mod 联军;
#[path = "rows.rs"]
mod 行;
#[path = "pool.rs"]
mod 连接;
#[path = "channel.rs"]
mod 频道;

/// 集成测试播种。只在 feature `testing` 下编译：
/// `server` 的 dev-dependencies 以 `storage = { features = ["testing"] }` 启用。
#[cfg(feature = "testing")]
#[path = "testing.rs"]
pub mod 测试辅助;

pub use 地区::查地区名称;
pub use 库存::{增库存, 库存所有者, 查库存};
pub use 玩家::{
    串行化注册, 插入玩家, 插入绑定, 显示玩家编号, 查玩家, 查绑定, 清零今日招募, 签到打卡, 绑定,
    结算生产次数,
};
pub use 系统::{写状态, 查状态, 系统状态行, 锁定状态};
pub use 联军::{日结联军资本, 查联军名称};
pub use 连接::{
    事务, 回滚, 建立连接池, 开事务, 执行者, 提交, 迁移, 连接池
};
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

    /// 数据库里的值超出了领域的表示范围或词表（窄化失败、未知编码）。
    ///
    /// 这是**数据损坏**：宁可报错让人来查，绝不静默截断或 fallback。
    #[error("数据损坏：{表}.{列}")]
    数据损坏 {
        /// 哪张表。
        表: &'static str,
        /// 哪一列。
        列: &'static str,
    },
}
