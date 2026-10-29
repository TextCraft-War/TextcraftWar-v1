-- 玩家：跨平台身份。资源与装备不在这张表，都在 库存（所有者类型 = player）。

CREATE TABLE 玩家 (
    id BIGSERIAL PRIMARY KEY,
    工人 非负计数,
    私军 非负计数,
    科技 非负计数,
    稳定度 BIGINT NOT NULL DEFAULT 50,
    工资 非负计数,
    联军_id BIGINT,
    联军职务 TEXT NOT NULL DEFAULT 'member',
    出生保护至 BIGINT,
    创建时间 时间戳
);

CREATE INDEX 玩家_联军_id ON 玩家 (联军_id);

COMMENT ON TABLE 玩家 IS '指挥官（跨平台身份）。工人 + 私军 = 总人口，扩军是 1:1 的转化';
COMMENT ON COLUMN 玩家.私军 IS '操作兵器的兵员；阵亡即人口永久损失';
COMMENT ON COLUMN 玩家.稳定度 IS '可以为负；负值不能生产（发不出工资的惩罚状态）';
COMMENT ON COLUMN 玩家.联军_id IS '逻辑引用 联军.id；NULL = 还没有联军';
COMMENT ON COLUMN 玩家.联军职务 IS 'member / officer / leader；没有联军时无意义';
COMMENT ON COLUMN 玩家.出生保护至 IS 'Unix 秒；NULL = 已出保护期';
