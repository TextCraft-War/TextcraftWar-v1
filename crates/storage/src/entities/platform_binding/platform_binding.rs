//! `平台绑定` 表：跨平台账号绑定（`平台` × `平台_uid` -> `玩家`）。
//!
//! 拆成独立表而非在 `玩家` 表上加 onebot/discord/telegram/whatsapp 四列：
//! - 新平台只需加 [`平台`] 枚举变体，无需 `ALTER TABLE`；
//! - 一个玩家可绑定多个平台账号，1:N 天然多行；
//! - 登录路径按（平台，`平台_uid`）复合唯一索引单点命中。
//!
//! 复合唯一索引（平台，`平台_uid`）与 `玩家_id` 普通索引在迁移中定义
//! （`DeriveEntityModel` 的属性只支持单列唯一，复合唯一须走迁移）。
// sea-orm 派生宏对枚举列生成的代码含多余限定路径，触发 `unused_qualifications`，
// 结构体级 `#[allow]` 无法覆盖派生展开，上游修复前本模块内豁免。
#![allow(unused_qualifications)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::枚举::平台;

/// `平台绑定` 表的行模型：一行即一条"某平台账号绑定到某玩家"的记录。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "平台绑定")]
pub struct Model {
    /// 主键，数据库自增。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 绑定的玩家 id（逻辑外键，事务保证一致，不建物理外键）。
    #[serde(rename = "playerId")]
    pub 玩家_id: i64,
    /// 绑定的平台。
    #[serde(rename = "platform")]
    pub 平台: 平台,
    /// 平台侧用户标识（如 QQ 号、Discord/Telegram/WhatsApp 的用户 id）。
    #[serde(rename = "platformUid")]
    pub 平台_uid: String,
    /// 绑定时间（Unix 秒）。
    #[serde(rename = "boundAt")]
    pub 绑定时间: i64,
}

/// `平台绑定` 表的关系定义（逻辑关联玩家，不建物理外键）。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
