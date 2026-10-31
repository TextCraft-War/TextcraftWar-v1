//! 联军：S3 只有"查名称"这一个读（`我的资料` 显示所属联军用），
//! S4 补上联军的**日结批处理**；建联军 / 成员 / 税率等随 S7 落地，
//! 届时本模块按表继续长。

use domain::实体::联军Id;

use crate::连接::{事务, 执行者};
use crate::错误;

/// 查联军名（公开信息；`uid` 前缀是 `C…`，本来就是公开编号，D36）。
///
/// `None` = 这个 id 没有对应联军。玩家的 `联军_id` 是外键，
/// 指向不存在联军的外键是坏数据，调用方按数据损坏处理。
pub async fn 查联军名称(
    于: 执行者<'_>, 联军_id: 联军Id
) -> Result<Option<String>, 错误> {
    let mut 源 = 于.解析().await?;
    sqlx::query_scalar::<_, String>("SELECT 名称 FROM 联军 WHERE id = $1")
        .bind(联军_id.0)
        .fetch_optional(源.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "查询联军名称",
            来源,
        })
}

/// 联军日结（每天 0 点，D32）：把当天的资本增量落一行进
/// `联军资本增量` 历史，再把累计两列清零。
///
/// 可重入（impl.md 第五节）：历史行的主键是 `(联军_id, 日结时间)`，
/// `ON CONFLICT DO NOTHING` 让进程在两步之间被杀后重跑也不会写重。
/// `日结时间` 是**被结算那天**的 0 点（UTC+8 Unix 秒）——正常情况下是昨天；
/// 漏跑好几天时多出来的天数没法按天拆回去，只并入这一行。
pub async fn 日结联军资本(句柄: &mut 事务, 日结时间: i64) -> Result<(), 错误> {
    sqlx::query(
        "INSERT INTO 联军资本增量 (联军_id, 日结时间, 资本增量)
         SELECT id, $1, 当天资本增量 FROM 联军 WHERE 当天资本增量 > 0
         ON CONFLICT (联军_id, 日结时间) DO NOTHING",
    )
    .bind(日结时间)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "落联军资本增量",
        来源,
    })?;

    sqlx::query(
        "UPDATE 联军 SET 当天资本增量 = 0, 当天扩军累计 = 0
          WHERE 当天资本增量 > 0 OR 当天扩军累计 > 0",
    )
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "清零联军当日累计",
        来源,
    })?;

    Ok(())
}
