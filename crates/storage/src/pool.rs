//! 连接池与迁移。
//!
//! 连接参数由 `server` 从环境变量读入后传进来；本模块不读环境变量。

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::错误;

/// 连接池。
///
/// 用别名而不是 newtype：`server` 要开事务，必须能拿到底层的
/// `PgPool`/`Transaction`，包一层只会让拆包代码到处出现。
pub type 连接池 = PgPool;

/// 连接数据库并建好连接池。
///
/// `url` 形如 `postgres://用户:密码@主机:5432/库名`。
pub async fn 建立连接池(url: &str) -> Result<连接池, 错误> {
    PgPoolOptions::new()
        .max_connections(8)
        .connect(url)
        .await
        .map_err(|来源| 错误::连接失败 { 来源 })
}

/// 一条指令的事务。
///
/// 对 `server` 来说是**不透明句柄**：它只能把 `&mut 事务` 交给本 crate 的函数，
/// 不能自己发 SQL，也看不到 `sqlx` 的类型（[D40]）。
///
/// [D40]: ../../../docs/decisions.md
pub type 事务 = sqlx::Transaction<'static, sqlx::Postgres>;

/// 开一个事务。每条指令恰好一个：成功提交，失败回滚。
pub async fn 开事务(池: &连接池) -> Result<事务, 错误> {
    池.begin().await.map_err(|来源| 错误::数据库失败 {
        操作: "开启事务",
        来源,
    })
}

/// 提交事务。
pub async fn 提交(事务: 事务) -> Result<(), 错误> {
    事务.commit().await.map_err(|来源| 错误::数据库失败 {
        操作: "提交事务",
        来源,
    })
}

/// 回滚事务。
///
/// 丢弃一个 [`事务`] 也会回滚（`sqlx` 在 `Drop` 里排队执行）；
/// 这里显式回滚是为了能让"回滚也失败了"出现在日志里。
pub async fn 回滚(事务: 事务) -> Result<(), 错误> {
    事务.rollback().await.map_err(|来源| 错误::数据库失败 {
        操作: "回滚事务",
        来源,
    })
}

/// 执行编译期内嵌的迁移。
///
/// 迁移文件在 `crates/storage/migrations/`，由 `sqlx::migrate!` 在编译期读入，
/// 所以运行时不依赖文件系统，也能离线部署。启动时调用一次即可：已执行过的迁移会被跳过。
pub async fn 迁移(池: &连接池) -> Result<(), 错误> {
    sqlx::migrate!("./migrations")
        .run(池)
        .await
        .map_err(|来源| 错误::迁移失败 { 来源 })
}

#[cfg(test)]
mod tests {
    /// 迁移目录的路径只在编译期有意义：路径写错、文件被排除，
    /// 这里会直接失败，不用等到连上数据库才发现。
    #[test]
    fn 迁移文件已内嵌() {
        let 迁移集 = sqlx::migrate!("./migrations");
        assert!(!迁移集.migrations.is_empty(), "迁移目录没被内嵌");
        assert!(迁移集.migrations.iter().all(|迁移| !迁移.sql.is_empty()));

        // sqlx 的契约：版本号来自文件名前缀，必须是大于零的整数，按升序执行。
        // `00_` 开头的文件在 0.8.6 里能跑，但违反契约，将来可能被当成非法文件名静默忽略；
        // 让它在 `cargo test` 里炸掉，比在线上"少建一张表"好。
        let 版本号: Vec<i64> = 迁移集.migrations.iter().map(|迁移| 迁移.version).collect();
        assert!(
            版本号.iter().all(|版本| *版本 >= 1),
            "迁移版本号必须 ≥ 1：{版本号:?}"
        );
        assert!(
            版本号
                .iter()
                .zip(版本号.iter().skip(1))
                .all(|(前者, 后者)| 前者 < 后者),
            "迁移版本号必须严格递增：{版本号:?}"
        );

        let 首版 = 迁移集.migrations.first().expect("迁移集不该为空");
        assert_eq!(首版.version, 1, "第一份迁移必须是基础类型");
    }
}
