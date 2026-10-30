//! 连接池、事务与执行上下文。
//!
//! 连接参数由 `server` 从环境变量读入后传进来；本模块不读环境变量。
//!
//! # 为什么是 newtype 而不是类型别名（D45）
//!
//! `pub type 连接池 = PgPool` 挡不住任何东西：类型别名不会屏蔽底层类型的
//! 固有方法，`server` 虽然**不依赖** `sqlx`，却照样能写 `状态.池.begin()`、
//! `池.close()`——"不透明句柄"只是文档里的愿望。newtype 之后，
//! `server` 拿到的壳上没有任何 `sqlx` 方法可调，"只有 storage 懂数据库"
//! 从纪律变成编译期事实。

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, Postgres};

use crate::错误;

/// 数据库连接池。
///
/// `server` 能做的只有把它传给 [`建立连接池`] 的产物该去的地方（`状态`、
/// [`开事务`]、[`执行者::池`]），池本身的方法一个都看不见。
///
/// `Clone` 是安全的：`PgPool` 本来就是 Arc 句柄，克隆共享同一个池。
#[derive(Clone, Debug)]
pub struct 连接池(
    /// 底层池；仅 `storage` 内部可见。
    pub(crate) sqlx::PgPool,
);

/// 一条指令的事务。
///
/// 对 `server` 是**不透明句柄**：能做的只有把它交给本 crate 的查询函数，
/// 以及 [`开事务`] / [`提交`] / [`回滚`] 三个动作（D40/D45）。
/// 写函数的签名只收 `&mut 事务`——拿 [`连接池`] 直接写库编译不过（D46）。
pub struct 事务(
    /// 底层事务；仅 `storage` 内部可见。
    pub(crate) sqlx::Transaction<'static, Postgres>,
);

impl 事务 {
    /// storage 内部：拿到底层连接跑 SQL。
    pub(crate) fn 连接(&mut self) -> &mut PgConnection {
        // `Transaction` 自己 `DerefMut` 到 `PgConnection`，这里靠解构强转。
        &mut self.0
    }
}

/// 一条查询在哪里执行（D46）。
///
/// **读**查询两种都能用；**写**函数不收它——写只接受 `&mut 事务`，
/// 这是编译期强制的读写分流：
///
/// - 纯读路径用 [`执行者::池`]：借一条连接跑完即还，**不开事务**。
///   群聊机器人高频"查状态 / 看库存"不该为每次读占住一个池连接到 BEGIN/COMMIT。
/// - 事务内的读用 [`执行者::事务`]：读得到本事务未提交的写入
///   （注册流程的"锁住 → 再查绑定"必须这样，见 D42）。
pub enum 执行者<'a> {
    /// 池上借一条临时连接。
    池(&'a 连接池),
    /// 在一个已开的事务里执行。
    事务(&'a mut 事务),
}

/// [`执行者`] 解析后的连接来源：要么持有借来的池连接，要么指向事务。
///
/// 私有：查询函数拿 [`连接源::连接`] 用，不关心背后是哪种。
pub(crate) enum 连接源<'a> {
    /// 从池里借出、用完自动归还的连接。
    池(sqlx::pool::PoolConnection<Postgres>),
    /// 事务的底层连接。
    事务(&'a mut sqlx::Transaction<'static, Postgres>),
}

impl 连接源<'_> {
    /// 跑 SQL 用的连接。两种来源都 deref 到 `PgConnection`。
    pub(crate) fn 连接(&mut self) -> &mut PgConnection {
        match self {
            Self::池(连接) => 连接,
            Self::事务(句柄) => 句柄,
        }
    }
}

impl<'a> 执行者<'a> {
    /// 把"在哪里执行"解析成一条可用的连接。
    pub(crate) async fn 解析(self) -> Result<连接源<'a>, 错误> {
        match self {
            Self::池(池) => {
                let 连接 = 池.0.acquire().await.map_err(|来源| 错误::数据库失败 {
                    操作: "从连接池借连接",
                    来源,
                })?;
                Ok(连接源::池(连接))
            },
            Self::事务(句柄) => Ok(连接源::事务(&mut 句柄.0)),
        }
    }
}

/// 连接数据库并建好连接池。
///
/// `url` 形如 `postgres://用户:密码@主机:5432/库名`。
///
/// 上限 8：读路径借连接是**毫秒级**的（借 → 查 → 还），写路径一条指令一个事务，
/// 2 人规模 + 群机器人流量离打满还远；真到了瓶颈，第一个该查的是慢查询，不是上限。
pub async fn 建立连接池(url: &str) -> Result<连接池, 错误> {
    let 池 = PgPoolOptions::new()
        .max_connections(8)
        .connect(url)
        .await
        .map_err(|来源| 错误::连接失败 { 来源 })?;

    Ok(连接池(池))
}

/// 开一个事务。写路径每条指令恰好一个：成功提交，失败回滚。
pub async fn 开事务(池: &连接池) -> Result<事务, 错误> {
    let 事务句柄 = 池.0.begin().await.map_err(|来源| 错误::数据库失败 {
        操作: "开启事务",
        来源,
    })?;

    Ok(事务(事务句柄))
}

/// 提交事务。
pub async fn 提交(句柄: 事务) -> Result<(), 错误> {
    句柄.0.commit().await.map_err(|来源| 错误::数据库失败 {
        操作: "提交事务",
        来源,
    })
}

/// 回滚事务。
///
/// 丢弃一个 [`事务`] 也会回滚（`sqlx` 在 `Drop` 里排队执行）；
/// 这里显式回滚是为了能让"回滚也失败了"出现在日志里。
pub async fn 回滚(句柄: 事务) -> Result<(), 错误> {
    句柄.0.rollback().await.map_err(|来源| 错误::数据库失败 {
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
        .run(&池.0)
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
