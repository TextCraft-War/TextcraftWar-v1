//! storage crate 的统一错误类型。

use thiserror::Error as ThisError;

use crate::entities::枚举::名称审核状态;

/// 存储层错误。
#[derive(Debug, ThisError)]
pub enum 错误 {
    /// 状态机收到非法转移请求。
    #[error("非法状态转移：{从:?} -> {到:?}")]
    非法状态转移 {
        /// 转移前状态。
        从: 名称审核状态,
        /// 请求去到的状态。
        到: 名称审核状态,
    },
}
