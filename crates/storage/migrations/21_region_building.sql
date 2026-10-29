-- 地区建筑：建筑挂地区下，跟着地区易主。
-- 一种建筑在一个地区只有一份，进度用完成度表示，到阈值才解锁效果。

CREATE TABLE 地区建筑 (
    地区_id BIGINT NOT NULL,
    建筑 TEXT NOT NULL,
    完成度 百分比,
    PRIMARY KEY (地区_id, 建筑)
);

COMMENT ON TABLE 地区建筑 IS '一种建筑一个地区一份；完成后跟着地区走，谁控制地区谁用';
COMMENT ON COLUMN 地区建筑.建筑 IS '建筑编码（如 school）；清单待定，加一种建筑不需要迁移';
COMMENT ON COLUMN 地区建筑.完成度 IS '到阈值才解锁效果（原版：学校 60%、炼钢厂 70%）；v1 只增不减';
