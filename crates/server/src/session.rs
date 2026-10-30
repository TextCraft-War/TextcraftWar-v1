//! 会话层：握手状态机与每条消息的平台一致性门卫。
//!
//! 一条连接 = 一个平台（`docs/arch/protocol.md`）：连接上只记一样东西——
//! **握手时的平台**，每条指令消息的平台必须与它一致，否则 `bad_request`。
//! 状态机只有两种结局：握手成功的平台、或一条 [`传输错误`]——
//! 输入输出都在这层说清，帧的搬运归传输层（`ws.rs`，D48）。

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use domain::平台;
use protocol::{上行消息, 下行消息, 协议主版本};

use crate::{传输错误, 分发, 状态, 连接, 错误};

/// 客户端必须在这个时间内发来握手消息。
const 握手超时: Duration = Duration::from_secs(5);

/// 单条指令的长度上限（字符数，不是字节数）。
///
/// 协议承诺过的上限：防的是"有人往日志/数据库里灌数据"，
/// 不是为了限制玩法。超长直接拒绝，不截断——截断会让玩家以为自己发对了。
const 指令长度上限: usize = 512;

/// 读取并校验握手消息，成功时返回这条连接所属的平台。
pub(crate) async fn 握手(套接字: &mut WebSocket, 状态: &状态) -> Result<平台, 错误> {
    let 帧 = tokio::time::timeout(握手超时, 连接::收帧(套接字))
        .await
        .map_err(|_| 传输错误::握手超时)??;

    let Message::Text(原文) = 帧 else {
        return Err(传输错误::非文本帧.into());
    };

    let 消息 = serde_json::from_str::<上行消息>(&原文).map_err(|_| 传输错误::无法解析)?;

    let 上行消息::握手 {
        平台: 平台原文,
        token,
        协议版本: 客户端版本,
    } = 消息
    else {
        return Err(传输错误::不是握手消息.into());
    };

    if token != 状态.令牌.as_ref() {
        return Err(传输错误::令牌错误.into());
    }

    if 客户端版本 != 协议主版本 {
        return Err(传输错误::版本不匹配 {
            服务端: 协议主版本
        }
        .into());
    }

    let 平台 = 平台::解析(&平台原文).ok_or(传输错误::未知平台 {
        原文: 平台原文
    })?;

    连接::发送(
        套接字,
        &下行消息::就绪 {
            协议版本: 协议主版本,
        },
    )
    .await?;

    Ok(平台)
}

/// 每条指令消息的**门卫**：长度上限 → 平台编码合法 → 平台一致，
/// 全过才包装成 `分发::来信` 交给分发层。
///
/// 校验放在分发之前：分发层只认识"已确认的平台"，不需要再防一次。
pub(crate) fn 校验指令(
    连接平台: 平台,
    平台原文: &str,
    频道_id: &str,
    用户_uid: &str,
    文本: &str,
) -> Result<分发::来信, 错误> {
    let 长度 = 文本.chars().count();
    if 长度 > 指令长度上限 {
        return Err(传输错误::文本过长 { 长度 }.into());
    }

    let 平台 = 平台::解析(平台原文).ok_or_else(|| 传输错误::未知平台 {
        原文: 平台原文.to_owned(),
    })?;

    if 平台 != 连接平台 {
        return Err(传输错误::平台不一致 {
            连接: 连接平台.编码().to_owned(),
            消息: 平台原文.to_owned(),
        }
        .into());
    }

    Ok(分发::来信 {
        平台,
        频道_id: 频道_id.to_owned(),
        用户_uid: 用户_uid.to_owned(),
        文本: 文本.to_owned(),
    })
}

#[cfg(test)]
mod 测试 {
    use super::{指令长度上限, 校验指令};
    use crate::{传输错误, 错误};
    use domain::平台;

    /// 门卫放行的消息要原样进 `来信`——校验是检查，不是改写。
    #[test]
    fn 平台一致时放行并保留原文() {
        let 来信 = 校验指令(平台::Onebot, "onebot", "群-1", "账号-1", "注册").expect("应当放行");

        assert_eq!(来信.平台, 平台::Onebot);
        assert_eq!(来信.频道_id, "群-1");
        assert_eq!(来信.用户_uid, "账号-1");
        assert_eq!(来信.文本, "注册");
    }

    /// 一条连接 = 一个平台（protocol.md）：冒充别的平台必须在门卫就被拦下。
    #[test]
    fn 平台不一致会被拦下() {
        let 失败 =
            校验指令(平台::Onebot, "discord", "群-1", "账号-1", "注册").expect_err("应当拦下");

        assert_eq!(失败.错误码(), "bad_request");
        assert!(matches!(失败, 错误::传输(传输错误::平台不一致 { .. })));
    }

    /// 超长的文本按字符数计（不是字节数），超了就拒，不截断。
    #[test]
    fn 超长文本会被拦下() {
        let 文本 = "字".repeat(指令长度上限 + 1);
        let 失败 = 校验指令(平台::Onebot, "onebot", "群-1", "账号-1", &文本).expect_err("应当拦下");

        assert_eq!(失败.错误码(), "bad_request");
        assert!(matches!(失败, 错误::传输(传输错误::文本过长 { .. })));
    }

    /// 恰好在上限内的文本放行（边界不差一）。
    #[test]
    fn 上限之内的文本放行() {
        let 文本 = "字".repeat(指令长度上限);
        let 来信 = 校验指令(平台::Onebot, "onebot", "群-1", "账号-1", &文本).expect("应当放行");

        assert_eq!(来信.文本.len(), 指令长度上限 * 3, "UTF-8 每字三字节");
    }
}
