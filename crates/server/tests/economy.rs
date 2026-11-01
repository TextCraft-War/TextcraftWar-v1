//! `签到`（S4）的端到端流程：真数据库、真事务、真冷却判定。
//!
//! 覆盖两件事（impl.md 4.1/4.2）：
//!
//! 1. 签到发奖：文案报千分位所得、库存落账、锚点写入；
//! 2. 冷却中再签被 `bad_request` 拒绝，库存分毫不动；冷却到点能再签。
//!
//! 冷却场景靠 `测试辅助::改玩家签到` 拨锚点构造，不用真等一天。
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
use domain::实体::{物品, 资源};
use domain::平台;
use server::{处理, 来信, 状态};

/// 独立账号 / 地区 / 频道：与本目录其他测试互不共享（并行安全）。
const 测试地区编号: i32 = 990_003;
const 已绑定频道: &str = "测试频道-签到";
const 未绑定频道: &str = "测试频道-签到-未绑";

const 账号_签到: &str = "测试账号-签到";
const 账号_冷却: &str = "测试账号-签到冷却";

/// 当前 Unix 秒。拨锚点用；与处理内部取的时刻最多差几秒，
/// 断言都留了余量（见各用例注释）。
fn 当前秒() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |时长| i64::try_from(时长.as_secs()).unwrap_or(i64::MAX))
}

/// 播种"地区 + 频道绑定"并清掉指定账号的旧数据；可重复跑。
async fn 播种(状态: &状态, 账号: &str) -> Result<(), storage::错误> {
    storage::测试辅助::播种注册环境(
        &状态.池,
        平台::Onebot,
        账号,
        测试地区编号,
        "签到地区",
        已绑定频道,
        未绑定频道,
    )
    .await
    .map(|_| ())
}

/// 造一条来信（零参指令：`args` 缺省，D51）。
fn 来信(频道_id: &str, 账号: &str, 指令名: &str) -> 来信 {
    来信 {
        平台: 平台::Onebot,
        频道_id: 频道_id.to_owned(),
        用户_uid: 账号.to_owned(),
        指令名: 指令名.to_owned(),
        参数: serde_json::Value::Null,
    }
}

/// 注册一个账号并返回其玩家 id。
async fn 注册并取_id(
    状态: &状态,
    账号: &str,
) -> Result<domain::实体::玩家Id, Box<dyn std::error::Error>> {
    处理(状态, &来信(已绑定频道, 账号, "注册")).await?;

    storage::测试辅助::查玩家_id(&状态.池, 平台::Onebot, 账号)
        .await?
        .ok_or_else(|| storage::错误::数据损坏 {
            表: "平台绑定",
            列: "玩家_id",
        })
        .map_err(Into::into)
}

/// 读某玩家背包里的生活资料数量。
async fn 查生活资料(
    状态: &状态,
    玩家_id: domain::实体::玩家Id,
) -> Result<i64, storage::错误> {
    let 库存 = storage::查库存(
        storage::执行者::池(&状态.池),
        storage::库存所有者::玩家(玩家_id),
    )
    .await?;

    Ok(库存.取(物品::资源(资源::生活资料)))
}

/// 签到发奖：文案报所得、库存落账、再签被拒（一个函数内顺序执行，
/// 共用同一账号的库存行）。
#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 签到发奖_库存落账_再签被冷却拒绝() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    播种(&状态, 账号_签到).await?;
    let 玩家_id = 注册并取_id(&状态, 账号_签到).await?;

    // ── 第一次签到：从没签过，直接给奖。
    let 文案 = 处理(&状态, &来信(已绑定频道, 账号_签到, "签到")).await?;

    assert!(文案.contains("====[签到成功]===="), "实际：{文案}");
    assert!(
        文案.contains(&format!("■ 生活资料 +{}", domain::实体::玩家::签到奖励)),
        "奖励要按千分位展示，实际：{文案}"
    );
    assert_eq!(
        查生活资料(&状态, 玩家_id).await?,
        domain::实体::玩家::签到奖励,
        "奖励落账"
    );

    let 玩家 = storage::查玩家(storage::执行者::池(&状态.池), 玩家_id)
        .await?
        .ok_or("玩家应存在")?;
    assert!(玩家.上次签到.is_some(), "锚点应当写入");

    // ── 立刻再签：冷却远没走完（几秒 << 一天），拒绝且不动账。
    let 再签 = 处理(&状态, &来信(已绑定频道, 账号_签到, "签到")).await;

    assert_eq!(
        再签.err().map(|失败| 失败.错误码()),
        Some("bad_request"),
        "冷却中应回 bad_request（D49.3）"
    );
    assert_eq!(
        查生活资料(&状态, 玩家_id).await?,
        domain::实体::玩家::签到奖励,
        "被拒的签到不得改账"
    );

    Ok(())
}

/// 冷却判定跟着锚点走：一小时前签过 → 拒；一天多以前签过 → 放行。
#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 冷却解除后可以再签() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    播种(&状态, 账号_冷却).await?;
    let 玩家_id = 注册并取_id(&状态, 账号_冷却).await?;

    // 一小时前签过：拒绝（余量 3600，远小于一天的窗口，秒级时钟差不影响）。
    storage::测试辅助::改玩家签到(&状态.池, 玩家_id, Some(当前秒() - 3_600)).await?;
    let 结果 = 处理(&状态, &来信(已绑定频道, 账号_冷却, "签到")).await;
    assert_eq!(结果.err().map(|失败| 失败.错误码()), Some("bad_request"));

    // 一天多以前签过：放行（余量 10 秒之外全是安全区）。
    storage::测试辅助::改玩家签到(
        &状态.池,
        玩家_id,
        Some(当前秒() - domain::实体::玩家::签到冷却秒 - 10),
    )
    .await?;
    let 文案 = 处理(&状态, &来信(已绑定频道, 账号_冷却, "签到")).await?;
    assert!(文案.contains("签到成功"), "实际：{文案}");
    assert_eq!(
        查生活资料(&状态, 玩家_id).await?,
        domain::实体::玩家::签到奖励
    );

    Ok(())
}
