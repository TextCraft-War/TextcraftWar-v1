//! `注册` 的端到端流程：真数据库、真事务、真 SQL。
//!
//! 需要一个**测试专用**的库（测试会建地区、绑频道、删自己用的账号）：
//!
//! ```text
//! $env:DATABASE_URL="postgres://postgres@127.0.0.1:55400/textcraft_test"
//! cargo test -p server --test register -- --ignored
//! ```
//!
//! 没设 `DATABASE_URL` 时整组测试直接跳过——`cargo test` 在没装 `PostgreSQL`
//! 的机器上（新克隆、CI）也必须能过，不能因为缺库就红。

// 本测试是一个独立的 crate，会看到包的全部依赖；用不到的（`axum`、`rand`…）
// 不该因此被当成"多余依赖"删掉——它们是库侧要的。
#![allow(unused_crate_dependencies, reason = "测试只用到库的一小部分依赖")]

use domain::平台;
use server::{处理, 来信, 状态, 错误};

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

/// 连库 + 迁移；`DATABASE_URL` 没设时返回 `None`（跳过测试）。
async fn 连库() -> Option<状态> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let 池 = storage::建立连接池(&url).await.ok()?;
    storage::迁移(&池).await.ok()?;

    Some(状态 {
        令牌: "测试".into(),
        池,
    })
}

/// 播种"地区 + 频道绑定"，并清掉指定测试账号的旧数据，让测试可重复跑。
async fn 播种(状态: &状态, 账号: &str) -> Result<i64, sqlx::Error> {
    sqlx::query(
        "DELETE FROM 玩家
          WHERE id IN (SELECT 玩家_id FROM 平台绑定 WHERE 平台 = 'onebot' AND 平台_uid = $1)",
    )
    .bind(账号)
    .execute(&状态.池)
    .await?;

    sqlx::query("DELETE FROM 平台绑定 WHERE 平台 = 'onebot' AND 平台_uid = $1")
        .bind(账号)
        .execute(&状态.池)
        .await?;

    sqlx::query("DELETE FROM 地区频道绑定 WHERE 平台 = 'onebot' AND 频道_id = $1")
        .bind(未绑定频道)
        .execute(&状态.池)
        .await?;

    let 地区_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO 地区 (uid, 名称, 创建时间) VALUES ($1, '测试地区', $2)
         ON CONFLICT (uid) DO UPDATE SET 名称 = EXCLUDED.名称
         RETURNING id",
    )
    .bind(测试地区编号)
    .bind(现在())
    .fetch_one(&状态.池)
    .await?;

    sqlx::query(
        "INSERT INTO 地区频道绑定 (地区_id, 平台, 频道_id, 绑定时间) VALUES ($1, 'onebot', $2, $3)
         ON CONFLICT (平台, 频道_id) DO UPDATE SET 地区_id = EXCLUDED.地区_id",
    )
    .bind(地区_id)
    .bind(已绑定频道)
    .bind(现在())
    .execute(&状态.池)
    .await?;

    Ok(地区_id)
}

/// 当前 Unix 秒。
fn 现在() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |时长| i64::try_from(时长.as_secs()).unwrap_or(i64::MAX))
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
async fn 查编号(状态: &状态, 账号: &str) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        "SELECT 玩家.uid
           FROM 平台绑定
           JOIN 玩家 ON 玩家.id = 平台绑定.玩家_id
          WHERE 平台绑定.平台 = 'onebot' AND 平台绑定.平台_uid = $1",
    )
    .bind(账号)
    .fetch_optional(&状态.池)
    .await
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 注册会建玩家与首条绑定() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await else {
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

    let 驻扎地区: Option<i64> = sqlx::query_scalar("SELECT 驻扎地区_id FROM 玩家 WHERE uid = $1")
        .bind(编号)
        .fetch_one(&状态.池)
        .await?;
    assert_eq!(驻扎地区, Some(地区_id), "出生地区应当是频道所属地区");
    assert_eq!(
        提取玩家编号(&文案).as_deref(),
        Some(format!("P{编号}").as_str())
    );

    Ok(())
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 重复注册是幂等的() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await else {
        return Ok(());
    };
    播种(&状态, 账号_重复).await?;

    let 种子 = 来信(已绑定频道, 账号_重复, "注册");
    let 第一次 = 处理(&状态, &种子).await?;
    let 第二次 = 处理(&状态, &种子).await?;

    assert!(第二次.contains("已经注册过"), "实际：{第二次}");
    // 两次给的编号必须一样：第二次不许再建一个玩家。
    assert_eq!(提取玩家编号(&第一次), 提取玩家编号(&第二次));

    let 玩家数: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
           FROM 平台绑定
          WHERE 平台 = 'onebot' AND 平台_uid = $1",
    )
    .bind(账号_重复)
    .fetch_one(&状态.池)
    .await?;
    assert_eq!(玩家数, 1, "同一个账号只能有一条绑定");

    Ok(())
}

#[tokio::test]
#[ignore = "需要 DATABASE_URL 指向一个测试库"]
async fn 未注册的人发别的指令会被提示注册() -> Result<(), Box<dyn std::error::Error>> {
    let Some(状态) = 连库().await else {
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
    let Some(状态) = 连库().await else {
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
    let Some(状态) = 连库().await else {
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
    let 失败 = 错误::未注册;
    assert!(!format!("{失败}").is_empty());
}
