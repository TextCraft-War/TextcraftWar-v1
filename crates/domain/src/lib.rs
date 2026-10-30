//! 纯游戏规则层：签到、工业生产、战争结算的**纯粹计算**。
//!
//! # 硬约束
//!
//! 本 crate **零外部依赖、零 IO、零 async**。所有规则写成
//! `fn 结算(旧状态, 指令) -> (新状态, 事件)` 形式的纯函数，
//! 因此只靠 `cargo test` 就能验证，不需要 PostgreSQL，也不需要 WebSocket。
//!
//! # 依赖方向（由编译器强制，不是靠自觉）
//!
//! ```text
//! server -> { domain, storage, protocol }
//! storage -> domain
//! protocol -> serde（不含 domain）
//! domain -> 无
//! ```
//!
//! 这条分层带来的直接好处：战争结算可以「纯函数批量算完，再一次性写库」，
//! 而不是「边算边锁行」——性能红线从架构上就被满足了，不靠自律。

use std::fmt;

// rustc 拒绝非 ASCII 模块**文件名**（error[E0754]），
// 所以文件用英文，模块名保持中文，靠 `#[path]` 转一层。
#[path = "uid.rs"]
pub mod 编号;

/// 接入的聊天平台。
///
/// 平台是**身份模型**的一部分（一个指挥官可以绑多个平台账号），
/// 所以它必须同时被 `storage`（落库/校验）和 `server`（解析 wire 上的字符串）认识。
/// 放在 `domain` 里，两个方向读同一份定义。
///
/// 落库是 `TEXT`（`平台绑定.平台` / `地区频道绑定.平台`），取值见 [`平台::编码`]。
/// 未知编码一律当错误处理，**不要** fallback 成某个默认平台——那会把
/// "插件写错了平台名"变成"凭空多出一个平台命名空间"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum 平台 {
    /// QQ（走 `OneBot` / Satori 适配器）。
    Onebot,
    /// Discord。
    Discord,
    /// Telegram。
    Telegram,
}

impl 平台 {
    /// 落库与 wire 上使用的编码。
    pub const fn 编码(self) -> &'static str {
        match self {
            Self::Onebot => "onebot",
            Self::Discord => "discord",
            Self::Telegram => "telegram",
        }
    }

    /// 从编码解析；未知编码返回 `None`。
    pub fn 解析(原文: &str) -> Option<Self> {
        match 原文 {
            "onebot" => Some(Self::Onebot),
            "discord" => Some(Self::Discord),
            "telegram" => Some(Self::Telegram),
            _ => None,
        }
    }
}

impl fmt::Display for 平台 {
    fn fmt(&self, 格式: &mut fmt::Formatter<'_>) -> fmt::Result {
        格式.write_str(self.编码())
    }
}

#[cfg(test)]
mod 测试 {
    use super::平台;

    #[test]
    fn 编码与解析能往返() {
        for 每个 in [平台::Onebot, 平台::Discord, 平台::Telegram] {
            assert_eq!(平台::解析(每个.编码()), Some(每个), "{}", 每个.编码());
        }
    }

    #[test]
    fn 未知编码返回_none() {
        // 协议里 `platform` 是平台侧传来的字符串，不能假装认识它。
        assert_eq!(平台::解析("qq-guild"), None);
        assert_eq!(平台::解析(""), None);
        // 大小写敏感：编码是契约的一部分，不做"宽容匹配"。
        assert_eq!(平台::解析("OneBot"), None);
    }
}
