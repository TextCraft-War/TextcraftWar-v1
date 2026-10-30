-- 玩家：指挥官的全部可变状态。
--
-- 照 v2 的 Player 类型逐字段过了一遍（docs/decisions.md D28），做了三处改造：
--   1. 物资全部在 库存（所有者类型 = player），不做成列——加一种资源不用 ALTER TABLE；
--   2. "今日是否签到"/"小时是否生产" 这类布尔改成"上次签到时间"/"上次生产时间"，
--      判定变成 "now - 上次 >= 周期"，不需要定时任务去重置，也没有时区问题；
--   3. 日期一律 Unix 秒，不用字符串（v2 有 `上次驻扎日期: string`，那是坑）。

CREATE TABLE 玩家 (
    id BIGSERIAL PRIMARY KEY,

    -- 人口与生产
    工人 非负计数,
    私军 非负计数,
    科技 非负计数,
    生产技术 非负计数,
    厂房 非负计数,
    稳定度 BIGINT NOT NULL DEFAULT 50,
    工资 非负计数,

    -- 节奏（都是"上次发生的时间"，用时间差判断，不存布尔）
    上次签到时间 可空时间戳,
    上次生产时间 可空时间戳,
    上次招募时间 可空时间戳,
    上次炮击时间 可空时间戳,

    -- 位置与组织
    驻扎地区_id BIGINT,
    联军_id BIGINT,
    联军职务 TEXT NOT NULL DEFAULT 'member',
    出生保护至 可空时间戳,
    创建时间 时间戳
);

CREATE INDEX 玩家_联军_id ON 玩家 (联军_id);
CREATE INDEX 玩家_驻扎地区_id ON 玩家 (驻扎地区_id);

COMMENT ON TABLE 玩家 IS '指挥官（跨平台身份）。工人 + 私军 = 总人口，扩军是 1:1 的转化';
COMMENT ON COLUMN 玩家.私军 IS '操作兵器的兵员；阵亡即人口永久损失';
COMMENT ON COLUMN 玩家.生产技术 IS '产量乘数；原版"改进生产技术"，消耗生活资料升级';
COMMENT ON COLUMN 玩家.厂房 IS '产能上限；原版"扩建厂房"，消耗钢铁';
COMMENT ON COLUMN 玩家.稳定度 IS '可以为负；负值不能生产（发不出工资的惩罚状态）';
COMMENT ON COLUMN 玩家.上次签到时间 IS 'NULL = 从没签到过；是不是"今天签过了"由时间差算，不存布尔';
COMMENT ON COLUMN 玩家.上次生产时间 IS '每小时一次的生产闸门，同上：靠时间差判断';
COMMENT ON COLUMN 玩家.上次招募时间 IS '每天一次招募的闸门；招募上限是常量，不是列';
COMMENT ON COLUMN 玩家.上次炮击时间 IS '炮击冷却；具体间隔待定';
COMMENT ON COLUMN 玩家.驻扎地区_id IS '逻辑引用 地区.id；NULL = 没驻扎。地区易主不影响它（弱引用）';
COMMENT ON COLUMN 玩家.联军_id IS '逻辑引用 联军.id；NULL = 还没有联军';
COMMENT ON COLUMN 玩家.联军职务 IS 'member / officer / leader；没有联军时无意义';
COMMENT ON COLUMN 玩家.出生保护至 IS 'Unix 秒；NULL = 已出保护期';
