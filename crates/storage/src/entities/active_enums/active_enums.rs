//! 跨表共享的数据库枚举（SeaORM `ActiveEnum`）。
//!
//! 独立成模块的原因：实体派生宏对枚举列生成的代码会引用这些类型，
//! 若枚举与实体同模块，限定路径被视为多余，触发 `unused_qualifications`。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::错误::错误;

/// 游戏内名称的审核状态机。
///
/// 合法转移（见 [`名称审核状态::转移`]）：
/// - `无申请 | 已驳回 -> 待审`：玩家提交改名申请；
/// - `待审 -> 无申请`：审核通过，`待审名称` 生效为 `当前名称`；
/// - `待审 -> 已驳回`：审核驳回。
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum 名称审核状态 {
    /// 无待审申请（含审核通过生效后的空档）。
    #[serde(rename = "none")]
    #[sea_orm(string_value = "none")]
    无申请,
    /// 有改名申请待审核。
    #[serde(rename = "pending")]
    #[sea_orm(string_value = "pending")]
    待审,
    /// 申请被驳回，玩家可重新提交。
    #[serde(rename = "rejected")]
    #[sea_orm(string_value = "rejected")]
    已驳回,
}

impl 名称审核状态 {
    /// 判断从当前状态到目标状态的转移是否合法。
    const fn 允许转移(self, 目标: Self) -> bool {
        return matches!(
            (self, 目标),
            (Self::无申请 | Self::已驳回, Self::待审) | (Self::待审, Self::无申请 | Self::已驳回)
        );
    }

    /// 执行状态转移；非法转移返回 [`错误::非法状态转移`]。
    ///
    /// 落库时应配合条件更新（`WHERE 名称审核状态 = 转移前状态`），
    /// 防止并发下两个请求同时读到同一旧状态而双写。
    pub const fn 转移(self, 目标: Self) -> Result<Self, 错误> {
        if self.允许转移(目标) {
            return Ok(目标);
        }
        return Err(错误::非法状态转移 {
            从: self, 到: 目标
        });
    }
}

/// 接入的聊天平台（英文白名单术语，落库为英文小写字符串）。
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum 平台 {
    /// `OneBot` v11（QQ 系）。
    #[serde(rename = "onebot")]
    #[sea_orm(string_value = "onebot")]
    Onebot,
    /// Discord。
    #[serde(rename = "discord")]
    #[sea_orm(string_value = "discord")]
    Discord,
    /// Telegram。
    #[serde(rename = "telegram")]
    #[sea_orm(string_value = "telegram")]
    Telegram,
    /// `WhatsApp`。
    #[serde(rename = "whatsapp")]
    #[sea_orm(string_value = "whatsapp")]
    Whatsapp,
}
