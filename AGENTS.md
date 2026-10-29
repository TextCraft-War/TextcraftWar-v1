# TextcraftWar — Agent 工作指南

> 本文件是给 AI coding agent 的项目级硬规范。所有规则均为强制,除非显式标注"建议"。

## 一、项目定位

- **名字**:英文 `Textcraft: War`,中文 `文字工艺：战争`,口语简称"战争文游"。
- **文档**:入口在 [docs/README.md](./docs/README.md)。改行为必须同时改文档;
  新决策追加到 [docs/decisions.md](./docs/decisions.md),不改历史条目。
- **是什么**:高性能多人文字战争游戏服务器(纯后端,不含前端/2D 地图)。
- **调性**:社交属性为主、大战略属性较少;定位"简陋但好玩";面向全球玩家。
- **核心循环**:玩家(同志)签到 → 工业生产(工人/产出/盈利/稳定度)→ 征战结算。
- **开源**:MIT。
- **性能红线**:前代文游"一旦打仗就卡爆"是前车之鉴,战争结算路径必须低延迟、禁全表锁。

## 二、技术栈

- Rust 2024 edition,MSRV 1.85,工具链锁定 [1.97.0](./rust-toolchain.toml)。
- 异步运行时:`tokio`(按需启用 `rt-multi-thread`/`net`/`macros`/`time`/`sync`/`signal`,**不用** `full`)。
- Web 框架:`axum 0.8`(`features=["ws"]`),仅用 WebSocket。
- 数据库访问:**`sqlx 0.8` 直写 SQL**(不是 ORM),`postgres` + `runtime-tokio` + `tls-rustls` + `macros` + `migrate`。
  - 默认用 `sqlx::query_as` / `query`(运行期校验),**不依赖**构建期数据库连接,保证 `cargo check` 在没装 PG 的机器上也能过。
  - 迁移用 `sqlx::migrate!`(编译期内嵌,离线可用)。
- 数据库:**PostgreSQL**(不是 SQLite)。
- 缓存:**v1 不引入缓存**。2 人规模没有缓存问题,过早引入只会掩盖真实的 SQL 问题。
- i18n:**v1 只有中文**,不引入 `fluent`。玩家可见文本统一收敛到 `server` 的 `文案` 模块(函数边界),将来接多语言只换实现。
- 日志:`tracing` + `tracing-subscriber`,**禁止** `println!`/`eprintln!`/`dbg!`。
- 工具链:`clippy`(仅 `all` 组 deny,`pedantic`/`nursery` 为 warn)+ `rustfmt` + `cargo-deny` + `bacon`。
  - 原则:**只有"会变成线上事故"的规则才 deny**,风格类一律 warn。

## 三、Workspace 架构

四 crate,依赖单向流动,且**由编译器强制**:

```text
server -> { domain, storage, protocol }
storage -> domain
protocol -> serde(不含 domain)
domain -> 无
```

- `crates/domain` — 纯游戏规则(签到、工业产出、战争结算)。**零 IO、零 async、零外部依赖**,
  全部写成 `fn 结算(旧状态, 指令) -> (新状态, 事件)` 的纯函数,可靠 `cargo test` 单独验证。
  想在这里 `use` 数据库或 JSON 会直接编译失败——这是刻意的。
- `crates/protocol` — 发给 Koishi 插件的 wire DTO 与 serde 序列化。**不引用 domain 类型**,
  防止内部模型与对外契约互相绑架。
- `crates/storage` — sqlx 查询 + "数据库行 ↔ domain 类型"的翻译。只做翻译,不做规则。
- `crates/server` — axum WebSocket 服务器、文本命令解析、事务编排、文案渲染。

## 四、通信架构(Koishi 插件对接)

- `server` 暴露**通用 WebSocket 端点**,收发文本命令 + JSON 消息(消息类型由 `protocol` crate 定义)。
- Koishi 侧另写对接插件:注册聊天命令、转发到 rust 文游、回传渲染结果。
- 复用 Koishi 适配器生态(QQ/Discord/Telegram 等),**不自己实现 Satori 协议**。
- JSON 字段名遵循英文白名单(见第六章),Rust 侧中文标识符用 `#[serde(rename = "...")]` 与 JSON key 解耦。

## 五、数据库规范

- 仅 PostgreSQL。连接池用 `sqlx::postgres::PgPoolOptions` 配置。
- 表名、键名使用中文。**注意**:中文标识符在 SQL 里必须双引号(`"玩家"`),写 SQL 时别漏。
- 枚举列一律存 `TEXT`,不要用 PG 的 `enum` 类型:`storage` 里用 `String` 读出,
  再 `TryFrom` 成 domain 的枚举——这样加枚举变体不需要 `ALTER TYPE`,也不会把 DB 类型绑进 Rust 类型。
- 战争结算等高并发写路径:
  - 优先批量更新,避免循环逐行写。
  - 事务粒度最小化,禁用大事务锁全表。
  - 读路径不做缓存(v1),直接查;真出现瓶颈再加。
- 事务边界归 `server`(应用层)所有:`storage` 的函数接收 `&impl sqlx::Executor`,
  自己**不开事务**,避免"跨两个玩家写一半成功"。

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

## 七、文案与 i18n

- **v1 只有中文**。玩家可见文本**禁止**散落在业务代码里,统一走 `server` 的 `文案` 模块。
- 将来接多语言时只换 `文案` 模块的内部实现,调用点不动——这就是现在不引入 fluent 的理由。
- 数值一律千分位(`1,500,000,000`);对齐、符号(`■` 等)要能在低信息熵语言下也一眼分辨。

## 八、编码规范(强制)

- `unsafe_code = "forbid"`,**绝不**写 unsafe。
- 错误用 `Result` 显式传播;`panic!`/`unwrap`/`expect`/`todo!`/`unimplemented!` 全 `deny`
  (测试代码已在 [.clippy.toml](./.clippy.toml) 里单独放开)。
- crate 错误类型用 `thiserror` 定义。
- `clippy`:`all` 组 `deny`;`pedantic`/`nursery` 为 `warn`(看得见,但不挡路)。
- 精度损失/截断/符号丢失的类型转换 `deny`——v1 经济全整数,不会误伤。
- 异步中**禁止**持锁等待(`await_holding_lock = "deny"`)。
- `missing_docs` 为 `warn`,但**每个 crate 的根 `//!` 必须写清职责与依赖方向**,这不是可选项。

## 九、构建与检查命令

| 操作 | 命令 |
|---|---|
| 检查 | `cargo check` |
| lint | `cargo clippy`(pedantic/nursery 为 warn) |
| 严格 lint | `cargo clippy -- -D warnings`(或 `bacon` 的 `clippy-strict` 作业) |
| 格式化 | `cargo fmt` |
| 测试 | `cargo test -- --nocapture` |
| watch | `bacon`(默认 check 作业) |
| 依赖审查 | `cargo deny check` |
| Release 构建 | `cargo build --release`(`opt-level=3`+`fat LTO`+`codegen-units=1`+`panic=abort`+`strip=symbols`) |

## 十、禁止事项

- 禁止 `unsafe`。
- 禁止 `panic`/`unwrap`/`expect`/`todo!`/`unimplemented!`。
- 禁止 `println!`/`eprintln!`/`dbg!`(用 `tracing`)。
- 禁止硬编码玩家可见字符串(统一走 `文案` 模块)。
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
