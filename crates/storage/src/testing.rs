//! 集成测试播种（feature `testing`，生产构建不编译）。
//!
//! `server` 的集成测试以前自带 `sqlx` dev-dependency、直接写 SQL 播种——
//! 这刺穿了"只有 storage 懂数据库"的边界：改一次表结构，测试也得跟着崩（D47）。
//! 播种收敛到这里之后：**表结构变化只改 storage 一处**，
//! `server` 的依赖表里不再有 `sqlx`。
//!
//! 只提供测试需要的最小面：播种、清理、断言用的只读查询。
//! 这些函数**不是**公开 API，签名随测试需求随时调整。

use domain::实体::{地区Id, 物品, 玩家Id, 联军Id};
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

/// 把全服结算进度拨到指定时刻（测试断言/构造漏跑场景用）。
///
/// 覆盖写单行表的两个锚点——`None` 模拟"从没结算过"。
pub async fn 发系统状态(
    池: &连接池,
    上次生产结算: Option<i64>,
    上次日结: Option<i64>,
) -> Result<(), 错误> {
    let mut 句柄 = 开事务(池).await?;
    sqlx::query("UPDATE 系统状态 SET 上次生产结算 = $1, 上次日结 = $2 WHERE id = 1")
        .bind(上次生产结算)
        .bind(上次日结)
        .execute(句柄.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "拨测试系统状态",
            来源,
        })?;

    提交(句柄).await
}

/// 把玩家的生产次数与今日招募数量拨到指定值（构造"快满 / 已满"的断言场景用）。
pub async fn 改玩家生产(
    池: &连接池,
    玩家_id: 玩家Id,
    生产次数: i64,
    今日招募数量: i64,
) -> Result<(), 错误> {
    let mut 句柄 = 开事务(池).await?;
    sqlx::query("UPDATE 玩家 SET 生产次数 = $1, 今日招募数量 = $2 WHERE id = $3")
        .bind(生产次数)
        .bind(今日招募数量)
        .bind(玩家_id.0)
        .execute(句柄.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "拨测试玩家生产",
            来源,
        })?;

    提交(句柄).await
}

/// 把玩家的签到锚点拨到指定时刻（`None` = 从没签过），构造冷却断言场景用。
pub async fn 改玩家签到(
    池: &连接池,
    玩家_id: 玩家Id,
    上次签到: Option<i64>,
) -> Result<(), 错误> {
    let mut 句柄 = 开事务(池).await?;
    sqlx::query("UPDATE 玩家 SET 上次签到时间 = $1 WHERE id = $2")
        .bind(上次签到)
        .bind(玩家_id.0)
        .execute(句柄.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "拨测试玩家签到",
            来源,
        })?;

    提交(句柄).await
}
/// 播种一支联军并拨好当日累计（幂等：按 `编号` upsert，历史行清空），返回联军 id。
///
/// 清历史行是让"日结写了几行"的断言可重复跑。
pub async fn 播种联军(
    池: &连接池,
    编号: i32,
    名称: &str,
    当天资本增量: i64,
    当天扩军累计: i64,
) -> Result<联军Id, 错误> {
    let mut 句柄 = 开事务(池).await?;
    let 联军_id: i64 = sqlx::query_scalar(
        "INSERT INTO 联军 (uid, 名称, 创建时间, 当天资本增量, 当天扩军累计)
              VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (uid) DO UPDATE
              SET 名称 = EXCLUDED.名称,
                  当天资本增量 = EXCLUDED.当天资本增量,
                  当天扩军累计 = EXCLUDED.当天扩军累计
         RETURNING id",
    )
    .bind(编号)
    .bind(名称)
    .bind(现在())
    .bind(当天资本增量)
    .bind(当天扩军累计)
    .fetch_one(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "播种测试联军",
        来源,
    })?;

    sqlx::query("DELETE FROM 联军资本增量 WHERE 联军_id = $1")
        .bind(联军_id)
        .execute(句柄.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "清理测试联军历史",
            来源,
        })?;

    提交(句柄).await?;

    Ok(联军Id(联军_id))
}

/// 查联军的当日累计（断言日结清零用）：`(当天资本增量, 当天扩军累计)`。
pub async fn 查联军当日(池: &连接池, 联军_id: 联军Id) -> Result<(i64, i64), 错误> {
    let mut 源 = 执行者::池(池).解析().await?;
    let 行: (i64, i64) =
        sqlx::query_as("SELECT 当天资本增量, 当天扩军累计 FROM 联军 WHERE id = $1")
            .bind(联军_id.0)
            .fetch_one(源.连接())
            .await
            .map_err(|来源| 错误::数据库失败 {
                操作: "查询测试联军当日累计",
                来源,
            })?;

    Ok(行)
}

/// 查联军的历史日结行（断言"一天一行、重跑不重复"用），按 `日结时间` 升序。
pub async fn 查联军资本增量行(
    池: &连接池,
    联军_id: 联军Id,
) -> Result<Vec<(i64, i64)>, 错误> {
    let mut 源 = 执行者::池(池).解析().await?;
    let 行: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT 日结时间, 资本增量 FROM 联军资本增量
          WHERE 联军_id = $1 ORDER BY 日结时间 ASC",
    )
    .bind(联军_id.0)
    .fetch_all(源.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "查询测试联军资本增量",
        来源,
    })?;

    Ok(行)
}
