//! 集成测试共用的连库入口。
//!
//! 测试需要一个**测试专用**的库（测试会建地区、绑频道、删自己用的账号）：
//!
//! ```text
//! $env:DATABASE_URL="postgres://postgres@127.0.0.1:55400/textcraft_test"
//! cargo test -p server --tests -- --ignored
//! ```
//!
//! 没设 `DATABASE_URL` 时整组测试跳过但**打印警告**；在 `CI=true` 的环境里
//! 缺库直接失败——CI 上静默跳过等于假绿灯（D47）。

/// 连库 + 迁移。`DATABASE_URL` 没设时返回 `Ok(None)`（跳过测试但打印警告）；
/// 在 `CI=true` 的环境里返回 `Err`，让用例**失败**而不是假绿（D47）。
#[allow(
    clippy::print_stderr,
    reason = "跳过必须肉眼可见；测试进程里没有初始化 tracing"
)]
pub(crate) async fn 连库() -> Result<Option<server::状态>, String> {
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

    Ok(Some(server::状态 {
        令牌: "测试".into(),
        池,
    }))
}
