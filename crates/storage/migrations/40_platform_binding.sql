-- 平台绑定：平台账号 -> 玩家。
-- 注册时写入第一条绑定；之后可以把"另一个还没注册过的平台账号"挂到同一个玩家名下，
-- 三步：绑定账户（A 平台出码）→ 确认绑定（B 平台输码）→ 最终同意（A 平台确认）。
-- 绑定不是合并：已经有玩家的平台账号不能绑（见 docs/decisions.md D23）。
--
-- 两条唯一约束顺带提供了两个方向的索引：(平台, 平台_uid) 用于登录，
-- (玩家_id, 平台) 用于列出一个玩家的全部身份，所以不额外建索引。

CREATE TABLE 平台绑定 (
    id BIGSERIAL PRIMARY KEY,
    玩家_id BIGINT NOT NULL,
    平台 TEXT NOT NULL,
    平台_uid TEXT NOT NULL,
    绑定时间 时间戳,
    UNIQUE (平台, 平台_uid),
    UNIQUE (玩家_id, 平台)
);

COMMENT ON TABLE 平台绑定 IS '一个平台账号只属于一个玩家；一个玩家在同一个平台只有一个账号';
COMMENT ON COLUMN 平台绑定.玩家_id IS '逻辑引用 玩家.id';
COMMENT ON COLUMN 平台绑定.平台 IS 'onebot / discord / telegram；不认识的平台在 storage 里被拒绝';
COMMENT ON COLUMN 平台绑定.平台_uid IS '平台侧的用户 id，原样存字符串';
