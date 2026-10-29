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

        let 首版 = 迁移集.migrations.first().expect("迁移集不该为空");
        assert_eq!(首版.version, 1, "第一份迁移必须是基础类型");
    }
}
