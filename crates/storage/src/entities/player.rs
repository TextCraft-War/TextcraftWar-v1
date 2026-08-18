//! `玩家` 表：账号身份等冷数据（注册后几乎不变）。
//! 工业生产、战争等高频读写字段请拆到对应热表，避免宽行重写。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `玩家` 表的行模型：一行即一位玩家（同志）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "玩家")]
pub struct Model {
    /// 主键，数据库自增。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 平台用户 uid（英文白名单术语，列名保持英文）。
    #[sea_orm(unique)]
    pub uid: String,
    /// 玩家昵称。
    #[serde(rename = "nickname")]
    pub 昵称: String,
    /// 是否被封禁。
    #[serde(rename = "banned")]
    pub 是否封禁: bool,
    /// 封禁原因（未封禁时为空）。
    #[serde(rename = "banReason")]
    pub 封禁原因: Option<String>,
    /// 账号创建时间（Unix 秒）。
    #[serde(rename = "createdAt")]
    pub 创建时间: i64,
}

/// `玩家` 表的关系定义（暂无外键，拆表后在此补 1:1 关系）。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
