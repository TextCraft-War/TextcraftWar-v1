-- 联军：玩家组织。新玩家可以没有联军，所以 玩家.联军_id 可空。
-- 不建物理外键：联军的级联语义（解散时成员怎么办）由应用层决定。

CREATE TABLE 联军 (
    id BIGSERIAL PRIMARY KEY,
    名称 TEXT NOT NULL UNIQUE,
    联军军队 非负计数,
    创建时间 时间戳
);

COMMENT ON TABLE 联军 IS '玩家组织，玩家可以没有';
COMMENT ON COLUMN 联军.联军军队 IS '联军共享兵员：空袭的目标，也是驻军的唯一来源';
