-- 平台绑定：平台账号 -> 玩家。一个玩家可以绑多个平台账号，反之不行。

CREATE TABLE 平台绑定 (
    id BIGSERIAL PRIMARY KEY,
    玩家_id BIGINT NOT NULL,
    平台 TEXT NOT NULL,
    平台_uid TEXT NOT NULL,
    绑定时间 时间戳,
    UNIQUE (平台, 平台_uid)
);

CREATE INDEX 平台绑定_玩家_id ON 平台绑定 (玩家_id);

COMMENT ON TABLE 平台绑定 IS '一个平台账号只能绑一个玩家；一个玩家可以绑多个平台账号';
COMMENT ON COLUMN 平台绑定.平台 IS 'onebot / discord / telegram';
COMMENT ON COLUMN 平台绑定.平台_uid IS '平台侧的用户 id，原样存字符串';
