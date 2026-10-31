//! 联军：S3 只有"查名称"这一个读（`我的资料` 显示所属联军用）。
//! 建联军 / 成员 / 税率等随 S7 落地；届时本模块按表继续长。

use domain::实体::联军Id;

use crate::连接::执行者;
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
