-- 联军：玩家组织。新玩家可以没有联军，所以 玩家.联军_id 可空。
-- 不建物理外键：联军的级联语义（解散时成员怎么办）由应用层决定。
--
-- 照 v2 的 CoalitionArmy 逐字段过了一遍（docs/decisions.md D34–D38），改造点：
--   1. 成员列表、地区列表是**派生值**（COUNT / WHERE），不落列；v2 存它们是因为文档库没有 join。
--      **宣称人口 / 宣称兵力是玩家自填的公开数字，必须落列**；实际人口 / 实际兵力是
--      派生值（成员的 工人 之和 / 联军军队），不落列（D38）；
--   2. JSON 数组一律拆成表：`联军资本增量`（日结历史）、`联军分配记录`；
--   3. 元首与总理不存 uid 字符串，落在 `玩家.联军职务` 上（一人一职，唯一性由部分唯一索引保证）；
--   4. uid 是**公开**编号（显示 C123456），只用于查询与展示，不授予任何权限。

CREATE TABLE 联军 (
    id BIGSERIAL PRIMARY KEY,
    uid 六位编号 UNIQUE,
    名称 TEXT NOT NULL UNIQUE,
    名称是否审核 BOOLEAN NOT NULL DEFAULT TRUE,
    政治体制 TEXT NOT NULL DEFAULT 'democracy',
    首都_地区_id BIGINT,
    宣称人口 非负计数,
    宣称兵力 非负计数,
    联军军队 非负计数,
    扩军计划 非负计数,
    当天扩军累计 非负计数,
    税率 百分比,
    当天资本增量 非负计数,
    上次改名时间 可空时间戳,
    创建时间 时间戳
);

COMMENT ON TABLE 联军 IS '玩家组织，玩家可以没有';
COMMENT ON COLUMN 联军.uid IS '对外编号（显示为 C123456，落库只存 6 位数字）：公开，用于查询与展示，不授予任何权限';
COMMENT ON COLUMN 联军.名称是否审核 IS 'v1 没有审核后台，建联军时写 TRUE（视为已通过）；接审核后改成默认 FALSE，已有行不用迁移';
COMMENT ON COLUMN 联军.政治体制 IS 'democracy / authoritarian / totalitarian；决定每个动作的默认权限阈值，阈值表随指令一起补（D37）';
COMMENT ON COLUMN 联军.首都_地区_id IS '逻辑引用 地区.id；不变量：必须是本联军控制的地区，NULL = 还没设首都';
COMMENT ON COLUMN 联军.宣称人口 IS '对外**公开**的人口数字，玩家自己填；实际人口 = 成员的 工人 之和，只对成员可见';
COMMENT ON COLUMN 联军.宣称兵力 IS '对外**公开**的兵力数字，玩家自己填；实际兵力 = 联军军队，只对成员可见';
COMMENT ON COLUMN 联军.联军军队 IS '联军共享兵员，也是**实际兵力**（只对成员可见）：空袭的目标，驻军的唯一来源';
COMMENT ON COLUMN 联军.扩军计划 IS '每天允许转入联军军队的兵力上限；0 = 未设置（禁止转入），由元首设置';
COMMENT ON COLUMN 联军.当天扩军累计 IS '今天已经转入多少兵；每天 0 点由全服定时任务清零';
COMMENT ON COLUMN 联军.税率 IS '成员生产利润里上缴联军的比例（0–100）；上缴的部分进联军仓库（生活资料），成员拿剩下的';
COMMENT ON COLUMN 联军.当天资本增量 IS '今天成员生产利润的合计（生活资料计）；每天 0 点日结时落一行到 联军资本增量 再清零';
COMMENT ON COLUMN 联军.上次改名时间 IS '改名冷却与审核的锚点；NULL = 建好还没改过';
