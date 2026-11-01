# protocol — wire DTO

>
> 契约正文见 [docs/protocol.md](../../../docs/arch/protocol.md)。本文只写**实现约定**。

## 一、这个 crate 是什么

Koishi ↔ Rust 的 JSON 契约，一个纯数据 crate。没有逻辑，没有 IO。

## 二、实现约定

### 2.1 消息是带标签的枚举

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum 上行消息 {
    Hello { ... },
    Command { ... },
    Ping,
}
```

- 用 `#[serde(tag = "type")]`（内部标签），Koishi 侧最友好：一个 `switch (msg.type)` 就够。
- 变体名用中文（符合项目命名规范），但 `type` 的**取值**必须由 `#[serde(rename)]` 固定成英文小写：
  `hello` / `command` / `ping`。

### 2.2 字段名一律 camelCase 英文

Rust 侧字段可以是中文，但 JSON key 必须英文：

```rust
#[serde(rename = "channelId")]
pub 频道_id: String,
```

理由见 [decisions.md D10](../../../docs/decisions.md)。

> D51 的分界：**信封**字段（`id` / `platform` / `channelId` / …）是协议管道词汇，
> 英文 camelCase；`command` 的**值**与 `args` 的**键**是游戏词汇，用中文
> （与 design.md 指令表一致）。

### 2.3 不引用 domain

本 crate **没有** `domain` 依赖，这是刻意的（见 architecture 第三节）。
如果发现某个类型"两边都想用"，正确做法是在 `server` 里写转换，而不是让 protocol 依赖 domain。

### 2.4 容忍未知字段

两端都必须是**加字段不破坏**的。serde 默认忽略未知字段，不要加 `deny_unknown_fields`。

### 2.5 版本常量

```rust
/// 当前协议主版本号。
pub const 协议主版本: u32 = 1;
```

## 三、测试

- 每个消息类型至少一条 `serde_json` 往返测试。
- 用**固定的 JSON 字符串**做断言（不是 `serde_json::to_value` 自比），否则字段名写错也测不出来。

```rust
#[test]
fn command_序列化形状固定() {
    let json = serde_json::to_string(&样例).unwrap();
    assert!(json.contains(r#""channelId":"523909412""#));
}
```

## 四、还没做的事

- [x] 消息类型定义（含 D51 的 `command` + `args`）
- [x] 往返测试
- [ ] `segments` 的 `image` 变体（字段待定，见 docs/arch/protocol.md 第七节）
