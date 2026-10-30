//! `注册` 的端到端流程：真数据库、真事务、真 SQL。
//!
//! 需要一个**测试专用**的库（测试会建地区、绑频道、删自己用的账号）：
//!
//! ```text
//! $env:DATABASE_URL="postgres://postgres@127.0.0.1:55400/textcraft_test"
//! cargo test -p server --test register -- --ignored
//! ```
//!
//! 没设 `DATABASE_URL` 时整组测试跳过但**打印警告**；在 `CI=true` 的环境里
//! 缺库直接失败——CI 上静默跳过等于假绿灯（D47）。
//! 播种与断言走 `storage::测试辅助`：本测试**不写一条 SQL**，
//! 表结构变化只改 `storage` 一处。

// 本测试是一个独立的 crate，会看到包的全部依赖；用不到的（`axum`、`rand`…）
// 不该因此被当成"多余依赖"删掉——它们是库侧要的。
#![allow(unused_crate_dependencies, reason = "测试只用到库的一小部分依赖")]

use domain::实体::地区Id;
use domain::平台;
use server::{业务错误, 处理, 来信, 状态, 错误};

/// 测试写死的地区编号：一个测试库只用一个地区，重复跑不会越积越多。
const 测试地区编号: i32 = 990_001;

/// 测试写死的频道 id（会被绑到 `测试地区编号` 那个地区）。
const 已绑定频道: &str = "测试频道-注册";

/// 另一个**没有**绑定地区的频道。
const 未绑定频道: &str = "测试频道-未绑定";

/// 每个用例一个**独立账号**：cargo test 并行跑用例，共用账号会让
/// A 的"注册"撞上 B 刚建好的号（收到"已经注册过"），互相干扰。
const 账号_注册: &str = "测试账号-注册";
const 账号_重复: &str = "测试账号-重复";
const 账号_没注册: &str = "测试账号-没注册";
const 账号_没地区: &str = "测试账号-没地区";
const 账号_未知指令: &str = "测试账号-未知指令";

/// 从文案里挖出 `P123456` 这种编号。
fn 提取玩家编号(文案: &str) -> Option<String> {
    let 起点 = 文案.find('P')?;
    let 其余 = 文案.get(起点..)?;

    let 数字: String = 其余
        .trim_start_matches('P')
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();

    (!数字.is_empty()).then_some(format!("P{数字}"))
}

/// 连库 + 迁移。`DATABASE_URL` 没设时返回 `Ok(None)`（跳过测试但打印警告）；
/// 在 `CI=true` 的环境里返回 `Err`，让用例**失败**而不是假绿（D47）。
#[allow(
    clippy::print_stderr,
    reason = "跳过必须肉眼可见；测试进程里没有初始化 tracing"
)]
async fn 连库() -> Result<Option<状态>, String> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        if std::env::var_os("CI").is_some_and(|值| 值 == "true") {
            return Err("CI 环境必须提供测试库（DATABASE_URL）：静默跳过等于假绿灯".to_owned());
        }
        eprintln!("⚠️ DATABASE_URL 未设置：集成测试被跳过，没有对真库做任何验证");
        return Ok(None);
    };

    let 池 = storage::建立连接池(&url)
        .await
        .map_err(|失败| 失败.to_string())?;
    storage::迁移(&池).await.map_err(|失败| 失败.to_string())?;

    Ok(Some(状态 {
        令牌: "测试".into(),
        池,
    }))
}

/// 播种"地区 + 频道绑定"并清掉指定账号的旧数据，返回地区 id；可重复跑。
async fn 播种(状态: &状态, 账号: &str) -> Result<地区Id, storage::错误> {
    storage::测试辅助::播种注册环境(
        &状态.池,
        平台::Onebot,
        账号,
        测试地区编号,
        "测试地区",
        已绑定频道,
        未绑定频道,
    )
    .await
}

/// 造一条来信。
fn 来信(频道_id: &str, 账号: &str, 文本: &str) -> 来信 {
    来信 {
        平台: 平台::Onebot,
        频道_id: 频道_id.to_owned(),
        用户_uid: 账号.to_owned(),
        文本: 文本.to_owned(),
    }
}

/// 查这个账号绑到的玩家编号（没绑就是 `None`）。
async fn 查编号(状态: &状态, 账号: &str) -> Result<Option<i32>, storage::错误> {
    storage::测试辅助::查绑定编号(&状态.池, 平台::Onebot, 账号).await
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 注册会建玩家与首条绑定() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    let 地区_id = 播种(&状态, 账号_注册).await?;

    let 种子 = 来信(已绑定频道, 账号_注册, "注册");
    let 文案 = 处理(&状态, &种子).await?;

    assert!(文案.contains("注册完成"), "实际：{文案}");
    assert!(文案.contains("测试地区"), "实际：{文案}");

    let 编号 = 查编号(&状态, 账号_注册).await?;
    let 编号 = 编号.ok_or("注册之后应该有一条平台绑定")?;
    assert!(
        文案.contains(&format!("P{编号}")),
        "文案里应当出现 P{编号}，实际：{文案}"
    );

    let 驻扎地区 = storage::测试辅助::查驻扎地区(&状态.池, 编号).await?;
    assert_eq!(驻扎地区, Some(地区_id.0), "出生地区应当是频道所属地区");
    assert_eq!(
        提取玩家编号(&文案).as_deref(),
        Some(format!("P{编号}").as_str())
    );

    Ok(())
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 重复注册是幂等的() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    播种(&状态, 账号_重复).await?;

    let 种子 = 来信(已绑定频道, 账号_重复, "注册");
    let 第一次 = 处理(&状态, &种子).await?;
    let 第二次 = 处理(&状态, &种子).await?;

    assert!(第二次.contains("已经注册过"), "实际：{第二次}");
    // 两次给的编号必须一样：第二次不许再建一个玩家。
    assert_eq!(提取玩家编号(&第一次), 提取玩家编号(&第二次));

    let 条数 = storage::测试辅助::绑定条数(&状态.池, 平台::Onebot, 账号_重复).await?;
    assert_eq!(条数, 1, "同一个账号只能有一条绑定");

    Ok(())
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 未注册的人发别的指令会被提示注册() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    播种(&状态, 账号_没注册).await?;

    let 结果 = 处理(&状态, &来信(已绑定频道, 账号_没注册, "签到")).await;

    assert_eq!(结果.err().map(|失败| 失败.错误码()), Some("not_registered"));

    Ok(())
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 频道没绑地区时不能注册() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    播种(&状态, 账号_没地区).await?;

    let 结果 = 处理(&状态, &来信(未绑定频道, 账号_没地区, "注册")).await;

    assert_eq!(结果.err().map(|失败| 失败.错误码()), Some("no_region"));
    // 失败必须整体回滚：没有玩家、没有绑定。
    assert_eq!(查编号(&状态, 账号_没地区).await?, None);

    Ok(())
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 已注册的人发不认识的指令会收到_unknown_command() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(状态) = 连库().await? else {
        return Ok(());
    };
    播种(&状态, 账号_未知指令).await?;

    处理(&状态, &来信(已绑定频道, 账号_未知指令, "注册")).await?;
    let 结果 = 处理(&状态, &来信(已绑定频道, 账号_未知指令, "随便说说")).await;

    assert_eq!(
        结果.err().map(|失败| 失败.错误码()),
        Some("unknown_command")
    );

    Ok(())
}

/// 错误类型本身要能被当作 `Debug` 打印（日志里要用）。
#[test]
fn 错误可以打印() {
    let 失败 = 错误::业务(业务错误::领域(domain::错误::未注册));
    assert!(!format!("{失败}").is_empty());
}
