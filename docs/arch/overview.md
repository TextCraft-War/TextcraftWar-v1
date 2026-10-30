# 工程架构

>
> 游戏规则见 [design.md](../game/design.md)；wire 格式见 [protocol.md](./protocol.md)。

## 一、总览

```
                        ┌──────────────────────┐
                        │       server         │
                        │  axum · WS · 文本解析 │
                        │  事务编排 · 文案渲染  │
                        └───┬──────┬──────┬────┘
                            │      │      │
              ┌─────────────┘      │      └──────────────┐
              ▼                    ▼                     ▼
      ┌───────────────┐    ┌───────────────┐    ┌────────────────┐
      │    domain     │    │    storage    │    │    protocol    │
      │ 纯规则 · 零依赖│◄───│ sqlx · 行映射 │    │ serde DTO      │
      └───────────────┘    └───────────────┘    └────────────────┘
```

依赖方向单向，**且由编译器强制**，不靠自觉。

## 二、四个 crate 的职责

| crate | 一句话 | 可以依赖 | 绝不依赖 |
|---|---|---|---|
| `domain` | 游戏规则，纯函数 | **无** | 一切 |
| `protocol` | 对外 JSON 契约 | serde | domain |
| `storage` | SQL 与"行 ↔ domain 类型"的翻译 | sqlx、domain | protocol、server |
| `server` | WS、文本解析、编排、文案 | 全部 | — |

细节见各自的 `crates/*/docs/impl.md`。

## 三、三条硬边界

这三条不是约定，是**写错就编译不过**的物理约束：

**1. `domain` 的 `[dependencies]` 是空的。**
连 serde 都不给。想在这里 `use sqlx`、写 `#[derive(Serialize)]`、或者 `async fn`，都会直接失败。
目的：规则层永远可以被单独 `cargo test` 验证，不需要 PostgreSQL、不需要 WebSocket。

**2. `protocol` 不引用 `domain`。**
wire DTO 和内部模型必须各自演化。如果 protocol 直接复用 domain 的枚举，那改一个内部字段名就会改动 Koishi 插件的契约——两个仓库被焊死。

**3. `server` 拿不到 `sqlx`。**
storage 把连接池与事务包成 **newtype**（D45）——类型别名不屏蔽底层固有方法，
`池.begin()` 照编译，newtype 才是物理边界。`server` 拿到的只有不透明的事务句柄 +
`开事务/提交/回滚` 三个动作；查询函数**读**收 `执行者`（池直查或事务内读）、
**写**只收 `&mut 事务`（D46）——拿池直接写库编译不过。
事务边界归 `server` 所有：战争结算要同时改两个玩家和一张联军表，
"跨实体写一半"必须整体回滚，而这个编排只有 `server` 能做。

## 四、一条指令的生命周期

以 `签到` 为例：

```
QQ 群 "签到"
  → Koishi 插件（另一仓库）打包成 command JSON
  → WebSocket
  → server: 解 JSON → 取 (platform, channelId)
  → storage: channelId → 地区
  → storage: (platform, userId) → 玩家
  → server: 调 domain::签到(玩家状态, 当前时间)
  → server: 开事务 → storage 写玩家 + 写事件 → 提交
  → server: 文案::签到结果(...) → reply JSON
  → WebSocket → Koishi → 群里
```

关键点：**每一步只做一件事**。domain 不知道有数据库，storage 不知道有 JSON，protocol 不知道有游戏规则。

## 五、为什么不是 DDD 全套

DDD 的**语言**（统一语言、限界上下文）有用，DDD 的**机器**（聚合、仓储、工厂、领域事件）在这个规模是纯税。

这个项目的真实复杂度在**数值规则**，不在持久化。所以采用「函数核心 / 命令外壳」：

- `domain` = 函数核心，纯计算，可以测到死
- `server` = 命令外壳，管 IO、事务、错误呈现
- `storage` = 翻译层，只做行 ↔ 类型

好处是"战争结算"天然满足性能红线：纯函数批量算完再一次性写库，不存在边算边锁行。

## 六、目录约定

```
crates/
├── domain/
│   ├── docs/impl.md     本 crate 实现约定
│   └── src/lib.rs         规则（按业务拆模块）
├── protocol/
│   ├── docs/impl.md
│   └── src/lib.rs         消息类型
├── storage/
│   ├── docs/impl.md     表结构、事务、映射
│   ├── migrations/        sqlx 迁移（编译期内嵌）
│   └── src/lib.rs
└── server/
    ├── docs/impl.md     WS 生命周期、分发、文案
    └── src/
        ├── main.rs        入口
        ├── 文案.rs        玩家可见文本的唯一出口
        └── ...
```

## 七、硬规范在哪里

工程硬规范（lint、命名、依赖审查）在仓库根 [AGENTS.md](../../AGENTS.md)。本文档只解释**为什么**，不复述规则；两者冲突时以 AGENTS.md 为准。
