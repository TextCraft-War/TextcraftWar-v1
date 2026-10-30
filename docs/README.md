# 文档

这套文档只有一个目标：**让三个月后的你（或另一个 AI agent）能在一小时内接手这个项目**。

## 目录布局

```text
docs/
├── README.md          本文件：导航与职责边界
├── roadmap.md         【唯一的进度与状态事实来源】
├── decisions.md       决策账本：每个重要决定背后的理由
├── game/              玩法域——这是个什么游戏
│   ├── design.md      玩法、规则、数值、指令表
│   ├── catalog.md     落库的稳定编码（物品 / 建筑 / 平台 / 枚举值）
│   └── reference/     原版《马列机器人》指南与《新马列文游策划》的历史文本
├── arch/              架构域——代码为什么这样组织
│   ├── overview.md    crate 分层、依赖方向、数据流
│   ├── data-model.md  表结构与领域类型（唯一事实来源）
│   └── protocol.md    服务器和 Koishi 怎么说话（跨仓库契约）
└── guides/            操作域——怎么跑起来
    ├── deployment.md  环境变量、启动序列、PostgreSQL 准备
    └── testing.md     测试矩阵、测试库准备、CI 防呆
```

`decisions.md` 与 `roadmap.md` 留在根级：前者被 20+ 处源码 doc-comment 链接（挪动 = 改 20+ 处业务代码），
后者是高频读写的进度枢纽，不值得为归类套一层目录。

## 怎么读

按这个顺序，每一步都不依赖后面的内容：

1. [game/design.md](./game/design.md) — 这是个什么游戏，规则是什么
2. [arch/overview.md](./arch/overview.md) — 代码为什么这样分层
3. [arch/data-model.md](./arch/data-model.md) — 表结构与领域类型（唯一事实来源）
4. [game/catalog.md](./game/catalog.md) — 落库的稳定编码（物品 / 建筑 / 平台 / 枚举值）
5. [arch/protocol.md](./arch/protocol.md) — 服务器和 Koishi 怎么说话
6. [roadmap.md](./roadmap.md) — 现在做到哪了，下一步做什么
7. [decisions.md](./decisions.md) — 每个决定为什么这么做
8. [guides/](./guides/) — 部署与测试的操作手册

各 crate 的 `docs/impl.md` 是本 crate 的实现约定，**读代码前看**。

## 职责边界（本文档体系最重要的规则）

**一份事实只写在一个地方。** 别处只放链接，不复制。

| 文档 | 只写这些 | 绝不写 |
|---|---|---|
| `game/design.md` | 玩法、规则、数值、指令 | 表结构、代码结构 |
| `game/catalog.md` | 落库的稳定编码与中文对照 | 规则、数值、表结构 |
| `arch/overview.md` | crate 分层、依赖方向、数据流 | 玩法规则 |
| `arch/data-model.md` | 表结构、领域类型、不变量 | 玩法规则、SQL 查询 |
| `arch/protocol.md` | wire 格式、字段、错误码 | 内部类型、玩法 |
| `roadmap.md` | 切片、验收标准、**状态** | 设计细节 |
| `decisions.md` | 决策与理由 | 当前实现细节 |
| `guides/*.md` | 环境变量、启动、测试的操作步骤 | 设计理由（链回出处） |
| `crates/*/docs/impl.md` | 本 crate 的实现约定 | 全局规则 |

出现矛盾时的优先级：**代码 > `game/design.md` > 其他文档**。代码和文档不一致，说明文档该改了，
或者代码写错了——但一定是其中一件事，不允许"都行"。

## 进度状态只写在一处

每份文档**不再**维护"状态：草案 / 施工中"这类字段——手维护的状态必然漂移。
"做到哪了、哪片没做、下一步是什么"只看 [roadmap.md](./roadmap.md)。

## 更新纪律

1. **改了行为，同一次提交里改对应文档。** 文档和代码分两次提交，视为没做完。
2. **新增决定 → 在 `decisions.md` 追加一条，不改历史条目。** 要推翻旧决定，新写一条并注明「取代 D-x」；
   被取代的旧条目正文折叠进 `<details>`，只留一行废止标注。
3. **数值改动只改 `game/design.md`。** 别处引用时只写"见 design.md"，不抄数字。
4. **不要写"将来会……"。** 要么写进 `roadmap.md`，要么就别写。
