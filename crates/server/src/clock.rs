//! 当前时间。
//!
//! 时间只从这一个地方取：处理器收的是**注入**的 `时间` 参数（D48），
//! 取"现在"的动作收敛在分发层对 [`现在`] 的这一次调用上。
//! 将来要测"跨天签到""保护期到期"，把这里的实现换掉（或把分发层的
//! 取法换成假时钟）就够了，不用去追散落各处的 `SystemTime::now()`。

use std::time::{SystemTime, UNIX_EPOCH};

/// 当前 Unix 秒。
///
/// 系统时钟早于 1970 时返回 `0`——这个场景只可能是时钟被拨坏，
/// 让它在数据里明显不对劲，比 `panic!` 掉整条连接好。
pub(crate) fn 现在() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |时长| i64::try_from(时长.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod 测试 {
    /// 2026 年之后的时钟值（秒）。比这个小的只能是时钟坏了。
    const 二零二六年: i64 = 1_767_225_600;

    #[test]
    fn 返回的是合理的_unix_秒() {
        assert!(super::现在() > 二零二六年);
    }
}
