//! 共享消息类型与 `serde` 序列化：面向 Koishi 插件的 JSON 契约。
//!
//! 契约正文见 `docs/arch/protocol.md`。本 crate **不依赖 `domain`**：
//! 内部模型与对外契约各自演化，避免改一个内部字段名就迫使插件侧升级。

use serde::{Deserialize, Serialize};

/// 当前协议主版本号。
///
/// 加字段不改版本；删字段或改语义必须 +1，且服务端要同时支持新旧版本一个发布周期。
pub const 协议主版本: u32 = 1;

/// Koishi -> Rust 的上行消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum 上行消息 {
    /// 握手：连接建立后必须先发这一条。
    #[serde(rename = "hello")]
    握手 {
        /// 平台标识，与「地区频道绑定」表中的枚举一致。
        #[serde(rename = "platform")]
        平台: String,
        /// 握手用的共享密钥。
        #[serde(rename = "token")]
        token: String,
        /// 客户端支持的协议主版本。
        #[serde(rename = "protocolVersion")]
        协议版本: u32,
    },
    /// 玩家发来的一条指令。
    #[serde(rename = "command")]
    指令 {
        /// 由 Koishi 生成，全局唯一，用于关联回复。
        #[serde(rename = "id")]
        id: String,
        /// 平台标识。
        #[serde(rename = "platform")]
        平台: String,
        /// 群 / 频道 id。游戏只在群里玩。
        #[serde(rename = "channelId")]
        频道_id: String,
        /// 平台用户 id。
        #[serde(rename = "userId")]
        用户_uid: String,
        /// 平台昵称快照，可缺省。
        #[serde(rename = "nickname", default, skip_serializing_if = "Option::is_none")]
        昵称: Option<String>,
        /// 消息里被 @ 到的账号，按出现顺序。Rust 侧不解析平台私有的 @ 语法。
        #[serde(rename = "mentions", default, skip_serializing_if = "Vec::is_empty")]
        提及: Vec<提及>,
        /// 玩家原话。命令解析在 Rust 侧完成。
        #[serde(rename = "text")]
        文本: String,
    },
    /// 心跳。Koishi 每 30 秒发一次。
    #[serde(rename = "ping")]
    心跳,
}

/// Rust -> Koishi 的下行消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum 下行消息 {
    /// 握手成功。
    #[serde(rename = "ready")]
    就绪 {
        /// 服务端使用的协议主版本。
        #[serde(rename = "protocolVersion")]
        协议版本: u32,
    },
    /// 对某条指令的回复。
    #[serde(rename = "reply")]
    回复 {
        /// 对应上行消息的 `id`。
        #[serde(rename = "id")]
        id: String,
        /// 平台标识。
        #[serde(rename = "platform")]
        平台: String,
        /// 群 / 频道 id，原样带回，由 Koishi 负责路由。
        #[serde(rename = "channelId")]
        频道_id: String,
        /// 消息段列表。v1 只发文本，但结构上支持图片。
        #[serde(rename = "segments")]
        段: Vec<消息段>,
    },
    /// 出错。
    #[serde(rename = "error")]
    错误 {
        /// 对应的上行 `id`；握手阶段出错时为 `None`。
        #[serde(rename = "id", default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// 平台标识；握手阶段出错时为 `None`。
        #[serde(rename = "platform", default, skip_serializing_if = "Option::is_none")]
        平台: Option<String>,
        /// 群 / 频道 id；握手阶段出错时为 `None`。
        #[serde(rename = "channelId", default, skip_serializing_if = "Option::is_none")]
        频道_id: Option<String>,
        /// 错误码，取值见 `docs/arch/protocol.md`。
        #[serde(rename = "code")]
        错误码: String,
        /// 给玩家看的中文说明，不含调试信息。
        #[serde(rename = "message")]
        文案: String,
    },
    /// 心跳回应。
    #[serde(rename = "pong")]
    心跳回应,
}

/// 一条 @ 提及。
///
/// 由 Koishi 解析：它已经把各平台私有的 @ 语法统一成"平台 + 用户 id"，
/// 服务端不需要认识 `<at id="…"/>` 或 `<@123>` 这类格式。
///
/// 玩家在游戏里**没有名字**——寻址靠平台账号，所以这个结构就是"指向某个玩家"的唯一方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct 提及 {
    /// 平台标识；与所属 `command` 的 `platform` 相同。
    #[serde(rename = "platform")]
    pub 平台: String,
    /// 被 @ 的平台用户 id。
    #[serde(rename = "userId")]
    pub 用户_uid: String,
}

/// 一条消息里的片段。
///
/// 用片段而不是纯字符串，是为了将来能发图片而不改动协议形状。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum 消息段 {
    /// 纯文本。
    #[serde(rename = "text")]
    文本 {
        /// 文本内容。
        #[serde(rename = "text")]
        内容: String,
    },
    /// 图片（v1 不发送，结构先留好）。
    #[serde(rename = "image")]
    图片 {
        /// 图片地址。
        #[serde(rename = "url")]
        url: String,
    },
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 指令的_json_键名必须是英文() {
        let 消息 = 上行消息::指令 {
            id: "c-1".to_owned(),
            平台: "onebot".to_owned(),
            频道_id: "523909412".to_owned(),
            用户_uid: "3071303571".to_owned(),
            昵称: None,
            提及: Vec::new(),
            文本: "签到".to_owned(),
        };

        let json = serde_json::to_string(&消息).unwrap();

        assert!(json.contains(r#""type":"command""#), "实际输出：{json}");
        assert!(
            json.contains(r#""channelId":"523909412""#),
            "实际输出：{json}"
        );
        assert!(
            json.contains(r#""userId":"3071303571""#),
            "实际输出：{json}"
        );
        // 空提及不该出现在 wire 上
        assert!(!json.contains(r#""mentions""#), "实际输出：{json}");
        // 中文标识符绝不能漏进 wire 格式
        assert!(!json.contains("频道_id"), "实际输出：{json}");
        assert!(!json.contains("昵称"), "实际输出：{json}");
    }

    #[test]
    fn 提及被解析成平台加用户_id() {
        let 样例 = r#"{
            "type":"command",
            "id":"c-2",
            "platform":"discord",
            "channelId":"900",
            "userId":"1",
            "mentions":[{"platform":"discord","userId":"42"}],
            "text":"炮击 @某人"
        }"#;

        let 消息: 上行消息 = serde_json::from_str(样例).unwrap();

        let 上行消息::指令 {
            提及: 被提及, 文本,
        ..
        } = 消息
        else {
            panic!("应当是 command");
        };
        assert_eq!(文本, "炮击 @某人");
        assert_eq!(
            被提及,
            vec![提及 {
                平台: "discord".to_owned(),
                用户_uid: "42".to_owned(),
            }]
        );
    }

    #[test]
    fn 握手能解析协议文档里的样例() {
        let 样例 = r#"{"type":"hello","platform":"onebot","token":"t","protocolVersion":1}"#;

        let 消息: 上行消息 = serde_json::from_str(样例).unwrap();

        assert_eq!(
            消息,
            上行消息::握手 {
                平台: "onebot".to_owned(),
                token: "t".to_owned(),
                协议版本: 1,
            }
        );
    }

    #[test]
    fn 回复的消息段形状正确() {
        let 消息 = 下行消息::回复 {
            id: "c-1".to_owned(),
            平台: "onebot".to_owned(),
            频道_id: "523909412".to_owned(),
            段: vec![消息段::文本 {
                内容: "签到成功".to_owned(),
            }],
        };

        let json = serde_json::to_string(&消息).unwrap();

        assert!(json.contains(r#""type":"reply""#), "实际输出：{json}");
        assert!(
            json.contains(r#""segments":[{"type":"text","text":""#),
            "实际输出：{json}"
        );
    }

    #[test]
    fn 心跳省略可选字段() {
        let 消息 = 下行消息::错误 {
            id: None,
            平台: None,
            频道_id: None,
            错误码: "bad_request".to_owned(),
            文案: "格式错误".to_owned(),
        };

        let json = serde_json::to_string(&消息).unwrap();

        assert!(!json.contains(r#""id""#), "实际输出：{json}");
        assert!(!json.contains(r#""channelId""#), "实际输出：{json}");
        assert!(json.contains(r#""code":"bad_request""#), "实际输出：{json}");
    }
}
