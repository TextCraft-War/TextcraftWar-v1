//! 频道 -> 地区的查询。
//!
//! 频道是玩家唯一的入口：游戏只在群里玩，所以"你在哪个地区"永远是
//! "你说话的这个群绑到哪个地区"。地区**不能**靠玩家自报。

use domain::实体::地区Id;
use domain::平台;

use crate::行::频道地区行;
use crate::连接::执行者;
use crate::错误;

/// 频道所属的地区。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 频道地区 {
    /// 地区主键。注册时写进 `玩家.驻扎地区_id`。
    pub id: 地区Id,
    /// 地区名称。**公开信息**，可以直接给玩家看。
    ///
    /// 这里刻意不带 `uid`：`R…` 是秘密凭证，只有"已经知道它的人"才该拿到。
    pub 名称: String,
}

impl From<频道地区行> for 频道地区 {
    fn from(行: 频道地区行) -> Self {
        Self {
            id: 地区Id(行.id),
            名称: 行.名称,
        }
    }
}

/// 按 `(平台, 频道_id)` 查频道绑定的地区；`None` = 这个频道还没绑地区。
///
/// 纯读：通常传 [`执行者::池`]（不开事务）；注册事务里传 [`执行者::事务`]
/// 走同一条连接。
pub async fn 查频道地区(
    于: 执行者<'_>,
    平台: 平台,
    频道_id: &str,
) -> Result<Option<频道地区>, 错误> {
    let mut 源 = 于.解析().await?;
    let 行 = sqlx::query_as::<_, 频道地区行>(
        "SELECT 地区.id, 地区.名称
           FROM 地区频道绑定
           JOIN 地区 ON 地区.id = 地区频道绑定.地区_id
          WHERE 地区频道绑定.平台 = $1 AND 地区频道绑定.频道_id = $2",
    )
    .bind(平台.编码())
    .bind(频道_id)
    .fetch_optional(源.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "查询频道地区",
        来源,
    })?;

    Ok(行.map(频道地区::from))
}
