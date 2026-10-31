//! 领域实体与词表：表结构的 Rust 镜像。
//!
//! **类型定义的唯一事实来源是 [`docs/arch/data-model.md`] 第五节**——本文只落代码，
//! 不重复解释字段语义；两边不一致时，先怀疑本文写错。
//!
//! 词表（物品编码）的唯一事实来源是 `docs/game/catalog.md`：
//! 本文件用一张「变体 => 编码」配对表同时生成 `编码` / `解析` 两个方向，永不漂移。
//!
//! [`docs/arch/data-model.md`]: ../../../docs/arch/data-model.md

use std::collections::BTreeMap;
use std::fmt;

use crate::错误类型::错误;

/// 生成「变体 => 落库编码」成对的小枚举。
///
/// `编码` / `解析` / `全部` 由**同一份配对表**展开，两个方向不可能各自漂移；
/// 加一种编码只改一张表，落库值与解析自动跟齐（catalog「编码只增不改」）。
macro_rules! 编码枚举 {
    (
        $(#[$枚举文档:meta])*
        $名称:ident {
            $(
                $(#[$变体文档:meta])*
                $变体:ident => $编码:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$枚举文档])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $名称 {
            $(
                $(#[$变体文档])*
                $变体,
            )+
        }

        impl $名称 {
            /// 落库编码（catalog 的稳定键，只增不改）。
            pub const fn 编码(self) -> &'static str {
                match self {
                    $(Self::$变体 => $编码,)+
                }
            }

            /// 从落库编码解析；未知编码返回 `None`——调用方必须当**数据损坏**处理，
            /// 绝不 fallback 成某个默认值。
            pub fn 解析(原文: &str) -> Option<Self> {
                match 原文 {
                    $($编码 => Some(Self::$变体),)+
                    _ => None,
                }
            }

            /// 中文展示名（变体名的字符串形式，与 catalog 词条一致）。
            ///
            /// [`物品::编码`] 是落库的稳定键（面向数据库与日志）；
            /// 这里是给人看的词——玩家可见文本由 `server` 的 `文案` 模块组装。
            pub const fn 名称(self) -> &'static str {
                match self {
                    $(Self::$变体 => stringify!($变体),)+
                }
            }

            /// 全部变体，供对账与往返测试。
            pub const 全部: &'static [Self] = &[$(Self::$变体),+];
        }
    };
}

// ---------------------------------------------------------------------------
// 标识与时间（data-model 5.1）
// ---------------------------------------------------------------------------

/// 玩家主键（内部，join / 索引 / 日志用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 玩家Id(
    /// 内部主键值。
    pub i64,
);

/// 联军主键（内部）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 联军Id(
    /// 内部主键值。
    pub i64,
);

/// 地区主键（内部 id，不是对外的 uid）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 地区Id(
    /// 内部主键值。
    pub i64,
);

/// Unix 秒。所有"什么时候"都用它，不用 `chrono` / `TIMESTAMP`（D19 / D28）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 时间(
    /// Unix 秒值。
    pub i64,
);

/// 玩家对外的秘密编号，显示为 `P123456`（落库只存 6 位数字，前缀不进库）。
///
/// 和 [`玩家Id`] 是两个东西：`玩家Id` 是内部主键；`玩家Uid` 是凭证，
/// **不得进日志、不得出现在公开输出里**（D30）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 玩家Uid(
    /// 6 位编号值，`100000..=999999`。
    pub i32,
);

/// 地区对外的秘密编号，显示为 `R123456`。是"打这个地区"的凭证（D30）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 地区Uid(
    /// 6 位编号值，`100000..=999999`。
    pub i32,
);

/// 联军对外的**公开**编号，显示为 `C123456`。
///
/// 与上面两个不同：它不是凭证，出现在排行榜、战报、`我的联军` 里都没问题（D36）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 联军Uid(
    /// 6 位编号值，`100000..=999999`。
    pub i32,
);

/// 建筑编码，落库值（如 `school`，见 catalog 第三节）。
///
/// 清单定稿成 `建筑` 枚举前，先留 newtype 挡住裸 `String`；
/// 换枚举时 DB 里的 `TEXT` 值不变。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct 建筑编码(
    /// 落库编码。
    pub String,
);

// ---------------------------------------------------------------------------
// 分类枚举（catalog 第五～七节的落库值）
// ---------------------------------------------------------------------------

编码枚举! {
    /// 接入的聊天平台（身份模型的一部分，落 `平台绑定.平台` / `地区频道绑定.平台`）。
    ///
    /// 未知编码一律当错误处理，**不要** fallback 成某个默认平台。
    平台 {
        /// QQ（走 `OneBot` / Satori 适配器）。
        Onebot => "onebot",
        /// Discord。
        Discord => "discord",
        /// Telegram。
        Telegram => "telegram",
    }
}

impl fmt::Display for 平台 {
    fn fmt(&self, 格式: &mut fmt::Formatter<'_>) -> fmt::Result {
        格式.write_str(self.编码())
    }
}

编码枚举! {
    /// 联军内的职务（落 `玩家.联军职务`，没有联军时是 `NULL` 不是"成员"）。
    联军职务 {
        /// 元首：每联军至多一个（部分唯一索引），**在三级权限之上**，能做任何事（D35/D37）。
        元首 => "leader",
        /// 总理：每联军至多一个（部分唯一索引），在三级权限之上。
        总理 => "premier",
        /// 干部：权限由 `权限等级` 决定。
        干部 => "officer",
        /// 成员：权限由 `权限等级` 决定。
        成员 => "member",
    }
}

编码枚举! {
    /// 联军政治体制（落 `联军.政治体制`）。只影响权限阈值：越集权要求越高（D37）。
    联军政体 {
        /// 民主制：建联军默认值，同一动作要求的等级最低。
        民主制 => "democracy",
        /// 威权制：要求更高。
        威权制 => "authoritarian",
        /// 极权制：要求最高。
        极权制 => "totalitarian",
    }
}

/// 联军内的权限等级：1 级最低、3 级最高；元首与总理**在三级之上**，不需要等级（D37）。
///
/// 只回答"能不能做某个动作"，具体阈值由 `(政体, 动作)` 的常量表决定，
/// 那张表等实现对应指令时再补。落库是 DOMAIN `权限等级`（`SMALLINT` 1..=3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum 权限等级 {
    /// 1 级，最低。
    一级,
    /// 2 级。
    二级,
    /// 3 级，最高。
    三级,
}

impl 权限等级 {
    /// 落库值（`SMALLINT`，1..=3）。
    pub const fn 编码(self) -> i16 {
        match self {
            Self::一级 => 1,
            Self::二级 => 2,
            Self::三级 => 3,
        }
    }

    /// 从落库值解析；范围外返回 `None`（数据损坏）。
    pub const fn 解析(落库值: i16) -> Option<Self> {
        match 落库值 {
            1 => Some(Self::一级),
            2 => Some(Self::二级),
            3 => Some(Self::三级),
            _ => None,
        }
    }

    /// 全部变体，供对账与往返测试。
    pub const 全部: &'static [Self] = &[Self::一级, Self::二级, Self::三级];
}

// ---------------------------------------------------------------------------
// 物品词表（catalog 第二节；编码只增不改，D25/D27）
// ---------------------------------------------------------------------------

编码枚举! {
    /// 经济物资：生产 / 开采 / 兑换得到，不直接参与战斗（catalog 2.2）。
    资源 {
        /// 生活资料：经济核心，`生产` 的产出。
        生活资料 => "resource:food",
        /// 世界货币：`兑换世界货币` 得到。
        世界货币 => "resource:money",
        /// 石油：`开采石油`，战斗机耗油、生产线原料。
        石油 => "resource:oil",
        /// 钢铁：炼钢厂产出，扩建厂房、造装备。
        钢铁 => "resource:steel",
        /// 铁矿石：炼钢原料。
        铁矿石 => "resource:iron_ore",
        /// 铝土矿：炼铝原料（v2 资源链，v1 先建键，D28）。
        铝土矿 => "resource:bauxite",
        /// 金属铝：航空 / 装备材料（v2 资源链）。
        金属铝 => "resource:aluminium",
        /// 铀矿：铀浓缩原料（v2 资源链）。
        铀矿 => "resource:uranium_ore",
        /// 浓缩铀：战略武器原料（v2 资源链）。
        浓缩铀 => "resource:enriched_uranium",
        /// 钚：战略武器原料（v2 资源链）。
        钚 => "resource:plutonium",
        /// 文化产品：资本主义专属生产线产物。
        文化产品 => "resource:culture",
        /// 科技蓝图：`提升科技` 产出，生物实验室消耗。
        科技蓝图 => "resource:tech_blueprint",
        /// 疫苗：生物实验室产物，对抗生物病毒。
        疫苗 => "resource:vaccine",
        /// 试剂：生物实验室的中间物。
        试剂 => "resource:reagent",
    }
}

编码枚举! {
    /// 耐久的战斗器械：要兵员操作，打完还在（catalog 2.3）。
    装备 {
        /// 运输机：掠夺装载、军事运输。
        运输机 => "equipment:transport",
        /// 大型运输机：掠夺编队的主力（原版按它设编队）。
        大型运输机 => "equipment:heavy_transport",
        /// 侦察机：`侦察 @玩家`。
        侦察机 => "equipment:recon_plane",
        /// 战斗机：空袭主力；`战斗机巡航` 时才算防空。
        战斗机 => "equipment:fighter",
        /// 预警机：`全球侦察`、返航。
        预警机 => "equipment:awacs",
        /// 隐形飞机：`隐形轰炸`。
        隐形飞机 => "equipment:stealth_plane",
        /// 重炮：炮击与反击的主力。
        重炮 => "equipment:heavy_artillery",
        /// 火箭炮：远程打击。
        火箭炮 => "equipment:rocket_launcher",
        /// 铁穹：拦截火箭弹。
        铁穹 => "equipment:iron_dome",
        /// 地对空导弹：部署在城市的防空阵地。
        地对空导弹 => "equipment:sam",
        /// 打捞船：遗迹打捞，会被水雷击沉。
        打捞船 => "equipment:salvage_ship",
        /// 地堡：高科技生产线产出。
        地堡 => "equipment:bunker",
    }
}

编码枚举! {
    /// 一次性战斗消耗品：用掉就没（catalog 2.4）。
    弹药 {
        /// 防空弹药：拦截空袭，地对空导弹的弹药。
        防空弹药 => "ammo:aa",
        /// 火箭弹：火箭炮的弹药。
        火箭弹 => "ammo:rocket",
        /// 铁穹弹药：铁穹的拦截弹。
        铁穹弹药 => "ammo:iron_dome",
        /// 地雷：`部署地雷`，驻扎踩到就炸。
        地雷 => "ammo:mine",
        /// 水雷：部署在港口，击沉打捞船。
        水雷 => "ammo:naval_mine",
        /// 生物病毒：`投放病毒`，杀工厂工人与休假工人。
        生物病毒 => "ammo:virus",
    }
}

/// 库存里能放的东西：资源 | 装备 | 弹药（data-model 3.1）。
///
/// 变体清单的唯一真相是 `docs/game/catalog.md`；本枚举只固定
/// 「物品 = 三个子词表」的结构，加一种物品不动表结构、不加 CHECK。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum 物品 {
    /// 经济物资。
    资源(
        /// 具体是哪种资源。
        资源,
    ),
    /// 耐久战斗器械。
    装备(
        /// 具体是哪种装备。
        装备,
    ),
    /// 一次性消耗品。
    弹药(
        /// 具体是哪种弹药。
        弹药,
    ),
}

/// 物品的门类，与 [`物品`] 的三个变体一一对应。
///
/// 下游想"按大类分派"时用它，避免对 [`物品`] 写 `_ =>` 兜底把新物品静默吞掉。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum 门类 {
    /// 经济物资（[`物品::资源`]）。
    资源,
    /// 耐久战斗器械（[`物品::装备`]）。
    装备,
    /// 一次性消耗品（[`物品::弹药`]）。
    弹药,
}

impl 物品 {
    /// 落库编码（catalog 的稳定键，如 `resource:food`）。
    pub const fn 编码(self) -> &'static str {
        match self {
            Self::资源(哪种) => 哪种.编码(),
            Self::装备(哪种) => 哪种.编码(),
            Self::弹药(哪种) => 哪种.编码(),
        }
    }

    /// 从落库编码解析；未知编码返回 `None`（数据损坏，绝不 fallback）。
    pub fn 解析(原文: &str) -> Option<Self> {
        资源::解析(原文)
            .map(Self::资源)
            .or_else(|| 装备::解析(原文).map(Self::装备))
            .or_else(|| 弹药::解析(原文).map(Self::弹药))
    }

    /// 属于哪个门类。规则按门类分派，新物品不会被 `_ =>` 吞掉。
    pub const fn 门类(self) -> 门类 {
        match self {
            Self::资源(_) => 门类::资源,
            Self::装备(_) => 门类::装备,
            Self::弹药(_) => 门类::弹药,
        }
    }

    /// 中文展示名（如 `生活资料`），给玩家看的词由 `文案` 组装。
    pub const fn 名称(self) -> &'static str {
        match self {
            Self::资源(哪种) => 哪种.名称(),
            Self::装备(哪种) => 哪种.名称(),
            Self::弹药(哪种) => 哪种.名称(),
        }
    }
}

impl fmt::Display for 物品 {
    fn fmt(&self, 格式: &mut fmt::Formatter<'_>) -> fmt::Result {
        格式.write_str(self.编码())
    }
}

// ---------------------------------------------------------------------------
// 实体（data-model 5.2）
// ---------------------------------------------------------------------------

/// 指挥官：全部可变状态都在这（物资除外——那些在 [`库存`] 里，D18/D28）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 玩家 {
    /// 内部主键。
    pub id: 玩家Id,
    /// 对外秘密编号（`P…`），不得进日志与公开输出。
    pub uid: 玩家Uid,

    // 人口：工人 + 私军 + 地下工人 + 休假工人 = 总人口（不变量）
    /// 从事生产的人口。
    pub 工人: i64,
    /// 操作兵器的兵员；阵亡即人口永久损失。
    pub 私军: i64,
    /// 分配在地下工厂的工人，空袭炸不到。
    pub 地下工人: i64,
    /// 暂时不生产的工人。
    pub 休假工人: i64,

    // 生产
    /// 科技等级，解锁军备的门槛。
    pub 科技: i64,
    /// 产量乘数（原版"改进生产技术"）。
    pub 生产技术: i64,
    /// 产能上限（原版"扩建厂房"）。
    pub 厂房: i64,
    /// 地下工厂的累计投入，够阈值才有地下产能。
    pub 地下工厂投入: i64,
    /// 投进科技池的资源，攒满换一级科技。
    pub 科技池投入: i64,
    /// 科技池的容量上限。
    pub 科技池容量: i64,
    /// 可积攒的生产次数：每小时 +1、上限 8（全服定时任务推进，D32）。
    pub 生产次数: u8,
    /// 发动攻击、枚举地区要花；恢复模型待定。
    pub 军事行动点: i64,
    /// 可以为负；负值不能生产。
    pub 稳定度: i64,
    /// 高于基准提升稳定度。
    pub 工资: i64,

    // 节奏：全是"上次…时间"或计数器，判定用时间差，不存布尔（D28）
    /// `None` = 从没签到。
    pub 上次签到: Option<时间>,
    /// 今天已经招了多少（每日总量有上限，0 点由定时任务清零）。
    pub 今日招募数量: i64,
    /// 炮击冷却锚点。
    pub 上次炮击: Option<时间>,
    /// 驻扎冷却锚点：防止刚撤走就驻扎到别处。
    pub 上次驻扎: Option<时间>,

    // 位置与组织
    /// 驻扎地；弱引用，地区易主不影响。
    pub 驻扎地区: Option<地区Id>,
    /// 所在联军；`None` = 还没有联军。
    pub 联军: Option<联军Id>,
    /// 联军内职务；与 [`玩家::联军`] 同生共死（DB 的 CHECK 保证）。
    pub 联军职务: Option<联军职务>,
    /// 1–3 级；元首/总理为 `None`（他们在三级之上）。
    pub 联军权限等级: Option<权限等级>,
    /// 对当前联军的贡献；入盟清零、退盟无意义（D33）。
    pub 联军贡献: i64,
    /// `None` = 没入盟。
    pub 加入联军时间: Option<时间>,
    /// `None` = 已出保护期。
    pub 出生保护至: Option<时间>,
}

impl 玩家 {
    /// 总人口 = 工人 + 私军 + 地下工人 + 休假工人（D39 的口径）。
    ///
    /// 饱和相加：人口是 i64 计数，正常玩法碰不到上界；真溢出说明数据已坏，
    /// 饱和值在下游一眼可见。
    pub const fn 总人口(&self) -> i64 {
        self.工人
            .saturating_add(self.私军)
            .saturating_add(self.地下工人)
            .saturating_add(self.休假工人)
    }

    /// 生产次数的上限：每小时 +1、攒到 8 为止（D32）。
    ///
    /// 这是规则值，定在领域层；全服批处理（storage 的批量 UPDATE）
    /// 拿它当 `LEAST` 的上限，调度器拿周期当补算步长。
    pub const 生产次数上限: u8 = 8;

    /// 生产次数的结算周期：每小时一步（D32）。
    pub const 生产结算周期秒: i64 = 3_600;

    /// 签到冷却：两次签到至少隔一天。按时间差判定（`now - 上次 >= 冷却秒`，
    /// data-model 4.1）——不依赖定时任务，冷却到点自然能再签，漏签不补。
    pub const 签到冷却秒: i64 = 86_400;

    /// 签到奖励：每天一次、奖励固定（data-model 4.1），发 [`物品::资源`] 的
    /// 生活资料。数值是**占位**（roadmap 原则 4）：design.md 待定清单第 1 项
    /// 的数值尺度定稿后只改这个常量，规则函数不动。
    pub const 签到奖励: i64 = 100;

    /// `签到` 的规则判定（S4）：从没签过，或距上次已隔 [`玩家::签到冷却秒`]，
    /// 返回本次应得的奖励；否则按 [`错误::冷却中`] 拒绝，剩余秒一并带回。
    ///
    /// 判定只用两个时刻。差值为负（锚点在未来：时钟回拨、异常数据）或
    /// 溢出时，视同整段冷却还没走完——绝不给奖，也绝不给出负的剩余秒。
    pub const fn 签到(上次签到: Option<时间>, 现在: 时间) -> Result<i64, 错误> {
        let Some(上次) = 上次签到 else {
            return Ok(Self::签到奖励);
        };

        match 现在.0.checked_sub(上次.0) {
            Some(已隔) if 已隔 >= Self::签到冷却秒 => Ok(Self::签到奖励),
            Some(已隔) if 已隔 > 0 => Err(错误::冷却中 {
                剩余秒: Self::签到冷却秒.saturating_sub(已隔),
            }),
            _ => Err(错误::冷却中 {
                剩余秒: Self::签到冷却秒,
            }),
        }
    }
}

/// 玩家组织：可没有（`Option<联军>` 的那一侧）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 联军 {
    /// 内部主键。
    pub id: 联军Id,
    /// 对外**公开**编号（`C…`），可出现在排行榜与战报。
    pub uid: 联军Uid,
    /// 联军名；改名的冷却与审核见 data-model 4.2。
    pub 名称: String,

    // 政治
    /// `false` ⇒ 对外显示成 `***`；v1 默认视为通过。
    pub 名称是否审核: bool,
    /// 政治体制，只影响权限阈值。
    pub 政治体制: 联军政体,
    /// 首都；`None` = 还没设。必须是自己控制的地区（跨表不变量）。
    pub 首都: Option<地区Id>,

    // 对外宣称（公开）。实际值不在这里：人口要 SUM 成员，兵力就是 联军军队（D38/D39）
    /// 成员自填的公开人口数字，可以吹牛。
    pub 宣称人口: i64,
    /// 成员自填的公开兵力数字，可以吹牛。
    pub 宣称兵力: i64,

    // 兵
    /// 共享兵员，实际兵力（只对成员可见）；驻军的唯一来源。
    pub 联军军队: i64,
    /// 每天可转入的上限；0 = 未设置（禁止转入）。
    pub 扩军计划: i64,
    /// 今天已经转入多少；每天 0 点由定时任务清零。
    pub 当天扩军累计: i64,

    // 财政
    /// 0..=100，上缴比例。
    pub 税率: u8,
    /// 今天成员生产利润的合计；每天 0 点日结落一行再清零（D34）。
    pub 当天资本增量: i64,
    /// 改名冷却与审核的锚点；`None` = 建好还没改过。
    pub 上次改名时间: Option<时间>,
}

/// 一个地区：频道组 + 资源容器（v1 没有地理属性，D6）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 地区 {
    /// 内部主键。
    pub id: 地区Id,
    /// 对外访问凭证（`R…`）：不知道就打不到这个地区。
    pub uid: 地区Uid,
    /// 地区名，公开信息。
    pub 名称: String,
    /// 控制方；`None` = 无主。只能是联军，不能是个人。
    pub 控制方: Option<联军Id>,
    /// 从联军军队转化而来，守地区。
    pub 驻军: i64,
}

/// 库存：物品 → 数量。玩家背包、联军仓库、地区仓库是同一个机制（D15/D18）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct 库存(
    /// 物品到数量的映射；没有条目 = 数量为 0。
    pub BTreeMap<物品, i64>,
);

impl 库存 {
    /// 取某种物品的数量，不存在时为 0。
    pub fn 取(&self, 哪种: 物品) -> i64 {
        self.0.get(&哪种).copied().unwrap_or(0)
    }

    /// 增减某种物品（`数量` 为负即扣减）。
    ///
    /// 两条不变量在这里被拒绝而非被违反：减成负数 = [`错误::物资不足`]；
    /// 加法溢出 = [`错误::数量溢出`]。调用方不必事先 `取` 一遍再自己判断。
    pub fn 增(&mut self, 哪种: 物品, 数量: i64) -> Result<(), 错误> {
        let 现有 = self.取(哪种);
        let 新有 = 现有
            .checked_add(数量)
            .ok_or(错误::数量溢出 { 物品: 哪种 })?;

        if 新有 < 0 {
            return Err(错误::物资不足 {
                物品: 哪种,
                需要: 数量.checked_neg().unwrap_or(i64::MAX),
                现有,
            });
        }

        if 新有 == 0 {
            // 0 不留条目：库存表里"没有这一行"和"这一行是 0"是同一个意思。
            self.0.remove(&哪种);
        } else {
            self.0.insert(哪种, 新有);
        }
        Ok(())
    }
}

/// 地区里的一种建筑及其完成度（0..=100，到阈值才有效果，D17）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 地区建筑 {
    /// 挂在哪个地区。
    pub 地区: 地区Id,
    /// 哪种建筑（catalog 第三节的编码）。
    pub 建筑: 建筑编码,
    /// 完成度 `0..=100`；v1 只增不减。
    pub 完成度: u32,
}

#[cfg(test)]
mod 测试 {
    use super::{
        平台, 库存, 弹药, 时间, 权限等级, 物品, 玩家, 联军政体, 联军职务, 装备, 资源, 门类,
    };
    use crate::错误类型::错误;

    #[test]
    fn 签到_从没签过直接给奖() {
        assert_eq!(玩家::签到(None, 时间(1_000)), Ok(玩家::签到奖励));
    }

    #[test]
    fn 签到_冷却没到被拒且剩余秒正确() {
        let 上次 = 时间(10_000);

        let 结果 = 玩家::签到(Some(上次), 时间(10_000 + 玩家::签到冷却秒 - 1));
        assert!(
            matches!(结果, Err(错误::冷却中 { 剩余秒: 1 })),
            "实际：{结果:?}"
        );
    }

    #[test]
    fn 签到_冷却恰好到点可以再签() {
        let 上次 = 时间(10_000);

        assert_eq!(
            玩家::签到(Some(上次), 时间(10_000 + 玩家::签到冷却秒)),
            Ok(玩家::签到奖励)
        );
    }

    #[test]
    fn 签到_锚点在未来按整段冷却拒绝() {
        // 时钟回拨 / 异常数据：绝不给奖，也绝不给出负的剩余秒。
        let 结果 = 玩家::签到(Some(时间(5_000)), 时间(1_000));
        assert!(
            matches!(
                结果,
                Err(错误::冷却中 {
                    剩余秒: 玩家::签到冷却秒
                })
            ),
            "实际：{结果:?}"
        );
    }

    #[test]
    fn 资源_编码与解析能往返() {
        assert_eq!(资源::全部.len(), 14, "catalog 2.2：资源 14 种");
        for 每个 in 资源::全部 {
            assert_eq!(资源::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
    }

    #[test]
    fn 装备_编码与解析能往返() {
        assert_eq!(装备::全部.len(), 12, "catalog 2.3：装备 12 种");
        for 每个 in 装备::全部 {
            assert_eq!(装备::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
    }

    #[test]
    fn 弹药_编码与解析能往返() {
        assert_eq!(弹药::全部.len(), 6, "catalog 2.4：弹药 6 种");
        for 每个 in 弹药::全部 {
            assert_eq!(弹药::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
    }

    #[test]
    fn 物品_编码与解析能往返() {
        assert_eq!(
            物品::解析(资源::生活资料.编码()),
            Some(物品::资源(资源::生活资料))
        );
        assert_eq!(物品::解析(装备::重炮.编码()), Some(物品::装备(装备::重炮)));
        assert_eq!(物品::解析(弹药::地雷.编码()), Some(物品::弹药(弹药::地雷)));
        assert_eq!(物品::资源(资源::生活资料).编码(), "resource:food");
        assert_eq!(物品::装备(装备::重炮).编码(), "equipment:heavy_artillery");
        assert_eq!(物品::弹药(弹药::生物病毒).编码(), "ammo:virus");
    }

    #[test]
    fn 名称_与变体名一致_编码与名是两回事() {
        // 名称给人看（catalog 词条），编码给库与日志（稳定键）——两个方向都不许漂。
        assert_eq!(资源::生活资料.名称(), "生活资料");
        assert_eq!(物品::资源(资源::钢铁).名称(), "钢铁");
        assert_eq!(物品::装备(装备::重炮).名称(), "重炮");
        assert_eq!(联军职务::元首.名称(), "元首");
    }

    #[test]
    fn 未知编码一律返回_none() {
        // 编码是契约：拼错、缺前缀、大写、未知类别都不是"猜一下"的理由。
        assert_eq!(资源::解析("food"), None, "子词表的编码带完整前缀");
        assert_eq!(物品::解析("resource:生活资料"), None);
        assert_eq!(物品::解析("weapon:food"), None);
        assert_eq!(物品::解析("RESOURCE:FOOD"), None);
        assert_eq!(物品::解析(""), None);
        assert_eq!(物品::解析("resource:"), None);
        assert_eq!(平台::解析("qq-guild"), None);
        assert_eq!(联军职务::解析("emperor"), None);
        assert_eq!(联军政体::解析("monarchy"), None);
        assert_eq!(权限等级::解析(0), None);
        assert_eq!(权限等级::解析(4), None);
    }

    #[test]
    fn 分类枚举的编码落库值正确() {
        assert_eq!(平台::Onebot.编码(), "onebot");
        assert_eq!(联军职务::元首.编码(), "leader");
        assert_eq!(联军政体::极权制.编码(), "totalitarian");
        assert_eq!(权限等级::三级.编码(), 3);
    }

    #[test]
    fn 分类枚举_编码与解析能往返() {
        for 每个 in 平台::全部 {
            assert_eq!(平台::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
        for 每个 in 联军职务::全部 {
            assert_eq!(联军职务::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
        for 每个 in 联军政体::全部 {
            assert_eq!(联军政体::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
        for 每个 in 权限等级::全部 {
            assert_eq!(权限等级::解析(每个.编码()), Some(*每个), "{}", 每个.编码());
        }
    }

    #[test]
    fn 门类与变体一一对应() {
        assert_eq!(物品::资源(资源::石油).门类(), 门类::资源);
        assert_eq!(物品::装备(装备::战斗机).门类(), 门类::装备);
        assert_eq!(物品::弹药(弹药::地雷).门类(), 门类::弹药);
    }

    #[test]
    fn 库存_没有条目时取出来是零() {
        let 空 = 库存::default();
        assert_eq!(空.取(物品::资源(资源::生活资料)), 0);
    }

    #[test]
    fn 库存_增减到零不留条目() {
        let mut 袋 = 库存::default();
        let 食物 = 物品::资源(资源::生活资料);

        袋.增(食物, 1_200).expect("增为正数不应失败");
        assert_eq!(袋.取(食物), 1_200);

        袋.增(食物, -1_200).expect("恰好减到零不应失败");
        assert_eq!(袋.取(食物), 0);
        assert!(袋.0.is_empty(), "减到 0 应当移除条目");
    }

    #[test]
    fn 库存_减成负数被拒绝且原值不动() {
        let mut 袋 = 库存::default();
        let 食物 = 物品::资源(资源::生活资料);
        袋.增(食物, 300).expect("增为正数不应失败");

        let 结果 = 袋.增(食物, -500);
        assert!(
            matches!(
                结果,
                Err(错误::物资不足 {
                    需要: 500,
                    现有: 300,
                    ..
                })
            ),
            "实际：{结果:?}"
        );
        assert_eq!(袋.取(食物), 300, "失败的扣减不得改动库存");
    }

    #[test]
    fn 库存_加法溢出被拒绝() {
        let mut 袋 = 库存::default();
        let 食物 = 物品::资源(资源::生活资料);
        袋.增(食物, i64::MAX).expect("增到上界不应失败");

        let 结果 = 袋.增(食物, 1);
        assert!(matches!(结果, Err(错误::数量溢出 { .. })), "实际：{结果:?}");
    }
}
