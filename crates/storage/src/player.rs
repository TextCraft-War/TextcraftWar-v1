//! 玩家与平台绑定：登录查询、注册写入。
//!
//! 这里**不做策略**：编号怎么生成、撞号重试几次、要不要串行化，都由 `server` 决定；
//! 本模块只回答"这个账号是谁""这个号被占了吗""写进去了吗"。

use domain::平台;
use sqlx::PgConnection;

use crate::错误;

/// 一次登录的产物：这个平台账号绑定到了哪个玩家。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct 绑定 {
    /// 玩家主键，后续查询都用它。
    pub 玩家_id: i64,
    /// 玩家对外的六位编号（显示为 `P…`）；**秘密凭证，不得进日志**。
    pub 编号: i32,
}

/// 按平台账号查玩家。
///
/// 返回 `None` 就是"这个账号还没注册"——这是每条指令的第一个分叉。
/// 走 `平台绑定 (平台, 平台_uid)` 的唯一索引，一次索引查找。
pub async fn 查绑定(
    执行者: &mut PgConnection,
    平台: 平台,
    平台_uid: &str,
) -> Result<Option<绑定>, 错误> {
    let 行 = sqlx::query_as::<_, (i64, i32)>(
        "SELECT 玩家.id, 玩家.uid
           FROM 平台绑定
           JOIN 玩家 ON 玩家.id = 平台绑定.玩家_id
          WHERE 平台绑定.平台 = $1 AND 平台绑定.平台_uid = $2",
    )
    .bind(平台.编码())
    .bind(平台_uid)
    .fetch_optional(执行者)
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "查询平台绑定",
        来源,
    })?;

    Ok(行.map(|(玩家_id, 编号)| 绑定 { 玩家_id, 编号 }))
}

/// 把同一平台账号的注册**串行化**（[D42]）。
///
/// 取一个事务级 advisory lock，键是 `平台:账号` 的哈希。两个并发的 `注册`
/// 只有一个能同时往下走：另一个等前一个提交后才查到绑定，于是直接回"已经注册过"。
/// 锁随事务结束自动释放，不需要解锁，也不怕连接被回收——所以必须在 `注册` 的
/// 那个事务里调用，否则等于没锁。
///
/// [D42]: ../../../docs/decisions.md
pub async fn 串行化注册(
    执行者: &mut PgConnection,
    平台: 平台,
    平台_uid: &str,
) -> Result<(), 错误> {
    let 键 = format!("{}:{平台_uid}", 平台.编码());

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(键)
        .execute(执行者)
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "锁定注册账号",
            来源,
        })?;

    Ok(())
}

/// 建一个玩家行，其余列全部取默认值。
///
/// 返回 `None` 表示编号撞上了已有玩家（`ON CONFLICT (uid) DO NOTHING` 让这次插入
/// 静默失败，事务仍然可用），调用方换一个编号重试即可。
///
/// 想撞一次都很难：90 万个号、2 人规模。所以**不**在这里做重试——
/// 重试次数与随机源属于服务器策略，塞进存储层就没法在测试里构造撞号。
pub async fn 插入玩家(
    执行者: &mut PgConnection,
    编号: i32,
    驻扎地区_id: i64,
    创建时间: i64,
) -> Result<Option<i64>, 错误> {
    let 行 = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO 玩家 (uid, 驻扎地区_id, 创建时间)
              VALUES ($1, $2, $3)
         ON CONFLICT (uid) DO NOTHING
           RETURNING id",
    )
    .bind(编号)
    .bind(驻扎地区_id)
    .bind(创建时间)
    .fetch_optional(执行者)
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "写入玩家",
        来源,
    })?;

    Ok(行.map(|(玩家_id,)| 玩家_id))
}

/// 写一条平台绑定。
///
/// 调用方必须先用 [`串行化注册`] 锁住这个账号、并用 [`查绑定`] 确认它还没有玩家；
/// 因此这里**不加** `ON CONFLICT` 兜底：真撞上唯一约束说明锁没生效（程序错误），
/// 让它以数据库错误炸出来，比静默留一条没有绑定的孤儿玩家好。
pub async fn 插入绑定(
    执行者: &mut PgConnection,
    玩家_id: i64,
    平台: 平台,
    平台_uid: &str,
    绑定时间: i64,
) -> Result<(), 错误> {
    sqlx::query(
        "INSERT INTO 平台绑定 (玩家_id, 平台, 平台_uid, 绑定时间)
              VALUES ($1, $2, $3, $4)",
    )
    .bind(玩家_id)
    .bind(平台.编码())
    .bind(平台_uid)
    .bind(绑定时间)
    .execute(执行者)
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "写入平台绑定",
        来源,
    })?;

    Ok(())
}
