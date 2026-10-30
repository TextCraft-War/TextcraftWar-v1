-- 玩家：指挥官的全部可变状态。
--
-- 照 v2 的 Player 类型逐字段过了一遍（docs/decisions.md D28/D31/D33），改造点：
--   1. 物资全部在 库存（所有者类型 = player），不做成列——加一种资源不用 ALTER TABLE；
--   2. 日期一律 Unix 秒，不用字符串（v2 有 `上次驻扎日期: string`，那是坑）；
--   3. 生产次数和今日招募数量由全服定时任务推进（D32），所以**不存"上次结算时间"**：
--      每小时 +1、每天清零，进度记在 系统状态 那张单行表里，漏跑可补算；
--   4. uid 是秘密凭证：不知道 P uid 就找不到这个人，和地区的 R uid 同一套规则（D30）；
--   5. 联军的职务与权限等级都有，但两者不是一回事：职务是位置（元首/总理在**三级之上**），
--      权限等级是 1–3 的放行档位；元首/总理不要等级，干部/成员必须有等级。
--      三个字段同生共死，怎么写由 CHECK 说清楚（D37）。

CREATE TABLE 玩家 (
    id BIGSERIAL PRIMARY KEY,
    uid 六位编号 UNIQUE,

    -- 人口
    工人 非负计数,
    私军 非负计数,
    地下工人 非负计数,
    休假工人 非负计数,

    -- 生产
    科技 非负计数,
    生产技术 非负计数,
    厂房 非负计数,
    地下工厂投入 非负计数,
    科技池投入 非负计数,
    科技池容量 非负计数,
    生产次数 非负计数,
    军事行动点 非负计数,
    稳定度 BIGINT NOT NULL DEFAULT 50,
    工资 非负计数,

    -- 节奏
    上次签到时间 可空时间戳,
    今日招募数量 非负计数,
    上次炮击时间 可空时间戳,
    上次驻扎时间 可空时间戳,

    -- 位置与组织
    驻扎地区_id BIGINT,
    联军_id BIGINT,
    联军职务 TEXT,
    联军权限等级 权限等级,
    联军贡献 非负计数,
    加入联军时间 可空时间戳,
    出生保护至 可空时间戳,
    创建时间 时间戳,

    CONSTRAINT 联军职务与等级一致 CHECK (
        (联军_id IS NULL AND 联军职务 IS NULL AND 联军权限等级 IS NULL)
        OR (联军_id IS NOT NULL AND 联军职务 IN ('leader', 'premier') AND 联军权限等级 IS NULL)
        OR (联军_id IS NOT NULL AND 联军职务 IN ('officer', 'member') AND 联军权限等级 IS NOT NULL)
    )
);

CREATE INDEX 玩家_联军_id ON 玩家 (联军_id);
CREATE INDEX 玩家_驻扎地区_id ON 玩家 (驻扎地区_id);
CREATE UNIQUE INDEX 玩家_联军元首唯一 ON 玩家 (联军_id) WHERE 联军职务 = 'leader';
CREATE UNIQUE INDEX 玩家_联军总理唯一 ON 玩家 (联军_id) WHERE 联军职务 = 'premier';

COMMENT ON TABLE 玩家 IS '指挥官（跨平台身份）。工人 + 私军 + 地下工人 + 休假工人 = 总人口，扩军是 1:1 的转化';
COMMENT ON COLUMN 玩家.uid IS '秘密凭证（显示为 P123456，落库只存 6 位数字）：知道它才能跨群找到这个人';
COMMENT ON COLUMN 玩家.私军 IS '操作兵器的兵员；阵亡即人口永久损失';
COMMENT ON COLUMN 玩家.地下工人 IS '分配在地下工厂的工人：空袭炸不到，产量另算';
COMMENT ON COLUMN 玩家.休假工人 IS '暂时不生产的工人；投放病毒会把工厂工人和休假工人一起杀掉';
COMMENT ON COLUMN 玩家.生产技术 IS '产量乘数；原版"改进生产技术"，消耗生活资料升级';
COMMENT ON COLUMN 玩家.厂房 IS '产能上限；原版"扩建厂房"，消耗钢铁';
COMMENT ON COLUMN 玩家.地下工厂投入 IS '地下工厂的累计投入；够阈值才有地下产能（阈值在 domain 里）';
COMMENT ON COLUMN 玩家.科技池投入 IS '投进科技池的资源，攒满容量换一级科技';
COMMENT ON COLUMN 玩家.科技池容量 IS '科技池的容量上限';
COMMENT ON COLUMN 玩家.生产次数 IS '可积攒的生产次数：每小时 +1、上限 8，没有次数不能生产（由全服定时任务推进）';
COMMENT ON COLUMN 玩家.军事行动点 IS '发动攻击/枚举地区要花；怎么恢复待定';
COMMENT ON COLUMN 玩家.稳定度 IS '可以为负；负值不能生产（发不出工资的惩罚状态）';
COMMENT ON COLUMN 玩家.上次签到时间 IS 'NULL = 从没签到过；是不是"今天签过了"由时间差算，不存布尔';
COMMENT ON COLUMN 玩家.今日招募数量 IS '今天已经招了多少；每天 0 点由全服定时任务清零';
COMMENT ON COLUMN 玩家.上次炮击时间 IS '炮击冷却；具体间隔待定';
COMMENT ON COLUMN 玩家.上次驻扎时间 IS '驻扎冷却的锚点：防止刚撤走就立刻驻扎到别处';
COMMENT ON COLUMN 玩家.驻扎地区_id IS '逻辑引用 地区.id；NULL = 没驻扎。地区易主不影响它（弱引用）';
COMMENT ON COLUMN 玩家.联军_id IS '逻辑引用 联军.id；NULL = 还没有联军';
COMMENT ON COLUMN 玩家.联军职务 IS 'leader 元首 / premier 总理 / officer 干部 / member 成员；NULL = 没有联军，与 联军_id 同生共死（CHECK 保证）';
COMMENT ON COLUMN 玩家.联军权限等级 IS '1–3 级权限：能执行哪些动作由 (政体, 动作) 的阈值决定；元首/总理在等级之上，所以为 NULL（D37）';
COMMENT ON COLUMN 玩家.联军贡献 IS '对当前联军的贡献；入盟时清零，退盟后无意义';
COMMENT ON COLUMN 玩家.加入联军时间 IS 'NULL = 没入盟';
COMMENT ON COLUMN 玩家.出生保护至 IS 'Unix 秒；NULL = 已出保护期';
