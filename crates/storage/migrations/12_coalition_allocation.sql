-- 联军分配记录：谁把什么、多少、什么时候给了谁。
--
-- v2 把它塞在 联军.生活资料分配记录 这个 JSON 数组里；数组一旦长了，就没法分页、
-- 没法按接收方索引、也没法在事务里安全地"追加一行"。拆表之后：
--   `分配记录` 是一条 `ORDER BY 时间 DESC LIMIT 20` 的查询，不是一次全表读改写。
-- 物品存编码（`resource:food` 这种），不存中文名——理由见 catalog.md。

CREATE TABLE 联军分配记录 (
    id BIGSERIAL PRIMARY KEY,
    联军_id BIGINT NOT NULL,
    分配者_玩家_id BIGINT NOT NULL,
    接收方_玩家_id BIGINT NOT NULL,
    物品 TEXT NOT NULL,
    数量 非负计数,
    时间 时间戳
);

CREATE INDEX 联军分配记录_联军_id_时间 ON 联军分配记录 (联军_id, 时间 DESC);

COMMENT ON TABLE 联军分配记录 IS '联军的物资分配流水；`分配历史记录` 指令读它';
COMMENT ON COLUMN 联军分配记录.联军_id IS '逻辑引用 联军.id';
COMMENT ON COLUMN 联军分配记录.分配者_玩家_id IS '逻辑引用 玩家.id：谁批的这笔';
COMMENT ON COLUMN 联军分配记录.接收方_玩家_id IS '逻辑引用 玩家.id：谁收的这笔';
COMMENT ON COLUMN 联军分配记录.物品 IS '物品编码，如 resource:food；不加 CHECK，加一种物品不改表';
COMMENT ON COLUMN 联军分配记录.时间 IS '分配发生的时刻';
