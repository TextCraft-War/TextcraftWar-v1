//! 玩家与平台绑定：登录查询、注册写入、玩家详情。
//!
//! 这里**不做策略**：编号怎么生成、撞号重试几次、要不要串行化，都由 `server` 决定；
//! 本模块只回答"这个账号是谁""这个号被占了吗""写进去了吗"。
//!
//! 读写分流（D46）：**读**收 [`执行者`]（池直查或事务内读都行），
//! **写**只收 `&mut 事务`——拿池直接写库编译不过。

use domain::实体::{时间, 玩家 as 玩家实体, 玩家Id, 玩家Uid};
use domain::平台;
use domain::编号::玩家前缀;

use crate::行::{玩家行, 绑定行};
use crate::连接::{事务, 执行者};
use crate::错误;

/// 一次登录的产物：这个平台账号绑定到了哪个玩家。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct 绑定 {
    /// 玩家主键，后续查询都用它。
    pub 玩家_id: 玩家Id,
    /// 玩家对外的六位编号（显示为 `P…`）；**秘密凭证，不得进日志**。
    pub 编号: 玩家Uid,
}

impl From<绑定行> for 绑定 {
    fn from(行: 绑定行) -> Self {
        Self {
            玩家_id: 玩家Id(行.玩家_id),
            编号: 玩家Uid(行.编号),
        }
    }
}

/// 按平台账号查玩家。
///
/// 返回 `None` 就是"这个账号还没注册"——这是每条指令的第一个分叉。
/// 走 `平台绑定 (平台, 平台_uid)` 的唯一索引，一次索引查找。
///
/// 纯读（未知指令的身份判定）用 [`执行者::池`]；注册流程里要读"本事务刚写的
/// 东西"（advisory lock 之后确认绑定，D42）必须传 [`执行者::事务`]。
pub async fn 查绑定(
    于: 执行者<'_>,
    平台: 平台,
    平台_uid: &str,
) -> Result<Option<绑定>, 错误> {
    let mut 源 = 于.解析().await?;
    let 行 = sqlx::query_as::<_, 绑定行>(
        "SELECT 玩家.id AS 玩家_id, 玩家.uid AS 编号
           FROM 平台绑定
           JOIN 玩家 ON 玩家.id = 平台绑定.玩家_id
          WHERE 平台绑定.平台 = $1 AND 平台绑定.平台_uid = $2",
    )
    .bind(平台.编码())
    .bind(平台_uid)
    .fetch_optional(源.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "查询平台绑定",
        来源,
    })?;

    Ok(行.map(绑定::from))
}

/// 读一个玩家的完整状态（`我的资料` 与一切规则计算的主入口）。
///
/// 纯读路径传 [`执行者::池`]；要在事务里基于它写回的，传 [`执行者::事务`]。
pub async fn 查玩家(
    于: 执行者<'_>, 玩家_id: 玩家Id
) -> Result<Option<玩家实体>, 错误> {
    let mut 源 = 于.解析().await?;
    let 行 = sqlx::query_as::<_, 玩家行>("SELECT * FROM 玩家 WHERE id = $1")
        .bind(玩家_id.0)
        .fetch_optional(源.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "查询玩家详情",
            来源,
        })?;

    行.map(玩家实体::try_from).transpose()
}

/// 把同一平台账号的注册**串行化**（D42）。
///
/// 取一个事务级 advisory lock，键是 `平台:账号` 的哈希。两个并发的 `注册`
/// 只有一个能同时往下走：另一个等前一个提交后才查到绑定，于是直接回"已经注册过"。
/// 锁随事务结束自动释放，不需要解锁，也不怕连接被回收——所以必须在 `注册` 的
/// 那个事务里调用，否则等于没锁。
pub async fn 串行化注册(
    句柄: &mut 事务, 平台: 平台, 平台_uid: &str
) -> Result<(), 错误> {
    let 键 = format!("{}:{平台_uid}", 平台.编码());

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(键)
        .execute(句柄.连接())
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
/// 静默失败，事务仍然可用），调用方换一个编号重试即可（上限见
/// `domain::编号::重试上限`，D44）。
///
/// 想撞一次都很难：90 万个号、2 人规模。所以**不**在这里做重试——
/// 重试次数与随机源属于服务器策略，塞进存储层就没法在测试里构造撞号。
pub async fn 插入玩家(
    句柄: &mut 事务,
    编号: 玩家Uid,
    驻扎地区_id: i64,
    创建时间: 时间,
) -> Result<Option<玩家Id>, 错误> {
    let 行 = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO 玩家 (uid, 驻扎地区_id, 创建时间)
              VALUES ($1, $2, $3)
         ON CONFLICT (uid) DO NOTHING
           RETURNING id",
    )
    .bind(编号.0)
    .bind(驻扎地区_id)
    .bind(创建时间.0)
    .fetch_optional(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "写入玩家",
        来源,
    })?;

    Ok(行.map(|(玩家_id,)| 玩家Id(玩家_id)))
}

/// 写一条平台绑定。
///
/// 调用方必须先用 [`串行化注册`] 锁住这个账号、并用 [`查绑定`] 确认它还没有玩家；
/// 因此这里**不加** `ON CONFLICT` 兜底：真撞上唯一约束说明锁没生效（程序错误），
/// 让它以数据库错误炸出来，比静默留一条没有绑定的孤儿玩家好。
pub async fn 插入绑定(
    句柄: &mut 事务,
    玩家_id: 玩家Id,
    平台: 平台,
    平台_uid: &str,
    绑定时间: 时间,
) -> Result<(), 错误> {
    sqlx::query(
        "INSERT INTO 平台绑定 (玩家_id, 平台, 平台_uid, 绑定时间)
              VALUES ($1, $2, $3, $4)",
    )
    .bind(玩家_id.0)
    .bind(平台.编码())
    .bind(平台_uid)
    .bind(绑定时间.0)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "写入平台绑定",
        来源,
    })?;

    Ok(())
}

/// 编号显示的便捷出口：`P123456`。
///
/// 放这里只为让 `server` 不必自己拼前缀；它是**给本人看的**，
/// 不许出现在日志里。
pub fn 显示玩家编号(编号: 玩家Uid) -> String {
    domain::编号::显示(玩家前缀, 编号.0)
}

/// 全服批处理：给所有玩家补上 `小时数` 个生产次数（`LEAST` 到上限为止，D32）。
///
/// 一条 UPDATE 覆盖全服（impl.md 第五节）；已到上限的行由 `WHERE` 跳过，
/// 不做无谓重写。漏跑几小时就补几小时，进度锚点归 `系统状态`（单行表）。
/// 返回实际推进的行数（日志用）。
pub async fn 结算生产次数(句柄: &mut 事务, 小时数: i64) -> Result<u64, 错误> {
    let 上限 = i64::from(domain::实体::玩家::生产次数上限);
    let 结果 = sqlx::query(
        "UPDATE 玩家
            SET 生产次数 = LEAST(生产次数 + $1, $2)
          WHERE 生产次数 < $2",
    )
    .bind(小时数)
    .bind(上限)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "结算生产次数",
        来源,
    })?;

    Ok(结果.rows_affected())
}

/// 全服批处理：把所有人的今日招募数量清零（每天 0 点，D32）。
///
/// `WHERE 今日招募数量 > 0` 让已清零的库不用重写。返回实际清零的行数。
pub async fn 清零今日招募(句柄: &mut 事务) -> Result<u64, 错误> {
    let 结果 = sqlx::query("UPDATE 玩家 SET 今日招募数量 = 0 WHERE 今日招募数量 > 0")
        .execute(句柄.连接())
        .await
        .map_err(|来源| 错误::数据库失败 {
            操作: "清零今日招募",
            来源,
        })?;

    Ok(结果.rows_affected())
}

/// `签到` 打卡：把 `上次签到时间` 推进到 `时刻`（S4）。
///
/// 条件更新（impl.md 2.4），`WHERE` 里的谓词**就是签到规则本身**
/// （从没签过，或距上次已隔一个冷却期，冷却秒取
/// `domain::玩家::签到冷却秒`——storage 不私定规则数值）：
/// 并发的第二次签到在数据库层面就被拦下（影响行数 0），不会重复发奖。
/// 返回 `false` = 没抢到，调用方按"冷却中"拒绝；此时不必重读重算——
/// 唯一能赢的写法就是把锚点推到"刚刚"，结果是确定的。
pub async fn 签到打卡(
    句柄: &mut 事务, 玩家_id: 玩家Id, 时刻: 时间
) -> Result<bool, 错误> {
    let 最早可签 = 时刻.0.saturating_sub(玩家实体::签到冷却秒);
    let 结果 = sqlx::query(
        "UPDATE 玩家
            SET 上次签到时间 = $1
          WHERE id = $2
            AND (上次签到时间 IS NULL OR 上次签到时间 <= $3)",
    )
    .bind(时刻.0)
    .bind(玩家_id.0)
    .bind(最早可签)
    .execute(句柄.连接())
    .await
    .map_err(|来源| 错误::数据库失败 {
        操作: "签到打卡",
        来源,
    })?;

    Ok(结果.rows_affected() > 0)
}

#[cfg(test)]
mod 测试 {
    use super::显示玩家编号;
    use domain::实体::玩家Uid;

    #[test]
    fn 玩家编号带_p_前缀显示() {
        assert_eq!(显示玩家编号(玩家Uid(123_456)), "P123456");
    }
}
