//! 后台调度器：定时结算的心跳（结算本体随 S4 落地，D32/D48）。
//!
//! 形状（`crates/server/docs/impl.md` 第五节）：**每 60 秒检查一次**
//! 全服结算进度，不"睡到整点"——启动时间不整，睡到整点会错位；
//! 检查只是一次单行表查询，成本可以忽略。该补算时：
//! `开事务 → storage::锁定状态 → 按时间差补算 → 写状态 → 提交`
//! （串行化点见 `crates/storage/docs/impl.md` 2.6，`FOR UPDATE` 行锁）。
//!
//! 本文件现在只有心跳与巡检；补算函数进来时，进度锚点
//! （`storage::查状态` / `锁定状态` / `写状态`）已经就位。

use std::time::Duration;

use tokio::time::MissedTickBehavior;

/// 巡检周期：60 秒。
const 检查周期: Duration = Duration::from_secs(60);

/// 启动后台调度任务，返回任务句柄（将来做优雅停机时用它收尾）。
///
/// 第一个 tick **立即**触发（`tokio::time::interval` 的语义）：
/// 进程重启后的第一次巡检天然就是"启动先补算"的入口（S4）。
pub fn 启动调度器(池: storage::连接池) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut 心跳 = tokio::time::interval(检查周期);
        // 巡检偶尔慢过 60 秒时不补帧：一次巡检一次检查，堆积没有意义。
        心跳.set_missed_tick_behavior(MissedTickBehavior::Skip);

        tracing::info!(周期秒 = 检查周期.as_secs(), "调度器已启动");

        loop {
            心跳.tick().await;

            if let Err(失败) = 巡检(&池).await {
                // 批处理失败只记日志、不阻塞消息处理，下一轮会补（impl.md 第五节）。
                tracing::warn!(%失败, "调度巡检失败，下一轮重试");
            }
        }
    })
}

/// 一轮巡检：读一次全服结算进度。纯读，不开事务（D46）。
/// S4 在这里按时间差决定要不要开事务补算。
async fn 巡检(池: &storage::连接池) -> Result<(), storage::错误> {
    let 状态 = storage::查状态(storage::执行者::池(池)).await?;

    tracing::debug!(
        上次生产结算 = ?状态.上次生产结算,
        上次日结 = ?状态.上次日结,
        "调度巡检"
    );

    Ok(())
}
