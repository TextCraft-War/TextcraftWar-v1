-- 地区频道绑定：频道 -> 地区。一个频道只能属于一个地区，一个地区可以绑多个频道（跨平台）。

CREATE TABLE 地区频道绑定 (
    id BIGSERIAL PRIMARY KEY,
    地区_id BIGINT NOT NULL,
    平台 TEXT NOT NULL,
    频道_id TEXT NOT NULL,
    绑定时间 时间戳,
    UNIQUE (平台, 频道_id)
);

CREATE INDEX 地区频道绑定_地区_id ON 地区频道绑定 (地区_id);

COMMENT ON TABLE 地区频道绑定 IS '频道是玩家的入口：同一地区可以同时绑 QQ 群、Discord 频道、TG 群';
COMMENT ON COLUMN 地区频道绑定.频道_id IS '群 / 频道 id，原样存字符串';
