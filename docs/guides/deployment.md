# 部署指南

操作手册：怎么把服务器跑起来。设计理由见 [arch/overview.md](../arch/overview.md)，
启动序列的实现细节见 `crates/server/docs/impl.md` 第二节。

## 一、前置条件

- **Rust 工具链**：由 [rust-toolchain.toml](../../rust-toolchain.toml) 锁定（1.97.0），`rustup` 会自动装。
- **PostgreSQL**：用户自备实例（v1 不捆绑数据库）。服务器启动时自动跑迁移建表，
  **数据库本身要先建好**：

  ```sql
  CREATE DATABASE textcraft;
  ```

- **种子数据**：v1 没有建地区 / 绑频道的指令，新部署必须先手工播种地区与频道绑定，
  否则所有频道都回 `no_region`、无法注册。最小种子：

  ```sql
  -- 一个地区（R 编号随机取一个未用的六位数）
  INSERT INTO 地区 (uid, 名称, 创建时间) VALUES (990001, '新伊甸', <Unix 秒>);
  -- 把群绑到地区
  INSERT INTO 地区频道绑定 (地区_id, 平台, 频道_id, 绑定时间)
      SELECT id, 'onebot', '<群号>', <Unix 秒> FROM 地区 WHERE uid = 990001;
  ```

## 二、环境变量

| 变量 | 必填 | 说明 |
|---|---|---|
| `DATABASE_URL` | 是 | PostgreSQL 连接串，如 `postgres://用户:密码@主机:5432/textcraft` |
| `WS_TOKEN` | 是 | 握手共享密钥，与 Koishi 插件侧配置一致 |
| `BIND_ADDR` | 否 | 监听地址，默认 `0.0.0.0:8080` |
| `RUST_LOG` | 否 | `tracing` 过滤，缺省 `info`（如 `RUST_LOG=debug` 看被拒指令） |

## 三、启动序列

```text
读环境变量 → 建连接池 → 跑迁移（自动建表/升级）→ 绑定端口对外服务
```

迁移在绑定端口**之前**执行：库连不上或迁移失败就启动失败，
不会出现"端口开着但所有指令都报错"的半死状态。

```bash
# 开发
cargo run

# 生产（Release 配置：opt-level=3 + fat LTO + panic=abort + strip）
cargo build --release
```

## 四、对接 Koishi

- 端点：`ws://<host>:<port>/ws`，文本帧，每帧一个 JSON。
- 握手、消息形状与错误码见 [arch/protocol.md](../arch/protocol.md)——**跨仓库契约**，
  改动必须同时知会 Koishi 插件侧。
- Koishi 侧对接插件在另一个仓库，本项目只暴露 WS 契约。

## 五、验证部署

```bash
# 建连后应收到 ready
# （.NET / PowerShell 的话 .scratch/ws_test.ps1 是现成的握手冒烟脚本）
```

起来之后看日志应出现：`数据库已连接，迁移已执行` 与 `Textcraft: War 服务器启动`。
