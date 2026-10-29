-- 库存：玩家背包 / 联军仓库 / 地区仓库共用一张表，靠"所有者"区分。
-- 掠夺就是改两行库存，不管物资搬的是哪种所有者。

CREATE TABLE 库存 (
    所有者类型 TEXT NOT NULL,
    所有者_id BIGINT NOT NULL,
    物品 TEXT NOT NULL,
    数量 非负计数,
    PRIMARY KEY (所有者类型, 所有者_id, 物品)
);

COMMENT ON TABLE 库存 IS '所有物品的存放处；所有者类型 + 所有者_id 决定这是谁的库存';
COMMENT ON COLUMN 库存.所有者类型 IS 'player / coalition / region';
COMMENT ON COLUMN 库存.物品 IS '带前缀的物品编码：resource:food、resource:money、equipment:warship …';
