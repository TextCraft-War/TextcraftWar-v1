-- 地区：频道组 + 资源容器。不会消失，只会易主，所以没有任何删除级联。

CREATE TABLE 地区 (
    id BIGSERIAL PRIMARY KEY,
    uid 六位编号 UNIQUE,
    名称 TEXT NOT NULL,
    控制方_联军_id BIGINT,
    驻军 非负计数,
    创建时间 时间戳
);

CREATE INDEX 地区_控制方_联军_id ON 地区 (控制方_联军_id);

COMMENT ON TABLE 地区 IS '没有地理属性：所有地区在空间上是同一个点，只靠 uid 和绑定的频道区分';
COMMENT ON COLUMN 地区.uid IS '对外访问凭证（显示为 R123456，落库只存 6 位数字）：不知道它就打不到这个地区，不得出现在任何公开输出里';
COMMENT ON COLUMN 地区.控制方_联军_id IS '逻辑引用 联军.id；NULL = 无主地区。地区只能被联军控制，不能属于个人';
COMMENT ON COLUMN 地区.驻军 IS '从联军军队转化而来；驻军 > 0 ⇔ 控制方 IS NOT NULL';
