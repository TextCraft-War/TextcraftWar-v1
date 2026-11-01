//! 把结构化指令映射成 [`domain::指令`]（D51）。
//!
//! 指令在 Koishi 侧注册并翻译：过线的已经是 `command`（指令名）+
//! `args`（参数对象），切词、参数类型、arity、`帮助`、近似提示全归
//! Koishi——**语法在平台上，语义在这里**。本模块只剩两件事：
//!
//! 1. **指令名 → 指令骨架**：精确匹配，加指令只改 [`识别`] 这张表
//!    （"词头表 > 40 独立成唯一事实源 + 帮助/近似匹配"的触发线已随
//!    D51 作废——帮助归 Koishi）；
//! 2. **args → 指令载荷**：逐指令反序列化，数值范围等游戏语义由
//!    domain 的强类型把守（随 `招募工人` 落地）。当前三条指令都是零参：
//!    `args` 必须缺席或为空对象。
//!
//! 映射只认名字，不碰数据库、不判权限：`注册` 能不能成功是下一层的事。
//! **产出是 `domain::指令`**（D44/D48）：一条指令"意味着什么动作"是规则层的事；
//! 认不出的名字与形状不对的 args 都留在本层，由分发层结合身份决定对外错误码
//! （`not_registered` / `unknown_command` / `bad_request`）。

use serde_json::Value;

/// 解析结果：认识的进领域，认不出的留在本层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum 结果 {
    /// 认识的领域指令。
    指令(domain::指令),
    /// 指令名不认识。Koishi 只转发注册过的指令（D51），
    /// 这条出现即两端版本偏差。
    不认识 {
        /// 插件转发的指令名，供日志与提示。
        指令名: String,
    },
    /// 认识指令名，但 `args` 形状不对：缺参、类型不对，
    /// 或零参指令带了非空 `args`（插件 bug 才会触发，D51）。
    参数错误 {
        /// 指令名，供日志。
        指令名: String,
    },
}

/// 指令名 → 指令骨架。加指令只改这张表（D49：词头映射必变，主干冻结）。
fn 识别(指令名: &str) -> Option<domain::指令> {
    match 指令名 {
        "注册" => Some(domain::指令::注册),
        "我的资料" => Some(domain::指令::我的资料),
        "签到" => Some(domain::指令::签到),
        _ => None,
    }
}

/// 映射一条结构化指令。
pub(crate) fn 解析(指令名: &str, 参数: &Value) -> 结果 {
    let Some(指令) = 识别(指令名) else {
        return 结果::不认识 {
            指令名: 指令名.to_owned(),
        };
    };

    // 零参指令：args 必须缺席（Null）或为空对象。带参指令的**额外键**按
    // "加字段容忍"忽略（protocol impl.md 2.4），但零参指令出现非空 args
    // 本身就是形状不对——静默吞掉只会把插件 bug 藏起来。
    // 穷尽列出其余形状（不许 `_`）：serde_json 的 Value 就这六种，
    // 以后加变体编译器会逼这里表态。
    match 参数 {
        Value::Null => (),
        Value::Object(表) if 表.is_empty() => (),
        Value::Bool(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Array(_)
        | Value::Object(_) => {
            return 结果::参数错误 {
                指令名: 指令名.to_owned(),
            };
        },
    }

    结果::指令(指令)
}

#[cfg(test)]
mod 测试 {
    use super::{结果, 解析};
    use domain::指令;
    use serde_json::{Value, json};

    #[test]
    fn 零参指令按名字映射() {
        assert_eq!(解析("注册", &Value::Null), 结果::指令(指令::注册));
        assert_eq!(解析("我的资料", &Value::Null), 结果::指令(指令::我的资料));
        assert_eq!(解析("签到", &Value::Null), 结果::指令(指令::签到));
        // 空对象与缺席等价：插件两种写法都合法。
        assert_eq!(解析("签到", &json!({})), 结果::指令(指令::签到));
    }

    #[test]
    fn 未知指令名留在本层() {
        assert_eq!(
            解析("随便说说", &Value::Null),
            结果::不认识 {
                指令名: "随便说说".to_owned()
            }
        );
        // 精确匹配：拆开、加字都不算（`生产` 与 `生产次数` 不打架的老规矩）。
        assert_eq!(
            解析("我的 资料", &Value::Null),
            结果::不认识 {
                指令名: "我的 资料".to_owned()
            }
        );
        assert_eq!(
            解析("注册账号", &Value::Null),
            结果::不认识 {
                指令名: "注册账号".to_owned()
            }
        );
    }

    #[test]
    fn 零参指令带非空_args_是形状不对() {
        assert_eq!(
            解析("签到", &json!({ "数量": 5 })),
            结果::参数错误 {
                指令名: "签到".to_owned()
            }
        );
    }
}
