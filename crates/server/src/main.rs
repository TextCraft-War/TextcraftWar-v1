//! `Textcraft: War` 服务器入口：建立 WebSocket 端点，交给 [`连接`] 处理。
//!
//! 配置全部走环境变量，见 `crates/server/docs/design.md`。

// rustc 拒绝非 ASCII 模块**文件名**（error[E0754]），
// 所以文件用英文，模块名保持中文，靠 `#[path]` 转一层。
#[path = "connection.rs"]
mod 连接;

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

/// 全局共享状态。每条连接持有它的克隆。
#[derive(Clone)]
pub(crate) struct 状态 {
    /// 握手用的共享密钥。
    pub(crate) 令牌: Arc<str>,
}

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

    let 状态 = 读取状态()?;
    let 地址 = 读取地址()?;

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

/// 初始化 `tracing`；`RUST_LOG` 缺省时用 `info`。
fn 初始化日志() {
    let 过滤器 = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(过滤器)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// 读取运行时状态。
fn 读取状态() -> Result<状态, 启动错误> {
    let 令牌 = std::env::var("WS_TOKEN").map_err(|_| 启动错误::缺少配置("WS_TOKEN"))?;

    Ok(状态 {
        令牌: Arc::from(令牌),
    })
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

/// 组装路由。目前只有 `/ws` 一个端点。
fn 路由(状态: 状态) -> Router {
    Router::new()
        .route("/ws", get(连接::ws_处理器))
        .with_state(状态)
}
