//! S4 第一块：调度器**漏跑补算**的端到端验证（真库）。
//!
//! 覆盖三件事（impl.md 第五节）：生产次数按整小时补齐且封顶、
//! 今日招募清零 + 联军日结（历史落一行、累计清零）、以及**重跑幂等**——
//! 把进度锚点拨回去再跑一轮，不许重复加次数、不许写第二行日结。
//!
//! 全部断言用例共享单行表 `系统状态`，所以本文件只放**一个**测试函数，
//! 各阶段在函数内顺序执行（并行用例会在这一行上互相拨锚点）。
//!
//! 播种与断言走 `storage::测试辅助`；连库与跳过策略见 [`tests::common`]。
//!
//! [`tests::common`]: ./common/mod.rs

// 本测试是一个独立的 crate，会看到包的全部依赖；用不到的（`axum`、`rand`…）
// 不该因此被当成"多余依赖"删掉——它们是库侧要的。
#![allow(unused_crate_dependencies, reason = "测试只用到库的一小部分依赖")]
// `common` 是私有模块，里面的 `pub(crate)` 函数会被 redundant_pub_crate 盯上；
// 改 `pub` 又被 unreachable_pub 盯上——两边互斥（lib.rs 同款注释），照 lib.rs 放行。
#![allow(clippy::redundant_pub_crate)]

mod common;

use common::连库;
use domain::平台;
use server::{处理, 来信};

/// 独立账号 / 地区 / 频道：与本目录其他测试互不共享（并行安全）。
const 账号: &str = "测试账号-补算";
const 测试地区编号: i32 = 990_002;
const 已绑定频道: &str = "测试频道-补算";
const 未绑定频道: &str = "测试频道-补算-未绑";

/// 独立联军编号（`C…` 落库只存六位数字）。
const 联军编号: i32 = 990_102;
const 联军名称: &str = "测试联军-补算";

/// 当前 Unix 秒。测试自己的"现在"，与 [`server::巡检`] 内部取的时刻
/// 最多差几秒——断言都对差值不敏感（见各阶段注释）。
fn 当前秒() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |时长| i64::try_from(时长.as_secs()).unwrap_or(i64::MAX))
}

/// "最近一个已结束的自然日"的 0 点（UTC+8）。
///
/// **故意**在测试里独立实现一遍日界公式：调度器算错了，测试按同一定义
/// 算出的期望值才会暴露它；两边抄同一份代码反而测不出错。
const fn 昨日零点(现在: i64) -> i64 {
    let 日序 = 现在.saturating_add(8 * 3600).div_euclid(86_400);
    日序
        .saturating_sub(1)
        .saturating_mul(86_400)
        .saturating_sub(8 * 3600)
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 漏跑补算_生产封顶_招募清零_联军日结且幂等() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };

    // ── 播种：一名玩家（生产次数 7、今日招募 7）、一支联军（当日增量 500、
    //    扩军累计 3）；生产锚点拨回 2 小时多之前，"昨天"还没日结。
    storage::测试辅助::播种注册环境(
        &状态.池,
        平台::Onebot,
        账号,
        测试地区编号,
        "补算地区",
        已绑定频道,
        未绑定频道,
    )
    .await?;

    let 注册信 = 来信 {
        平台: 平台::Onebot,
        频道_id: 已绑定频道.to_owned(),
        用户_uid: 账号.to_owned(),
        指令名: "注册".to_owned(),
        参数: serde_json::Value::Null,
    };
    处理(&状态, &注册信).await?;

    let 玩家_id = storage::测试辅助::查玩家_id(&状态.池, 平台::Onebot, 账号)
        .await?
        .ok_or("应当已注册")?;
    storage::测试辅助::改玩家生产(&状态.池, 玩家_id, 7, 7).await?;

    let 联军_id = storage::测试辅助::播种联军(&状态.池, 联军编号, 联军名称, 500, 3).await?;

    let 现在 = 当前秒();
    let 上次锚点 = 现在.saturating_sub(7205); // 2 小时 + 5 秒之前
    let 昨天零点 = 昨日零点(现在);
    storage::测试辅助::发系统状态(
        &状态.池,
        Some(上次锚点),
        Some(昨天零点.saturating_sub(86_400)), // 只结到前天 → 昨天 欠一次
    )
    .await?;

    // ── 补算一轮。
    server::巡检(&状态.池).await?;

    // 生产：7 + 2 = 9，封在上限 8；招募清零。
    let 玩家 = storage::查玩家(storage::执行者::池(&状态.池), 玩家_id)
        .await?
        .ok_or("玩家应存在")?;
    assert_eq!(玩家.生产次数, 8, "补 2 小时后封顶");
    assert_eq!(玩家.今日招募数量, 0, "日结清零");

    // 锚点：只推进整两小时，余下的 5 秒不丢（差值对巡检自己的"现在"不敏感——
    // 锚点从老锚点出发，与巡检取时刻无关）。
    let 进度 = storage::查状态(storage::执行者::池(&状态.池)).await?;
    assert_eq!(
        进度.上次生产结算,
        Some(上次锚点.saturating_add(7200)),
        "锚点推进整数周期，余数保留"
    );
    assert_eq!(
        进度.上次日结,
        Some(昨天零点),
        "日结锚点 = 被结算那天（昨天）的 0 点"
    );

    // 日结：当日增量落一行历史（昨天 0 点），累计两列清零。
    let 当日 = storage::测试辅助::查联军当日(&状态.池, 联军_id).await?;
    assert_eq!(当日, (0, 0), "当天资本增量与扩军累计都清零");
    let 历史 = storage::测试辅助::查联军资本增量行(&状态.池, 联军_id).await?;
    assert_eq!(历史, vec![(昨天零点, 500)], "一行历史，时间戳是昨天的 0 点");

    // ── 幂等：把进度拨回去重跑——不重复加次数（上限兜底）、不写第二行日结。
    storage::测试辅助::发系统状态(
        &状态.池,
        Some(上次锚点),
        Some(昨天零点.saturating_sub(86_400)),
    )
    .await?;
    server::巡检(&状态.池).await?;

    let 玩家 = storage::查玩家(storage::执行者::池(&状态.池), 玩家_id)
        .await?
        .ok_or("玩家应存在")?;
    assert_eq!(玩家.生产次数, 8, "重跑不许再涨");
    let 历史 = storage::测试辅助::查联军资本增量行(&状态.池, 联军_id).await?;
    assert_eq!(历史.len(), 1, "同一天不许写两行历史");

    Ok(())
}
