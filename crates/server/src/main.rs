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
    /// 数据库连接池。事务边界由 `server` 掌握，所以池放在这里。
    ///
    /// S2 起 `command` 要用它查频道与玩家，届时这条 `expect` 会失效并提醒删掉。
    #[expect(dead_code, reason = "S2 的 command 路由才会读它")]
    pub(crate) 池: storage::连接池,
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

/// 初始化 `tracing`；`RUST_LOG` 缺省时用 `info`。
fn 初始化日志() {
    let 过滤器 = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(过滤器)
        .with(tracing_subscriber::fmt::layer())
        .init();
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

/// 组装路由。目前只有 `/ws` 一个端点。
fn 路由(状态: 状态) -> Router {
    Router::new()
        .route("/ws", get(连接::ws_处理器))
        .with_state(状态)
}
