//! WebSocket 传输层：升级、帧循环、回执发送。
//!
//! 与会话层（`session.rs`）的分工（D48）：这里只管**帧**——读、写、
//! Ping/Pong、关闭；握手状态机与平台一致性门卫在会话层。
//! 连接上不保存游戏状态：每条上行消息自带平台、频道、用户标识，
//! 因此任何一条连接都能处理任何一条消息。

use std::time::Instant;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use domain::平台;
use protocol::{上行消息, 下行消息, 消息段};

use crate::{会话, 传输错误, 分发, 状态, 错误};

/// 处理 WebSocket 升级请求。
pub(crate) async fn ws_处理器(ws: WebSocketUpgrade, State(状态): State<状态>) -> Response {
    ws.on_upgrade(move |套接字| 处理连接(套接字, 状态))
}

/// 一条连接的完整生命周期：握手（会话层）→ 主循环 → 关闭帧。
async fn 处理连接(mut 套接字: WebSocket, 状态: 状态) {
    match 会话::握手(&mut 套接字, &状态).await {
        Ok(平台) => {
            tracing::info!(%平台, "连接就绪");
            if let Err(失败) = 主循环(&mut 套接字, &状态, 平台).await {
                tracing::warn!(%失败, "连接异常结束");
            }
        },
        Err(失败) => {
            tracing::warn!(%失败, "握手失败");
            let 回执 = 错误回执(&失败, None, None, None);
            if let Err(发送问题) = 发送(&mut 套接字, &回执).await {
                tracing::debug!(%发送问题, "发送握手错误失败");
            }
        },
    }

    if let Err(失败) = 套接字.send(Message::Close(None)).await {
        tracing::debug!(%失败, "发送关闭帧失败");
    }
}

/// 读一帧。对端关闭映射成 [`传输错误::连接已关闭`]，底层故障映射成
/// [`传输错误::传输失败`]——握手与主循环共用这一种读法。
pub(crate) async fn 收帧(套接字: &mut WebSocket) -> Result<Message, 错误> {
    let 帧 = 套接字
        .recv()
        .await
        .ok_or(传输错误::连接已关闭)?
        .map_err(|_| 传输错误::传输失败)?;

    Ok(帧)
}

/// 握手之后的消息循环。
///
/// 业务层面的失败（不认识的指令、JSON 坏了）**不会**断连：回一条 `error` 继续跑。
/// 只有传输本身坏了才结束——把一条坏消息的代价放大成"整条连接掉线"没有道理。
async fn 主循环(
    套接字: &mut WebSocket, 状态: &状态, 连接平台: 平台
) -> Result<(), 错误> {
    while let Some(帧) = 套接字.recv().await {
        let 帧 = 帧.map_err(|_| 传输错误::传输失败)?;

        match 帧 {
            Message::Text(原文) => 处理文本(套接字, 状态, 连接平台, &原文).await?,
            Message::Ping(载荷) => {
                套接字
                    .send(Message::Pong(载荷))
                    .await
                    .map_err(|_| 传输错误::发送失败)?;
            },
            Message::Close(_) => return Ok(()),
            Message::Binary(_) | Message::Pong(_) => {},
        }
    }

    Ok(())
}

/// 处理一条已握手的文本消息。
async fn 处理文本(
    套接字: &mut WebSocket,
    状态: &状态,
    连接平台: 平台,
    原文: &str,
) -> Result<(), 错误> {
    let 消息 = match serde_json::from_str::<上行消息>(原文) {
        Ok(消息) => 消息,
        Err(解析错误) => {
            tracing::debug!(%解析错误, "上行消息无法解析");
            let 失败 = 错误::from(传输错误::无法解析);
            return 发送(套接字, &错误回执(&失败, None, None, None)).await;
        },
    };

    match 消息 {
        上行消息::心跳 => 发送(套接字, &下行消息::心跳回应).await,
        上行消息::握手 { .. } => {
            let 失败 = 错误::from(传输错误::重复握手);
            let 回执 = 错误回执(&失败, None, None, None);
            发送(套接字, &回执).await
        },
        上行消息::指令 {
            id,
            平台: 平台原文,
            频道_id,
            用户_uid,
            文本,
            ..
        } => {
            let 开始 = Instant::now();
            let 结果 = 处理一条指令(状态, 连接平台, &平台原文, &频道_id, &用户_uid, &文本).await;

            let 回执 = match 结果 {
                Ok(内容) => {
                    tracing::info!(
                        指令_id = %id,
                        耗时微秒 = 开始.elapsed().as_micros(),
                        "指令完成"
                    );

                    下行消息::回复 {
                        id: id.clone(),
                        平台: 平台原文.clone(),
                        频道_id: 频道_id.clone(),
                        段: vec![消息段::文本 { 内容 }],
                    }
                },
                Err(失败) => {
                    // 级别由错误自己决定：玩家弄错是 info/debug，真故障才是 error。
                    // `tracing::event!` 不接受非 const 的级别，所以这里分三支。
                    match 失败.日志级别() {
                        tracing::Level::ERROR => {
                            tracing::error!(指令_id = %id, 平台 = %平台原文, 失败 = %失败, "指令失败");
                        },
                        tracing::Level::DEBUG => {
                            tracing::debug!(指令_id = %id, 平台 = %平台原文, 失败 = %失败, "指令被拒绝");
                        },
                        tracing::Level::TRACE | tracing::Level::INFO | tracing::Level::WARN => {
                            tracing::info!(指令_id = %id, 平台 = %平台原文, 失败 = %失败, "指令被拒绝");
                        },
                    }

                    错误回执(&失败, Some(id), Some(平台原文), Some(频道_id))
                },
            };

            发送(套接字, &回执).await
        },
    }
}

/// 过会话层的门卫，然后交给分发层。
///
/// 平台与长度的校验在分发之前（会话层）：分发层只认识"已确认的指令"，
/// 不需要再防一次。
async fn 处理一条指令(
    状态: &状态,
    连接平台: 平台,
    平台原文: &str,
    频道_id: &str,
    用户_uid: &str,
    文本: &str,
) -> Result<String, 错误> {
    let 来信 = 会话::校验指令(连接平台, 平台原文, 频道_id, 用户_uid, 文本)?;

    分发::处理(状态, &来信).await
}

/// 把内部错误翻译成一条对外的 `error` 消息。
///
/// `id` / `平台` / `频道_id` 来自触发这条错误的请求；握手阶段没有请求，传 `None`。
fn 错误回执(
    失败: &错误,
    id: Option<String>,
    平台: Option<String>,
    频道_id: Option<String>,
) -> 下行消息 {
    下行消息::错误 {
        id,
        平台,
        频道_id,
        错误码: 失败.错误码().to_owned(),
        文案: 失败.玩家文案(),
    }
}

/// 序列化并发送一条下行消息。会话层回"就绪"也走这里。
pub(crate) async fn 发送(套接字: &mut WebSocket, 消息: &下行消息) -> Result<(), 错误> {
    let 文本 = serde_json::to_string(消息).map_err(|_| 传输错误::无法解析)?;

    套接字
        .send(Message::Text(文本.into()))
        .await
        .map_err(|_| 传输错误::发送失败)?;

    Ok(())
}
