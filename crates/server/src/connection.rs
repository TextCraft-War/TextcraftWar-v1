//! 单条 WebSocket 连接的生命周期：握手 -> 主循环。
//!
//! 连接上**不保存游戏状态**：每条上行消息自带平台、频道、用户标识，
//! 因此任何一条连接都能处理任何一条消息。

use std::time::Duration;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use protocol::{上行消息, 下行消息, 协议主版本};

use crate::状态;

/// 客户端必须在这个时间内发来握手消息。
const 握手超时: Duration = Duration::from_secs(5);

/// 握手或传输过程中的失败。
#[derive(Debug, thiserror::Error)]
enum 连接错误 {
    /// 超时没有收到任何消息。
    #[error("握手超时")]
    超时,
    /// 连接在握手前被关闭。
    #[error("连接在握手前关闭")]
    连接已关闭,
    /// 底层传输失败。
    #[error("传输失败")]
    传输失败,
    /// 帧不是文本。
    #[error("协议只接受文本帧")]
    非文本帧,
    /// JSON 无法解析。
    #[error("消息格式错误")]
    无法解析,
    /// 第一条消息不是握手。
    #[error("第一条消息必须是 hello")]
    不是握手消息,
    /// 密钥不匹配。
    #[error("令牌错误")]
    令牌错误,
    /// 协议版本不匹配。
    #[error("协议版本不匹配，服务端为 {服务端}")]
    版本不匹配 {
        /// 服务端支持的协议主版本。
        服务端: u32,
    },
    /// 发送失败。
    #[error("发送失败")]
    发送失败,
}

impl 连接错误 {
    /// 对外暴露的错误码，取值见 `docs/protocol.md`。
    const fn 错误码(&self) -> &'static str {
        match self {
            Self::令牌错误 => "forbidden",
            Self::超时
            | Self::连接已关闭
            | Self::传输失败
            | Self::非文本帧
            | Self::无法解析
            | Self::不是握手消息
            | Self::版本不匹配 { .. }
            | Self::发送失败 => "bad_request",
        }
    }
}

/// 处理 WebSocket 升级请求。
//
// 本 crate 是二进制，两条 lint 在此互斥：`pub` 触发 `unreachable_pub`（没有外部消费者），
// `pub(crate)` 在私有模块里触发 `redundant_pub_crate`。取后者语义更准（只给 main.rs 用），
// 因此豁免 `redundant_pub_crate`。
#[allow(clippy::redundant_pub_crate)]
pub(crate) async fn ws_处理器(ws: WebSocketUpgrade, State(状态): State<状态>) -> Response {
    ws.on_upgrade(move |套接字| 处理连接(套接字, 状态))
}

/// 一条连接的完整生命周期。
async fn 处理连接(mut 套接字: WebSocket, 状态: 状态) {
    match 握手(&mut 套接字, &状态).await {
        Ok(平台) => {
            tracing::info!(%平台, "连接就绪");
            if let Err(错误) = 主循环(&mut 套接字).await {
                tracing::warn!(%错误, "连接异常结束");
            }
        },
        Err(错误) => {
            tracing::warn!(%错误, "握手失败");
            let 回执 = 下行消息::错误 {
                id: None,
                平台: None,
                频道_id: None,
                错误码: 错误.错误码().to_owned(),
                文案: 错误.to_string(),
            };
            if let Err(发送错误) = 发送(&mut 套接字, &回执).await {
                tracing::debug!(%发送错误, "发送握手错误失败");
            }
        },
    }

    if let Err(错误) = 套接字.send(Message::Close(None)).await {
        tracing::debug!(%错误, "发送关闭帧失败");
    }
}

/// 读取并校验握手消息，成功时返回平台标识。
async fn 握手(套接字: &mut WebSocket, 状态: &状态) -> Result<String, 连接错误> {
    let 帧 = tokio::time::timeout(握手超时, 套接字.recv())
        .await
        .map_err(|_| 连接错误::超时)?
        .ok_or(连接错误::连接已关闭)?
        .map_err(|_| 连接错误::传输失败)?;

    let Message::Text(原文) = 帧 else {
        return Err(连接错误::非文本帧);
    };

    let 消息 = serde_json::from_str::<上行消息>(&原文).map_err(|_| 连接错误::无法解析)?;

    let 上行消息::握手 {
        平台,
        token,
        协议版本: 客户端版本,
    } = 消息
    else {
        return Err(连接错误::不是握手消息);
    };

    if token != 状态.令牌.as_ref() {
        return Err(连接错误::令牌错误);
    }

    if 客户端版本 != 协议主版本 {
        return Err(连接错误::版本不匹配 {
            服务端: 协议主版本
        });
    }

    发送(
        套接字,
        &下行消息::就绪 {
            协议版本: 协议主版本,
        },
    )
    .await?;

    Ok(平台)
}

/// 握手之后的消息循环。
async fn 主循环(套接字: &mut WebSocket) -> Result<(), 连接错误> {
    while let Some(帧) = 套接字.recv().await {
        let 帧 = 帧.map_err(|_| 连接错误::传输失败)?;

        match 帧 {
            Message::Text(原文) => 处理文本(套接字, &原文).await?,
            Message::Ping(载荷) => {
                套接字
                    .send(Message::Pong(载荷))
                    .await
                    .map_err(|_| 连接错误::发送失败)?;
            },
            Message::Close(_) => return Ok(()),
            Message::Binary(_) | Message::Pong(_) => {},
        }
    }

    Ok(())
}

/// 处理一条已握手的文本消息。
async fn 处理文本(套接字: &mut WebSocket, 原文: &str) -> Result<(), 连接错误> {
    let 消息 = match serde_json::from_str::<上行消息>(原文) {
        Ok(消息) => 消息,
        Err(错误) => {
            tracing::debug!(%错误, "上行消息无法解析");
            return 发送(
                套接字,
                &下行消息::错误 {
                    id: None,
                    平台: None,
                    频道_id: None,
                    错误码: "bad_request".to_owned(),
                    文案: "消息格式错误".to_owned(),
                },
            )
            .await;
        },
    };

    match 消息 {
        上行消息::心跳 => 发送(套接字, &下行消息::心跳回应).await,
        上行消息::指令 {
            id, 平台, 频道_id,
        ..
        } => {
            // S0 只打通链路，还没有任何指令实现。
            tracing::info!(指令_id = %id, %平台, %频道_id, "收到指令（尚未实现）");
            发送(
                套接字,
                &下行消息::错误 {
                    id: Some(id),
                    平台: Some(平台),
                    频道_id: Some(频道_id),
                    错误码: "unknown_command".to_owned(),
                    文案: "这条指令还没有实现".to_owned(),
                },
            )
            .await
        },
        上行消息::握手 { .. } => {
            发送(
                套接字,
                &下行消息::错误 {
                    id: None,
                    平台: None,
                    频道_id: None,
                    错误码: "bad_request".to_owned(),
                    文案: "已经握过手了".to_owned(),
                },
            )
            .await
        },
    }
}

/// 序列化并发送一条下行消息。
async fn 发送(套接字: &mut WebSocket, 消息: &下行消息) -> Result<(), 连接错误> {
    let 文本 = serde_json::to_string(消息).map_err(|_| 连接错误::无法解析)?;

    套接字
        .send(Message::Text(文本.into()))
        .await
        .map_err(|_| 连接错误::发送失败)
}
