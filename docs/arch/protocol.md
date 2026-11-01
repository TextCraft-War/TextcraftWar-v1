# 协议：Koishi ↔ Rust

> 本文档是**跨仓库契约**。Koishi 插件在另一个仓库，两边的实现都必须以本文为准。
> 改这个文件 = 改契约，必须同时知会插件侧。
>
> 位置注记：2026-10-31 从 `docs/protocol.md` 迁入 `docs/arch/protocol.md`（文档三域重排）。
> 若插件侧有指向旧路径的链接，请更新；契约内容未因迁移发生任何变化。
>
> 修订注记：2026-11-01 按 [decisions.md D51](../decisions.md) 把 `command` 从
> 原始 `text` 改为结构化 `command` + `args`——指令在 Koishi 侧注册并翻译。
> v1 契约当时尚无真实消费者，就地修订，`protocolVersion` 仍为 1。

## 一、传输

- 单端点 WebSocket：`ws://<host>:<port>/ws`
- 文本帧，每帧一个 JSON 对象（不是 JSON 数组，一帧一消息）
- 服务端不主动发起业务消息；**所有 reply 都是对某条 command 的回应**
- 一条连接 = 一个平台。多平台就开多条连接，不做多路复用
- 「一条连接 = 一个平台」由服务端**强制**：每条 `command` 的 `platform` 必须与
  握手时声明的一致，否则回 `bad_request`——防止一条连接冒充另一个平台发消息

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

握手**之后**，业务层的失败（JSON 解析不了、指令不认识、玩家弄错）只回一条 `error`，
**不断连**——把一条坏消息的代价放大成"整条连接掉线"没有道理；只有传输本身坏了才断开。

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
  "command": "签到"
}
```

带参指令（D51）：

```json
{
  "type": "command",
  "id": "c-7f3b",
  "platform": "onebot",
  "channelId": "523909412",
  "userId": "3071303571",
  "command": "招募工人",
  "args": { "数量": 5 }
}
```

| 字段 | 说明 |
|---|---|
| `id` | 由 Koishi 生成，唯一。用于把 reply 关联回请求，也是未来重试去重的依据 |
| `channelId` | 群 / 频道 id。**游戏只在群里玩**，私聊不转发 |
| `userId` | 平台用户 id |
| `nickname` | 平台昵称快照（可缺省） |
| `mentions` | 被 @ 到的账号，按出现顺序：`[{ "platform": "onebot", "userId": "123456" }]`。可缺省 |
| `command` | 指令名，与 Koishi 侧注册的游戏指令同名。**指令在 Koishi 侧注册**：触发、切参、参数类型、arity、帮助都由 Koishi 命令系统完成 |
| `args` | 指令参数对象，键为参数名。**零参指令省略**；未知键两端都要容忍（加字段不破坏） |

> **信封与载荷的分界（D51）**：信封字段（`id` / `platform` / `channelId` / …）是
> 协议管道词汇，英文 camelCase；`command` 的**值**与 `args` 的**键**是游戏词汇，
> 用中文，与 design.md 指令表一致——`"command": "签到"` 本来就是中文，
> `"args": {"数量": 5}` 与之同侧。
>
> **语法归 Koishi，语义归 Rust**：玩家打错字由 Koishi 提示（近似匹配是它的内建能力），
> Koishi 只转发注册过的指令——所以 Rust 的 `unknown_command` 只在两端版本偏差时出现，
> 平时不会触发。参数的类型（`<数量:number>`）由 Koishi 声明负责，
> 数值的**游戏语义**（数量 > 0、每日上限）归 Rust 校验。

> **为什么 @ 要结构化传**：玩家**没有名字**，`邀请 @某人` / `炮击 @某人` / `掠夺 @某人`
> 的目标是平台账号；跨群时同一个目标也可以直接写编号 `P123456`，那只是文本里的普通字符，
> 由 Rust 解析。而各平台的 @ 文本格式完全不同（`<at id="…"/>`、`<@123>`、`@昵称`），
> 让 Rust 去解析等于把三套平台怪癖搬进游戏服务器。Koishi 已经把它们统一成 id 了，直接传。
>
> 一条 `command` 里的 `mentions` 理论上可以是别的平台（比如引用了一条跨平台消息），
> 所以每条提及自带 `platform`，而不是复用外层的 `platform`。
> 指令参数里怎么**引用**一条提及（按下标还是内联平台 + id），随 S6 首个
> 指向玩家的指令一起定。

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
| `bad_request` | 字段缺失或类型不对；指令参数形状不对（缺参、类型不对、零参指令带了 `args`）；`platform` 与握手不一致；握手后又发 hello；规则拒绝（如 `签到` 冷却中） |
| `unknown_command` | 不认识这条指令。指令在 Koishi 侧注册（D51），玩家打错字到不了 Rust——它出现即两端版本偏差 |
| `not_registered` | 玩家尚未注册——未注册的人只有 `注册` 能用，发别的指令都回它 |
| `no_region` | 当前频道没有绑定地区，且拒绝自动注册；`注册` 时所在频道未绑定地区也回它，不建玩家 |
| `forbidden` | 权限不足（如非联军成员想驻军）；握手 token 错误 |
| `internal` | 服务器内部错误（存储失败、编号生成重试用尽） |

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
- 单帧上限 **64 KiB**（传输层 `max_message_size` 强制）。超限直接断连——帧没读完，
  无处回 `error`。防的是"往日志与内存里灌数据"，不是限制玩法。
- 所有字段进 SQL 一律走参数绑定（`sqlx` 的 `.bind()`），**绝不拼接字符串**。

## 六、版本策略

`protocolVersion` 是主版本号。规则：

- **加字段**：不改版本号（两端都必须容忍未知字段）。
- **删字段 / 改语义**：版本号 +1，且服务端同时支持新旧两个版本一个发布周期。

> D51 的 `text` → `command` + `args` 修订发生在首个真实部署之前（插件仓库未建），
> 就地修订、版本号未动；上面的双版本纪律从第一个真实部署起算。

## 七、待定

❓ `platform` 的取值是否要支持自定义（如 `qq-guild`）？
❓ 图片段需要 Koishi 侧能接受 URL 还是文件路径——取决于插件用什么发图 API。
❓ 是否需要 `reply` 支持"多条消息"（segments 数组本身可以表达，但要不要一次发多条由 Koishi 决定）。
