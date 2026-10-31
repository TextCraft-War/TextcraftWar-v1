//! 地区：v1 只有读。建地区没有指令，靠 SQL 播种；地区也**不可删除**，
//! 只会易主（design.md 2.3）——所以按 id 查不到名称本身就是数据损坏。

use domain::实体::地区Id;

use crate::连接::执行者;
use crate::错误;

/// 查地区名（公开信息；`uid` 是秘密凭证，不走这个口）。
///
/// `None` = 这个 id 没有对应地区。调用方（`我的资料` 等）把它当数据损坏：
/// 玩家的 `驻扎地区_id` 是外键，地区又不可删除，指向不存在地区的外键只可能是坏数据。
pub async fn 查地区名称(
    于: 执行者<'_>, 地区_id: 地区Id
) -> Result<Option<String>, 错误> {
    let mut 源 = 于.解析().await?;
    sqlx::query_scalar::<_, String>("SELECT 名称 FROM 地区 WHERE id = $1")
        .bind(地区_id.0)
        .fetch_optional(源.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "查询地区名称",
            来源,
        })
}
