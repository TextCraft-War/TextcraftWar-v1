//! 频道 -> 地区的查询。
//!
//! 频道是玩家唯一的入口：游戏只在群里玩，所以"你在哪个地区"永远是
//! "你说话的这个群绑到哪个地区"。地区**不能**靠玩家自报。

use domain::平台;
use sqlx::PgConnection;

use crate::错误;

/// 频道所属的地区。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 频道地区 {
    /// 地区主键。注册时写进 `玩家.驻扎地区_id`。
    pub id: i64,
    /// 地区名称。**公开信息**，可以直接给玩家看。
    ///
    /// 这里刻意不带 `uid`：`R…` 是秘密凭证，只有"已经知道它的人"才该拿到。
    pub 名称: String,
}

/// 按 `(平台, 频道_id)` 查频道绑定的地区；`None` = 这个频道还没绑地区。
pub async fn 查频道地区(
    执行者: &mut PgConnection,
    平台: 平台,
    频道_id: &str,
) -> Result<Option<频道地区>, 错误> {
    let 行 = sqlx::query_as::<_, (i64, String)>(
        "SELECT 地区.id, 地区.名称
           FROM 地区频道绑定
           JOIN 地区 ON 地区.id = 地区频道绑定.地区_id
          WHERE 地区频道绑定.平台 = $1 AND 地区频道绑定.频道_id = $2",
    )
    .bind(平台.编码())
    .bind(频道_id)
    .fetch_optional(执行者)
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "查询频道地区",
        来源,
    })?;

    Ok(行.map(|(id, 名称)| 频道地区 { id, 名称 }))
}
