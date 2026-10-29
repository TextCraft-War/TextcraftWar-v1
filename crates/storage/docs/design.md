# storage — 持久化层

> 状态：草案（尚无实现）｜ 最后更新：2026-10-29
>
> **表结构与领域类型不在这里定义**，见 [docs/data-model.md](../../../docs/data-model.md)。
> 本文只写"怎么落地"。

## 一、这个 crate 是什么

两件事，只做这两件：

1. 写 SQL。
2. 把数据库行翻译成 `domain` 的类型，以及反过来。

**不写规则，不开事务。**

## 二、实现约定

### 2.1 不开事务

每个函数签名形如：

```rust
pub async fn 查玩家<'e>(
    执行器: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    玩家_id: 玩家Id,
) -> Result<Option<玩家行>, 错误>
```

传 `&PgPool` 还是 `&mut Transaction` 由 `server` 决定。本 crate 里**不出现** `begin()`。

### 2.2 行类型与领域类型分开

- `玩家行`：`#[derive(sqlx::FromRow)]`，字段与列一一对应，只在 `storage` 内部可见。
- `domain::玩家`：领域模型。
- 两者之间的转换写成 `TryFrom<玩家行> for 玩家`。

绝不让领域类型带 `FromRow`，也绝不让行类型带 `Serialize`。

### 2.3 中文字段与中文列名

已实测可行（见 [decisions.md D10](../../../docs/decisions.md)）：

- `#[derive(sqlx::FromRow)]` 吃中文字段名 ✓
- 中文列名在 SQL 里必须双引号（`"昵称"`）✓

### 2.4 条件更新

并发安全靠"旧值作为条件"，不靠锁：

```sql
UPDATE "玩家" SET "私军" = $1 WHERE "id" = $2 AND "私军" = $3
```

影响行数为 0 表示有人先改了 → 返回 `错误::并发冲突`，由 `server` 重读重算。

## 三、迁移

- 位置：`crates/storage/migrations/`
- 命名：`0001_初始表.sql`、`0002_xxx.sql`
- 通过 `sqlx::migrate!("./migrations")` 编译期内嵌，启动时自动执行
- 约束：**迁移只增不改**。已提交的迁移文件永远不修改，改结构就加新的

## 四、需要实现的函数（按 roadmap 顺序）

| 函数 | 用于 | 切片 |
|---|---|---|
| `查玩家(平台, 平台_uid)` | 登录/注册 | S2 |
| `建玩家` | 注册 | S2 |
| `查地区(频道)` | 路由 | S2 |
| `建地区 + 绑频道` | 自动注册 | S2 |
| `查玩家详情(id)` | `我的资料` | S3 |
| `改玩家资源` | 签到/生产 | S4 |
| `查装备库存` / `改装备库存` | 军备 | S5 |

## 五、还没做的事

- [ ] 迁移文件
- [ ] 行类型与 `FromRow` 派生
- [ ] 上表里的查询函数
