//! `Textcraft: War` 服务器的库：状态、路由、指令分发。
//!
//! 二进制入口只有 `main.rs` 那一层（读配置、建池、跑迁移、起调度器、监听）。
//! 把连接处理与分发放进库，是为了让集成测试能直接调用 [`处理`]，
//! 不必绕一圈 WebSocket。
//!
//! # 依赖方向（由编译器强制）
//!
//! ```text
//! server -> { domain, storage, protocol }
//! ```
//!
//! 本 crate **不依赖 `sqlx`**：事务的类型与动作都在 `storage` 后面，
//! 想在这里写一条 SQL 会直接编译失败（[D40]）。
//! 错误分三层（[D48]）：传输、业务（含领域拒绝）、存储；解析产出
//! `domain::指令`（[D44]），认不出的原文留在解析层。
//!
//! # 模块图
//!
//! ```text
//! 连接(ws.rs 帧循环) ── 会话(session.rs 握手与门卫) ── 分发(dispatch.rs 路由)
//!                                                            └─ 处理器(handlers/ 业务流程)
//! 调度(scheduler.rs) 由 main.rs 在迁移之后拉起，与监听并行。
//! ```
//!
//! [D40]: ../../../docs/decisions.md
//! [D44]: ../../../docs/decisions.md
//! [D48]: ../../../docs/decisions.md

// 本 crate 的实现模块全是私有的，于是 `pub(crate)` 在 clippy 眼里是"冗余"的
// （它会建议改成 `pub`；但那样又撞上 `unreachable_pub`——两边互斥）。
// 保留 `pub(crate)`：让"只在 crate 内可见"写在签名上，模块私有只是顺带的边界。
#![allow(clippy::redundant_pub_crate)]

// rustc 拒绝非 ASCII 模块**文件名**（error[E0754]），
// 所以文件用英文，模块名保持中文，靠 `#[path]` 转一层。
//
// `错误类型` 这个模块名看着别扭，是因为**模块与类型在同一个命名空间**：
// 模块若也叫 `错误`，把它里面的 `错误` 提到根上就会 E0255。
#[path = "session.rs"]
mod 会话;
#[path = "dispatch.rs"]
mod 分发;
#[path = "handlers/mod.rs"]
mod 处理器;
#[path = "text.rs"]
mod 文案;
#[path = "clock.rs"]
mod 时钟;
#[path = "parser.rs"]
mod 解析;
#[path = "scheduler.rs"]
mod 调度;
#[path = "ws.rs"]
mod 连接;
#[path = "error.rs"]
mod 错误类型;

use std::sync::Arc;

use axum::{Router, routing::get};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

pub use 分发::{处理, 来信};
pub use 调度::启动调度器;
pub use 错误类型::{业务错误, 传输错误, 错误};

/// 全局共享状态。每条连接持有它的克隆。
///
/// 这里**只放进程级依赖**（密钥、连接池）。玩家状态一律现查现用，
/// 不缓存（v1 的取舍，见 AGENTS.md）——所以"任何一条连接都能处理任何一条消息"
/// 这句话在数据层面才真的成立。
#[derive(Clone)]
pub struct 状态 {
    /// 握手用的共享密钥。
    pub 令牌: Arc<str>,
    /// 数据库连接池。事务边界由本层掌握，所以池放在这里。
    pub 池: storage::连接池,
}

/// 组装 HTTP 路由。目前只有 `/ws` 一个端点。
pub fn 路由(状态: 状态) -> Router {
    Router::new()
        .route("/ws", get(连接::ws_处理器))
        .with_state(状态)
}

/// 初始化 `tracing`；`RUST_LOG` 缺省时用 `info`。
///
/// 放在库里而不是 `main.rs`：日志配置与进程其余部分无关，
/// 但要保证"二进制入口只做启动"，且只能初始化一次。
pub fn 初始化日志() {
    let 过滤器 = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(过滤器)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
