# 测试指南

操作手册：怎么跑测试、怎么备测试库。分层与测试策略见 `crates/*/docs/impl.md`。

## 一、测试矩阵

| 层 | 位置 | 依赖 | 说明 |
|---|---|---|---|
| `domain` 单测 | `crates/domain/src/*` | **无**（零依赖是硬边界） | 纯规则函数，每个分支一条用例 |
| `protocol` 单测 | `crates/protocol/src/lib.rs` | serde_json | 固定 JSON 字符串断言 wire 形状 |
| `storage` 单测 | `crates/storage/src/*` | 无 | 迁移内嵌、版本号契约、行转换（不连库） |
| `server` 单测 | `crates/server/src/*` | 无 | 解析、文案、错误映射 |
| `server` 集成 | `crates/server/tests/register.rs` | **真 PostgreSQL** | 注册端到端：真事务、真 SQL |

## 二、跑全量单测（无需数据库）

```bash
cargo test --workspace
```

在没有装 PostgreSQL 的机器（新克隆、CI）上也必须全绿——这是刻意的：
`sqlx` 用运行期校验（`query_as`），不依赖构建期连库。

## 三、集成测试：测试库准备

`register.rs` 的用例标了 `#[ignore]`，要显式指定测试库才会跑。
**必须用专用测试库**：测试会建地区、绑频道、删自己用的账号，指向生产库等于自毁。

```bash
# 1. 建一个一次性测试库（迁移会在连上时自动跑齐）
psql -c "CREATE DATABASE textcraft_test"

# 2. 设 DATABASE_URL 后跑集成用例
export DATABASE_URL="postgres://<用户>@127.0.0.1:5432/textcraft_test"
cargo test -p server --test register -- --ignored
```

Windows 本机开发约定（示例）：PG 18 实例跑在 **55400** 端口的集群上，
`DATABASE_URL="postgres://postgres@127.0.0.1:55400/textcraft_test"`。

## 四、CI 防呆

- `DATABASE_URL` 未设置时，集成用例**跳过但打印警告**，不假装通过。
- 在 `CI=true` 的环境里，缺测试库会**直接失败**——CI 上"静默跳过"等于假绿灯，
  绿了也等于什么都没测。

## 五、SQL 与中文的小坑（Windows）

- 往 psql 传带中文的 SQL，走 `psql -f 文件.sql`，不要走 `-c "中文"`——
  Windows 控制台的管道编码会把中文捣碎。`.scratch/verify*.sql` 是现成的对账脚本样式。
- 迁移与查询里的中文标识符**不写双引号**（见 [arch/data-model.md 第一节](../arch/data-model.md)）。

## 六、依赖与格式门

```bash
cargo fmt --check          # 格式
cargo clippy --workspace --all-targets -- -D warnings   # 严格 lint（pedantic/nursery 也算失败）
cargo deny check           # 依赖来源与许可证（仅 crates.io）
```

提交前四件事：`fmt` / `clippy-strict` / `test` / 改了行为则同提交改文档。
