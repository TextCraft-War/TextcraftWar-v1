//! `Textcraft: War` 服务器入口：读配置、建池、跑迁移、监听。
//!
//! 真正的逻辑在库侧；本文件只负责"把进程正确地点起来"，
//! 以及"点不起来时说清楚为什么"。

// 依赖是按**包**声明的，而 `unused_crate_dependencies` 是按**target** 检查的：
// `domain` / `protocol` / `rand` / `serde_json` 都由库侧使用，这个 100 行的入口用不到，
// 但把它们从包的依赖里删掉，库就编译不过。
#![allow(unused_crate_dependencies, reason = "依赖由库侧使用，入口只是启动代码")]

use std::{net::SocketAddr, sync::Arc};

use server::{初始化日志, 状态, 路由};
use tokio::net::TcpListener;

/// 启动期错误。
#[derive(Debug, thiserror::Error)]
enum 启动错误 {
    /// 缺少必需的环境变量。
    #[error("缺少环境变量 {0}")]
    缺少配置(&'static str),
    /// 环境变量的值无法解析。
    #[error("环境变量 {键} 的值无法解析：{值}")]
    配置格式错误 {
        /// 变量名。
        键: &'static str,
        /// 原始值。
        值: String,
    },
    /// 数据库连接或迁移失败。
    #[error(transparent)]
    数据库(#[from] storage::错误),
    /// 监听端口失败。
    #[error("监听 {地址} 失败：{来源}")]
    监听失败 {
        /// 监听地址。
        地址: String,
        /// 底层错误。
        来源: std::io::Error,
    },
}

#[tokio::main]
async fn main() -> Result<(), 启动错误> {
    初始化日志();

    let 令牌 = 读取配置("WS_TOKEN")?;
    let 数据库_url = 读取配置("DATABASE_URL")?;
    let 地址 = 读取地址()?;

    // 迁移在绑定端口之前跑：库连不上就不该对外提供服务。
    let 池 = storage::建立连接池(&数据库_url).await?;
    storage::迁移(&池).await?;
    tracing::info!("数据库已连接，迁移已执行");

    let 状态 = 状态 {
        令牌: Arc::from(令牌),
        池,
    };

    let 监听器 = TcpListener::bind(地址)
        .await
        .map_err(|来源| 启动错误::监听失败 {
            地址: 地址.to_string(),
            来源,
        })?;

    tracing::info!(%地址, "Textcraft: War 服务器启动");

    axum::serve(监听器, 路由(状态))
        .await
        .map_err(|来源| 启动错误::监听失败 {
            地址: 地址.to_string(),
            来源,
        })
}

/// 读取必需的环境变量。
fn 读取配置(键: &'static str) -> Result<String, 启动错误> {
    std::env::var(键).map_err(|_| 启动错误::缺少配置(键))
}

/// 读取监听地址，缺省 `0.0.0.0:8080`。
fn 读取地址() -> Result<SocketAddr, 启动错误> {
    let 原文 = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());

    let 地址 = 原文
        .parse::<SocketAddr>()
        .map_err(|_| 启动错误::配置格式错误 {
            键: "BIND_ADDR",
            值: 原文.clone(),
        })?;

    Ok(地址)
}
