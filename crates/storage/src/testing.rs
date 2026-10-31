//! 集成测试播种（feature `testing`，生产构建不编译）。
//!
//! `server` 的集成测试以前自带 `sqlx` dev-dependency、直接写 SQL 播种——
//! 这刺穿了"只有 storage 懂数据库"的边界：改一次表结构，测试也得跟着崩（D47）。
//! 播种收敛到这里之后：**表结构变化只改 storage 一处**，
//! `server` 的依赖表里不再有 `sqlx`。
//!
//! 只提供测试需要的最小面：播种、清理、断言用的只读查询。
//! 这些函数**不是**公开 API，签名随测试需求随时调整。

use domain::实体::{地区Id, 物品, 玩家Id};
use domain::平台;

use crate::库存所有者;
use crate::玩家;
use crate::连接::{事务, 开事务, 执行者, 提交, 连接池};
use crate::错误;

/// 当前 Unix 秒。测试播种用，不进生产代码。
fn 现在() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |时长| i64::try_from(时长.as_secs()).unwrap_or(i64::MAX))
}

async fn 清理账号于(
    句柄: &mut 事务, 平台: 平台, 平台_uid: &str
) -> Result<(), 错误> {
    sqlx::query(
        "DELETE FROM 玩家
          WHERE id IN (SELECT 玩家_id FROM 平台绑定
                        WHERE 平台 = $1 AND 平台_uid = $2)",
    )
    .bind(平台.编码())
    .bind(平台_uid)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "清理测试玩家",
        来源,
    })?;

    sqlx::query("DELETE FROM 平台绑定 WHERE 平台 = $1 AND 平台_uid = $2")
        .bind(平台.编码())
        .bind(平台_uid)
        .execute(句柄.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "清理测试绑定",
            来源,
        })?;

    Ok(())
}

/// 清掉某测试账号名下的玩家与绑定，让用例可重复跑。
pub async fn 清理账号(池: &连接池, 平台: 平台, 平台_uid: &str) -> Result<(), 错误> {
    let mut 句柄 = 开事务(池).await?;
    清理账号于(&mut 句柄, 平台, 平台_uid).await?;
    提交(句柄).await
}

/// 播种一个"可注册"的环境，返回地区 id：
///
/// 1. 清理 `账号` 名下的旧数据（可重复跑）；
/// 2. 把 `已绑定频道` 上的旧绑定改绑 / 把 `未绑定频道` 解绑；
/// 3. upsert 一个地区（按 `地区编号` 幂等）并把 `已绑定频道` 绑上去。
///
/// 全程一个事务：半播种状态不会留下来。
pub async fn 播种注册环境(
    池: &连接池,
    平台: 平台,
    账号: &str,
    地区编号: i32,
    地区名称: &str,
    已绑定频道: &str,
    未绑定频道: &str,
) -> Result<地区Id, 错误> {
    let mut 句柄 = 开事务(池).await?;
    let 时间戳 = 现在();

    清理账号于(&mut 句柄, 平台, 账号).await?;

    // 两个频道都先解绑，再绑上"已绑定频道"——幂等，无论上次跑到哪。
    for 频道_id in [已绑定频道, 未绑定频道] {
        sqlx::query("DELETE FROM 地区频道绑定 WHERE 平台 = $1 AND 频道_id = $2")
            .bind(平台.编码())
            .bind(频道_id)
            .execute(句柄.连接())
            .await
            .map_err(|来源| 错误::数据库失败 {
                操作: "清理测试频道绑定",
                来源,
            })?;
    }

    let 地区_id: i64 = sqlx::query_scalar(
        "INSERT INTO 地区 (uid, 名称, 创建时间) VALUES ($1, $2, $3)
         ON CONFLICT (uid) DO UPDATE SET 名称 = EXCLUDED.名称
         RETURNING id",
    )
    .bind(地区编号)
    .bind(地区名称)
    .bind(时间戳)
    .fetch_one(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "播种测试地区",
        来源,
    })?;

    sqlx::query(
        "INSERT INTO 地区频道绑定 (地区_id, 平台, 频道_id, 绑定时间)
              VALUES ($1, $2, $3, $4)
         ON CONFLICT (平台, 频道_id) DO UPDATE SET 地区_id = EXCLUDED.地区_id",
    )
    .bind(地区_id)
    .bind(平台.编码())
    .bind(已绑定频道)
    .bind(时间戳)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "播种测试频道绑定",
        来源,
    })?;

    提交(句柄).await?;

    Ok(地区Id(地区_id))
}

/// 查账号绑定的玩家编号（测试断言用）。
pub async fn 查绑定编号(
    池: &连接池,
    平台: 平台,
    平台_uid: &str,
) -> Result<Option<i32>, 错误> {
    Ok(玩家::查绑定(执行者::池(池), 平台, 平台_uid)
        .await?
        .map(|绑定| 绑定.编号.0))
}

/// 查账号绑定的玩家主键（播种库存等需要引用玩家的用例用）。
pub async fn 查玩家_id(
    池: &连接池,
    平台: 平台,
    平台_uid: &str,
) -> Result<Option<玩家Id>, 错误> {
    Ok(玩家::查绑定(执行者::池(池), 平台, 平台_uid)
        .await?
        .map(|绑定| 绑定.玩家_id))
}

/// 给一个所有者发物资（幂等：重复跑覆盖成同值）。
///
/// 直写 `库存` 表是**测试专用的播种口**：正常游戏里物资只会经规则函数增减，
/// 这里绕过它制造已知库存，好断言展示与聚合逻辑。
pub async fn 发库存(
    池: &连接池,
    所有者: 库存所有者,
    物品: 物品,
    数量: i64,
) -> Result<(), 错误> {
    let mut 句柄 = 开事务(池).await?;
    sqlx::query(
        "INSERT INTO 库存 (所有者类型, 所有者_id, 物品, 数量)
              VALUES ($1, $2, $3, $4)
         ON CONFLICT (所有者类型, 所有者_id, 物品) DO UPDATE SET 数量 = $4",
    )
    .bind(所有者.编码())
    .bind(所有者.主键())
    .bind(物品.编码())
    .bind(数量)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "播种测试库存",
        来源,
    })?;

    提交(句柄).await
}

/// 查玩家编号对应的驻扎地区 id（测试断言用）。
pub async fn 查驻扎地区(池: &连接池, 编号: i32) -> Result<Option<i64>, 错误> {
    let mut 源 = 执行者::池(池).解析().await?;
    let 行: Option<i64> = sqlx::query_scalar("SELECT 驻扎地区_id FROM 玩家 WHERE uid = $1")
        .bind(编号)
        .fetch_optional(源.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "查询测试驻扎地区",
            来源,
        })?;

    Ok(行)
}

/// 查账号名下的绑定条数（测试断言用：第二次注册不得再建绑定）。
pub async fn 绑定条数(
    池: &连接池, 平台: 平台, 平台_uid: &str
) -> Result<i64, 错误> {
    let mut 源 = 执行者::池(池).解析().await?;
    let 条数: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM 平台绑定 WHERE 平台 = $1 AND 平台_uid = $2")
            .bind(平台.编码())
            .bind(平台_uid)
            .fetch_one(源.连接())
            .await
            .map_err(|来源| 错误::数据库失败 {
                操作: "统计测试绑定条数",
                来源,
            })?;

    Ok(条数)
}
