# storage — 持久化层

> 状态：草案（尚无实现）｜ 最后更新：2026-10-29
>
> 规则见 [design.md](../../../docs/design.md)；分层见 [architecture.md](../../../docs/architecture.md)。

## 一、这个 crate 是什么

两件事，只做这两件：

1. 写 SQL。
2. 把数据库行翻译成 `domain` 的类型，以及反过来。

**不写规则，不开事务。**

## 二、表结构（草案，待定稿）

命名一律中文，SQL 里必须加双引号。

### `玩家`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | `BIGSERIAL PK` | |
| `工人` | `BIGINT` | 与私军争抢同一批人口 |
| `私军` | `BIGINT` | 操作兵器的兵员，整数 |
| `生活资料` | `BIGINT` | |
| `世界货币` | `BIGINT` | |
| `科技` | `BIGINT` | |
| `稳定度` | `BIGINT` | 低于 0 不能生产 |
| `工资` | `BIGINT` | |
| `联军_id` | `BIGINT NULL` | 逻辑外键，不建物理约束（联军会解散） |
| `出生保护至` | `BIGINT NULL` | Unix 秒，NULL = 已过保护 |
| `创建时间` | `BIGINT` | Unix 秒 |

### `联军`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | `BIGSERIAL PK` | |
| `名称` | `TEXT UNIQUE` | |
| `联军军队` | `BIGINT` | 共享兵员，也是驻军的唯一来源 |
| `生活资料` | `BIGINT` | 联军仓库 |
| `创建时间` | `BIGINT` | |

### `地区`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | `BIGSERIAL PK` | |
| `uid` | `INT UNIQUE` | 6 位数字，**对外访问凭证**，不可泄露 |
| `名称` | `TEXT` | |
| `控制方_联军_id` | `BIGINT NULL` | NULL = 无主 |
| `驻军` | `BIGINT` | 从联军军队转化而来 |
| `生活资料` | `BIGINT` | 地区仓库 |
| `世界货币` | `BIGINT` | 地区仓库 |
| `创建时间` | `BIGINT` | |

### `平台绑定`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | `BIGSERIAL PK` | |
| `玩家_id` | `BIGINT` | |
| `平台` | `TEXT` | `onebot` / `discord` / `telegram` |
| `平台_uid` | `TEXT` | |
| `绑定时间` | `BIGINT` | |

约束：`UNIQUE (平台, 平台_uid)` —— 一个平台账号只能绑一个玩家。

### `地区频道绑定`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | `BIGSERIAL PK` | |
| `地区_id` | `BIGINT` | |
| `平台` | `TEXT` | |
| `频道_id` | `TEXT` | |
| `绑定时间` | `BIGINT` | |

约束：`UNIQUE (平台, 频道_id)` —— 一个频道只能属于一个地区。
反向不唯一：一个地区可以绑多个频道（跨平台共享）。

### `玩家装备`

| 列 | 类型 | 说明 |
|---|---|---|
| `玩家_id` | `BIGINT` | |
| `装备` | `TEXT` | 装备名，与 `domain` 的定义对齐 |
| `数量` | `BIGINT` | |

主键：`(玩家_id, 装备)`。

> 为什么装备单独一张表而不是塞进 `玩家`：装备种类会持续增加，
> 加一种装备不应该等于 `ALTER TABLE`。

## 三、三条实现约定

### 3.1 不开事务

每个函数签名形如：

```rust
pub async fn 查玩家<'e>(
    执行器: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    玩家_id: i64,
) -> Result<Option<玩家行>, 错误>
```

传 `&PgPool` 还是 `&mut Transaction` 由 `server` 决定。本 crate 里**不出现** `begin()`。

### 3.2 行类型与 domain 类型分开

- `玩家行`：`#[derive(sqlx::FromRow)]`，字段与列一一对应，只在 `storage` 内部可见。
- `domain::玩家状态`：领域模型。
- 两者之间的转换写成 `impl TryFrom<玩家行> for 玩家状态`。

绝不让 `domain` 的类型带 `FromRow`，也绝不让 `玩家行` 带 `Serialize`。

### 3.3 枚举列存 TEXT

不用 PG 的 `enum` 类型。读出为 `String`，在 `storage` 里 `TryFrom` 成 domain 的枚举。
这样加枚举变体不需要 `ALTER TYPE`。

## 四、迁移

- 位置：`crates/storage/migrations/`
- 命名：`0001_初始表.sql`、`0002_xxx.sql`
- 通过 `sqlx::migrate!("./migrations")` 编译期内嵌，启动时自动执行。
- 约束：**迁移只增不改**。已提交的迁移文件永远不修改，改结构就加新的。

## 五、还没做的事

- [ ] 迁移文件
- [ ] 行类型与 `FromRow` 派生
- [ ] 五个查询函数（查玩家 / 建玩家 / 改玩家 / 查地区 / 查频道→地区）
- [ ] 索引：`玩家.联军_id`、`地区.控制方_联军_id`
