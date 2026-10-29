-- 初始七张表：联军 / 玩家 / 地区 / 地区建筑 / 库存 / 平台绑定 / 地区频道绑定
--
-- 约定（见 docs/data-model.md 第一节）：
--   1. 中文标识符必须双引号；
--   2. 时间列一律 BIGINT 存 Unix 秒，不用 TIMESTAMP；
--   3. 不建物理外键——联军会解散、地区会易主，级联语义在这里全是错的；
--   4. 枚举列存 TEXT，不建 PG enum 类型（加变体不需要 ALTER TYPE）。

CREATE TABLE "联军" (
    "id"         BIGSERIAL PRIMARY KEY,
    "名称"       TEXT    NOT NULL UNIQUE,
    "联军军队"   BIGINT  NOT NULL DEFAULT 0 CHECK ("联军军队" >= 0),
    "创建时间"   BIGINT  NOT NULL
);

CREATE TABLE "玩家" (
    "id"          BIGSERIAL PRIMARY KEY,
    "工人"        BIGINT NOT NULL DEFAULT 0 CHECK ("工人" >= 0),
    "私军"        BIGINT NOT NULL DEFAULT 0 CHECK ("私军" >= 0),
    "科技"        BIGINT NOT NULL DEFAULT 0 CHECK ("科技" >= 0),
    -- 可以为负：负值不能生产，这是"工资发不出"的惩罚状态
    "稳定度"      BIGINT NOT NULL DEFAULT 50,
    "工资"        BIGINT NOT NULL DEFAULT 0 CHECK ("工资" >= 0),
    -- 逻辑引用 "联军"."id"，NULL = 新玩家还没有联军
    "联军_id"     BIGINT,
    -- member / officer / leader
    "联军职务"    TEXT   NOT NULL DEFAULT 'member',
    -- Unix 秒；NULL = 已在保护期外
    "出生保护至"  BIGINT,
    "创建时间"    BIGINT NOT NULL
);

CREATE INDEX "玩家_联军_id" ON "玩家" ("联军_id");

CREATE TABLE "地区" (
    "id"              BIGSERIAL PRIMARY KEY,
    -- 对外访问凭证：6 位数字，保密。谁拿到它，谁就能打这个地区
    "uid"             INT    NOT NULL UNIQUE
                             CHECK ("uid" BETWEEN 100000 AND 999999),
    "名称"            TEXT   NOT NULL,
    -- 逻辑引用 "联军"."id"，NULL = 无主地区
    "控制方_联军_id"  BIGINT,
    "驻军"            BIGINT NOT NULL DEFAULT 0 CHECK ("驻军" >= 0),
    "创建时间"        BIGINT NOT NULL
);

CREATE INDEX "地区_控制方_联军_id" ON "地区" ("控制方_联军_id");

CREATE TABLE "地区建筑" (
    "地区_id"  BIGINT NOT NULL,
    -- 建筑编码（如 school）；清单待定，加一种建筑不需要迁移
    "建筑"     TEXT   NOT NULL,
    -- 完成度 0..=100：到阈值才解锁效果，v1 只增不减
    "完成度"   INT    NOT NULL DEFAULT 0
                      CHECK ("完成度" BETWEEN 0 AND 100),
    PRIMARY KEY ("地区_id", "建筑")
);

CREATE TABLE "库存" (
    -- player / coalition / region
    "所有者类型"  TEXT   NOT NULL,
    "所有者_id"   BIGINT NOT NULL,
    -- 带前缀的物品编码，如 resource:food、equipment:warship
    "物品"        TEXT   NOT NULL,
    "数量"        BIGINT NOT NULL DEFAULT 0 CHECK ("数量" >= 0),
    PRIMARY KEY ("所有者类型", "所有者_id", "物品")
);

CREATE TABLE "平台绑定" (
    "id"         BIGSERIAL PRIMARY KEY,
    "玩家_id"    BIGINT NOT NULL,
    -- onebot / discord / telegram
    "平台"       TEXT   NOT NULL,
    "平台_uid"   TEXT   NOT NULL,
    "绑定时间"   BIGINT NOT NULL,
    -- 一个平台账号只能绑一个玩家
    UNIQUE ("平台", "平台_uid")
);

CREATE INDEX "平台绑定_玩家_id" ON "平台绑定" ("玩家_id");

CREATE TABLE "地区频道绑定" (
    "id"         BIGSERIAL PRIMARY KEY,
    "地区_id"    BIGINT NOT NULL,
    "平台"       TEXT   NOT NULL,
    -- 群 / 频道 id
    "频道_id"    TEXT   NOT NULL,
    "绑定时间"   BIGINT NOT NULL,
    -- 一个频道只能属于一个地区；反向不唯一，一个地区可以绑多个频道
    UNIQUE ("平台", "频道_id")
);

CREATE INDEX "地区频道绑定_地区_id" ON "地区频道绑定" ("地区_id");
