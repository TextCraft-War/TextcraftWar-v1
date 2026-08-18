# TextcraftWar — Agent 工作指南

> 本文件是给 AI coding agent 的项目级硬规范。所有规则均为强制,除非显式标注"建议"。

## 一、项目定位

- **是什么**:高性能多人文字战争游戏服务器(纯后端,不含前端/2D 地图)。
- **调性**:社交属性为主、大战略属性较少;定位"简陋但好玩";面向全球玩家。
- **核心循环**:玩家(同志)签到 → 工业生产(工人/产出/盈利/稳定度)→ 征战结算。
- **开源**:MIT。
- **性能红线**:前代文游"一旦打仗就卡爆"是前车之鉴,战争结算路径必须低延迟、禁全表锁。

## 二、技术栈

- Rust 2024 edition,MSRV 1.85,工具链锁定 [1.97.0](./rust-toolchain.toml)。
- 异步运行时:`tokio`(full feature)。
- Web 框架:`axum 0.7`(`features=["ws"]`),仅用 WebSocket。
- ORM:`sea-orm 1.0`,driver 用 `sqlx-postgres`。
- 数据库:**PostgreSQL**(不是 SQLite)。
- 缓存:`moka 0.12`(`future` feature)。
- i18n:`fluent 0.16` + `unic-langid 0.9`,资源在 `locales/<lang>/main.ftl`。
- 日志:`tracing` + `tracing-subscriber`,**禁止** `println!`/`eprintln!`/`dbg!`。
- 工具链:`clippy`(all+pedantic+nursery deny)+ `rustfmt` + `cargo-deny` + `bacon`,已配,保持现状。

## 三、Workspace 架构

三 crate,依赖单向流动 `server → protocol + storage`:

- `crates/protocol` — 共享消息类型与 serde 序列化(发给 Koishi 插件的 JSON 契约)。
- `crates/storage` — SeaORM 实体/迁移 + Moka 缓存,封装数据访问。
- `crates/server` — Axum WebSocket 服务器,组装 protocol + storage。

## 四、通信架构(Koishi 插件对接)

- `server` 暴露**通用 WebSocket 端点**,收发文本命令 + JSON 消息(消息类型由 `protocol` crate 定义)。
- Koishi 侧另写对接插件:注册聊天命令、转发到 rust 文游、回传渲染结果。
- 复用 Koishi 适配器生态(QQ/Discord/Telegram 等),**不自己实现 Satori 协议**。
- JSON 字段名遵循英文白名单(见第六章),Rust 侧中文标识符用 `#[serde(rename = "...")]` 与 JSON key 解耦。

## 五、数据库规范

- 仅 PostgreSQL。`storage` crate 的 SeaORM feature 为 `sqlx-postgres`。
- 表名、键名使用中文(契合中文高信息熵,便于记忆战争字段)。
- 战争结算等高并发写路径:
  - 优先批量更新,避免循环逐行写。
  - 事务粒度最小化,禁用大事务锁全表。
  - 读路径优先命中 Moka 缓存,miss 再查库。
- 连接池通过 `SeaOrmConnectOptions` 配置,池大小按战争峰值并发估算。

## 六、命名规范(中文优先,分层方案)

### 6.1 中文范围

- 函数、变量、常量、结构体、枚举、模块名:**中文**。
- 数据库表名、键名:**中文**。
- 文档注释(`///`、`//!`):**中文**。

### 6.2 英文白名单(不得中文化)

- 社区规范术语:`id`、`uid`、`token`、`url`、`json`、`http`、`ws`、`sql`、`orm`、`db`、`api`。
- 外部库函数与 trait 方法签名(如 `Display::fmt`、`From`、`Deref`、`Serialize`、`Deserialize`)。
- Rust 关键字与标准库类型。

### 6.3 命名风格

- 中文短语用下划线分隔:`前线_损耗率`。
- 中英混排按 snake_case:`worker_id`、`用户_uid`。
- 类型名(结构体/枚举/trait)UpperCamelCase,中文名直接用中文:`工人`、`部队类型`。
- 常量 SCREAMING_SNAKE_CASE,中文常量直接中文:`前线补给上限`。

### 6.4 技术约束

- `non_ascii_idents` lint 改为 `allow`(放开中文标识符)。
- derive 宏偶发中文标识符兼容问题,极少数情况下该位置回退英文并加注释说明。
- serde JSON key 面向 Koishi 插件的字段用 `#[serde(rename = "...")]` 分离 Rust 中文与 JSON 英文。

## 七、i18n 规范

- 所有玩家可见文本走 Fluent,**禁止硬编码玩家可见字符串**。
- 默认 `zh-CN`,必须预留 `en`、`ru` 等语言位。
- 低信息熵语言(英文/俄语)易致"文游难辨",文案设计须用符号(`■` 等)、对齐、数值千分位(`1,500,000,000`)辅助辨识(参考示例排版)。

## 八、编码规范(强制)

- `unsafe_code = "forbid"`,**绝不**写 unsafe。
- 错误用 `Result` 显式传播;`panic!`/`unwrap`/`expect`/`todo!`/`unimplemented!` 全 `deny`。
- crate 错误类型用 `thiserror` 风格定义(建议引入 `thiserror` 依赖)。
- 所有 `pub` 项必须有文档注释(`missing_docs = "deny"`)。
- `clippy` 的 `all`+`pedantic`+`nursery` 三组 `deny`,warnings 视为 error。
- 精度损失/截断/符号丢失的类型转换 `deny`,需显式转换。
- 异步中**禁止**持锁等待(`await-holding-lock = "deny"`),`large-futures = "deny"`。

## 九、构建与检查命令

| 操作 | 命令 |
|---|---|
| 检查 | `cargo check` |
| 严格 lint | `cargo clippy -- -D warnings` |
| 格式化 | `cargo fmt` |
| 测试 | `cargo test -- --nocapture` |
| watch | `bacon`(默认 check 作业) |
| 依赖审查 | `cargo deny check` |
| Release 构建 | `cargo build --release`(`opt-level=3`+`fat LTO`+`codegen-units=1`+`panic=abort`+`strip=symbols`) |

## 十、禁止事项

- 禁止 `unsafe`。
- 禁止 `panic`/`unwrap`/`expect`/`todo!`/`unimplemented!`。
- 禁止 `println!`/`eprintln!`/`dbg!`(用 `tracing`)。
- 禁止硬编码玩家可见字符串(用 Fluent)。
- 禁止任意 registry/git 依赖,仅 `crates.io`(见 [deny.toml](./deny.toml))。
- 禁止引入有漏洞/未维护/unsound 的 crate(见 [deny.toml](./deny.toml))。
- 禁止在 storage 之外直接访问数据库(数据访问收敛到 `storage` crate)。
- 禁止自己实现 Satori 协议(走 Koishi 插件对接路线)。

## 十一、部署目标

- `x86_64-pc-windows-msvc`(主开发平台)
- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`

## 十二、已知约束

- 中文标识符:derive 宏偶发兼容问题,必要时回退英文。
- PG 部署:用户需自备 PostgreSQL 实例。
- Koishi 侧对接插件不在本仓库(另开仓库),本仓库只暴露 WS 契约。
