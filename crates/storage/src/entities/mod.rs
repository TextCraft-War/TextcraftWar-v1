//! 数据库表实体汇总：每张表一个模块。
//! 文件名用英文（rustc 禁止非 ASCII 模块文件名自动加载），模块名保持中文。

#[path = "active_enums/active_enums.rs"]
pub mod 枚举;

#[path = "platform_binding/platform_binding.rs"]
pub mod 平台绑定;

#[path = "player/player.rs"]
pub mod 玩家;
