-- 基础语义类型。所有表都用这几个 DOMAIN，不再逐列重复 NOT NULL / DEFAULT / CHECK。
--
-- 风格约定（见 crates/storage/docs/design.md）：
--   1. 中文标识符不写双引号——PG 只折叠 ASCII 大写字母，中文原样保留；
--   2. 不凑列对齐，字段名、类型、约束之间只有一个空格；
--   3. 字段语义用 COMMENT ON 写进数据库目录，不塞在字段列表中间；
--   4. 一次迁移只做一件事，已提交的迁移文件永不修改（未部署前可就地改，见 D24）。

CREATE DOMAIN 非负计数 AS BIGINT NOT NULL DEFAULT 0 CHECK (VALUE >= 0);
CREATE DOMAIN 时间戳 AS BIGINT NOT NULL;
CREATE DOMAIN 可空时间戳 AS BIGINT;
CREATE DOMAIN 百分比 AS INT NOT NULL DEFAULT 0 CHECK (VALUE BETWEEN 0 AND 100);
CREATE DOMAIN 六位编号 AS INT NOT NULL CHECK (VALUE BETWEEN 100000 AND 999999);

COMMENT ON DOMAIN 非负计数 IS '非负整数：工人、军队、物品数量等一切"不可能为负"的计数';
COMMENT ON DOMAIN 时间戳 IS 'Unix 秒。不用 TIMESTAMP：跨时区、跨平台，整数最省事';
COMMENT ON DOMAIN 可空时间戳 IS 'Unix 秒，NULL 表示"还没发生过"（如从没签到）';
COMMENT ON DOMAIN 百分比 IS '0..=100 的完成度';
COMMENT ON DOMAIN 六位编号 IS '6 位随机数，落库只存数字：玩家 uid 显示为 P123456，地区 uid 显示为 R123456。两者都是秘密';
