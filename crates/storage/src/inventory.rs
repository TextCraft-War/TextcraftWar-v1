//! 库存：玩家背包 / 联军仓库 / 地区仓库共用一张表，靠"所有者"区分。
//!
//! 读按所有者**整体**读出全部物品（D18/D28）：一次掠夺要搬多种物资，
//! 零散查会变成 N 次往返，还会留下"搬了一半"的中间状态。
//! 未知物品编码按[`错误::数据损坏`]处理，绝不 fallback——词表在 `domain`，
//! 数据库里出现它不认识的编码就是损坏。

use std::collections::BTreeMap;

use domain::实体::{地区Id, 库存 as 库存实体, 物品, 玩家Id, 联军Id};

use crate::行::库存行;
use crate::连接::执行者;
use crate::错误;

/// 库存归谁所有，对应 `库存.所有者类型` 的落库值。
///
/// 三个变体共用一张表（迁移 `30_inventory.sql`）：掠夺就是改两行库存，
/// 不关心物资原来在谁手里。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 库存所有者 {
    /// 玩家背包。
    玩家(玩家Id),
    /// 联军仓库。
    联军(联军Id),
    /// 地区仓库。
    地区(地区Id),
}

impl 库存所有者 {
    /// 落库值（`库存.所有者类型`；`COMMENT` 里定下的三个词）。
    pub const fn 编码(self) -> &'static str {
        match self {
            Self::玩家(_) => "player",
            Self::联军(_) => "coalition",
            Self::地区(_) => "region",
        }
    }

    /// 所有者的内部主键。
    pub(crate) const fn 主键(self) -> i64 {
        match self {
            Self::玩家(id) => id.0,
            Self::联军(id) => id.0,
            Self::地区(id) => id.0,
        }
    }
}

/// 一次查出一个所有者的**全部**物品（D18）。
///
/// 纯读传 [`执行者::池`]；要在事务里基于它改写的（存入 / 取出 / 掠夺，S4+）
/// 传 [`执行者::事务`]——同一连接，读到的就是锁前的最新值。
pub async fn 查库存(
    于: 执行者<'_>, 所有者: 库存所有者
) -> Result<库存实体, 错误> {
    let mut 源 = 于.解析().await?;
    let 行 = sqlx::query_as::<_, 库存行>(
        "SELECT 物品, 数量 FROM 库存 WHERE 所有者类型 = $1 AND 所有者_id = $2",
    )
    .bind(所有者.编码())
    .bind(所有者.主键())
    .fetch_all(源.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "查询库存",
        来源,
    })?;

    let mut 袋 = BTreeMap::new();
    for 每行 in 行 {
        let 品种 = 物品::解析(&每行.物品).ok_or(错误::数据损坏 {
            表: "库存",
            列: "物品",
        })?;
        袋.insert(品种, 每行.数量);
    }

    Ok(库存实体(袋))
}
