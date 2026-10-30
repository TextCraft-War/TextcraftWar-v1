# 协议：Koishi ↔ Rust

> 状态：草案 ｜ 最后更新：2026-10-29
>
> 本文档是**跨仓库契约**。Koishi 插件在另一个仓库，两边的实现都必须以本文为准。
> 改这个文件 = 改契约，必须同时知会插件侧。

## 一、传输

- 单端点 WebSocket：`ws://<host>:<port>/ws`
- 文本帧，每帧一个 JSON 对象（不是 JSON 数组，一帧一消息）
- 服务端不主动发起业务消息；**所有 reply 都是对某条 command 的回应**
- 一条连接 = 一个平台。多平台就开多条连接，不做多路复用

## 二、连接生命周期

```
Koishi                          Rust
  │  ──── hello ────────────────►│
  │                              │ 校验 token / 版本
  │  ◄─── ready ──────────────── │
  │                              │
  │  ──── command ──────────────►│
  │  ◄─── reply / error ──────── │
  │                              │
  │  ──── ping ─────────────────►│
  │  ◄─── pong ───────────────── │
```

握手失败：服务端回一条 `error` 然后关闭连接。

## 三、消息

### hello（Koishi → Rust）

```json
{
  "type": "hello",
  "platform": "onebot",
  "token": "共享密钥",
  "protocolVersion": 1
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `platform` | string | `onebot` / `discord` / `telegram`，与 `地区频道绑定` 表里的枚举一致 |
| `token` | string | 共享密钥，v1 是静态配置 |
| `protocolVersion` | number | 主版本号；不匹配直接拒绝 |

### ready（Rust → Koishi）

```json
{ "type": "ready", "protocolVersion": 1 }
```

### command（Koishi → Rust）

```json
{
  "type": "command",
  "id": "c-7f3a",
  "platform": "onebot",
  "channelId": "523909412",
  "userId": "3071303571",
  "nickname": "晓",
  "text": "签到"
}
```

| 字段 | 说明 |
|---|---|
| `id` | 由 Koishi 生成，唯一。用于把 reply 关联回请求，也是未来重试去重的依据 |
| `channelId` | 群 / 频道 id。**游戏只在群里玩**，私聊不转发 |
| `userId` | 平台用户 id |
| `nickname` | 平台昵称快照（可缺省） |
| `mentions` | 被 @ 到的账号，按出现顺序：`[{ "platform": "onebot", "userId": "123456" }]`。可缺省 |
| `text` | 玩家原话，**未解析**。命令解析在 Rust 侧 |

> **为什么 @ 要结构化传**：玩家**没有名字**，`邀请 @某人` / `炮击 @某人` / `掠夺 @某人`
> 的目标只能是平台账号。而各平台的 @ 文本格式完全不同（`<at id="…"/>`、`<@123>`、`@昵称`），
> 让 Rust 去解析等于把三套平台怪癖搬进游戏服务器。Koishi 已经把它们统一成 id 了，直接传。
>
> 一条 `command` 里的 `mentions` 理论上可以是别的平台（比如引用了一条跨平台消息），
> 所以每条提及自带 `platform`，而不是复用外层的 `platform`。

> 为什么要传原始 `text` 而不是结构化命令：新增一条指令只需要改 Rust，
> 不用同时改 JS 插件。插件永远是"哑管道"。

### reply（Rust → Koishi）

```json
{
  "type": "reply",
  "id": "c-7f3a",
  "platform": "onebot",
  "channelId": "523909412",
  "segments": [
    { "type": "text", "text": "■ 签到成功\n■ 获得生活资料 ×1,200" }
  ]
}
```

- `segments` 是数组，不是字符串——为了将来能发图片（v2 的回复里配了插图）。
- v1 只实现 `text` 段；`image` 段已预留：`{ "type": "image", "url": "https://..." }`。
- 回复里带 `channelId`：Rust 侧完全无状态，路由由 Koishi 负责。

### error（Rust → Koishi）

```json
{
  "type": "error",
  "id": "c-7f3a",
  "platform": "onebot",
  "channelId": "523909412",
  "code": "unknown_command",
  "message": "不认识的指令"
}
```

`message` 是**给玩家看的**中文，不是调试信息。内部错误细节只进 `tracing` 日志，绝不进这里。

错误码（v1 草案）：

| code | 含义 |
|---|---|
| `bad_request` | 字段缺失或类型不对 |
| `unknown_command` | 不认识这条指令 |
| `not_registered` | 玩家尚未注册 |
| `no_region` | 当前频道没有绑定地区，且拒绝自动注册 |
| `forbidden` | 权限不足（如非联军成员想驻军） |
| `internal` | 服务器内部错误 |

### ping / pong

```json
{ "type": "ping" }
{ "type": "pong" }
```

Koishi 每 30 秒发一次；Rust 收到就回。60 秒没有任何帧则断开。

## 四、幂等

**v1 不做传输层去重，靠业务幂等。**

例：重复发 `签到`，第二次返回"今天已经签过了"，而不是报错。这比维护一张去重表简单，而且对玩家更友好。

`id` 仍然必须传，作用有二：关联请求与回复、将来真需要重试时做去重依据。

## 五、安全边界

- `token` 错误 → 拒绝握手。
- 服务端**不信任** `userId` / `nickname` / `channelId` 的任何格式，一律当不可信字符串处理。
- `text` 长度上限 512 字符，超长直接拒绝（防止有人往日志里灌数据）。
- 所有字段进 SQL 一律走参数绑定（`sqlx` 的 `.bind()`），**绝不拼接字符串**。

## 六、版本策略

`protocolVersion` 是主版本号。规则：

- **加字段**：不改版本号（两端都必须容忍未知字段）。
- **删字段 / 改语义**：版本号 +1，且服务端同时支持新旧两个版本一个发布周期。

## 七、待定

❓ `platform` 的取值是否要支持自定义（如 `qq-guild`）？
❓ 图片段需要 Koishi 侧能接受 URL 还是文件路径——取决于插件用什么发图 API。
❓ 是否需要 `reply` 支持"多条消息"（segments 数组本身可以表达，但要不要一次发多条由 Koishi 决定）。
