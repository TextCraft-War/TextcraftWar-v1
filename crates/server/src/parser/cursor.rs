//! 词元游标：按空白切词的零分配机制（D49.1）。
//!
//! 只管"逐词借出"与"还有没有实义尾巴"，不认识任何指令——
//! 词头映射与解析主流程在 [`crate::解析`] 主文件里。
//! 带参指令（S4 起）在这里逐个取**位置参数**，取完用
//! [`词元游标::有实义剩余`] 断言没有多余尾巴——多余参数不静默吞。

/// 句末允许被剥掉的标点。
pub(crate) const 句末标点: [char; 4] = ['。', '.', '！', '!'];

/// 零分配词元游标：按空白逐词借出 `&str`，不收集 `Vec`（D49.1）。
pub(crate) struct 词元游标<'a> {
    词元: std::iter::Peekable<std::str::SplitWhitespace<'a>>,
}

impl<'a> 词元游标<'a> {
    /// 从玩家原话出发建游标（首尾空白自动跳过）。
    // 每次解析都是新建即用，`Default` 给不出有意义的空游标。
    #[allow(
        clippy::new_without_default,
        reason = "无意义的空游标不值得一个 Default"
    )]
    pub(crate) fn 新(原文: &'a str) -> Self {
        Self {
            词元: 原文.split_whitespace().peekable(),
        }
    }

    /// 借出下一个词元。
    pub(crate) fn 下一个(&mut self) -> Option<&'a str> {
        self.词元.next()
    }

    /// 后面还有没有词元（不消费）。
    pub(crate) fn 还有下一个(&mut self) -> bool {
        self.词元.peek().is_some()
    }

    /// 后面还有没有**实义**参数：把句末标点剥掉之后不空的词元才算
    /// （`签到。` 的句号、`签到 ..` 的省略点都不算参数）。
    pub(crate) fn 有实义剩余(&mut self) -> bool {
        self.词元
            .any(|词| !词.trim_end_matches(句末标点).is_empty())
    }
}

#[cfg(test)]
mod 测试 {
    use super::词元游标;

    #[test]
    fn 游标逐词借出不收集() {
        let mut 游标 = 词元游标::新("  签到　尾巴1 尾巴2。 ");

        assert_eq!(游标.下一个(), Some("签到"));
        assert!(游标.还有下一个());
        assert_eq!(游标.下一个(), Some("尾巴1"));
        assert!(游标.还有下一个());
        assert_eq!(游标.下一个(), Some("尾巴2。"));
        assert!(!游标.还有下一个());
        // 有实义剩余 是消费性检查，放最后：纯句末标点的尾巴不算参数。
        assert!(!游标.有实义剩余());
    }

    #[test]
    fn 有实义剩余_分得清实义参数与纯标点尾巴() {
        let mut 有参数 = 词元游标::新("签到 晓");
        assert_eq!(有参数.下一个(), Some("签到"));
        assert!(有参数.有实义剩余(), "晓 是实义参数");

        let mut 只有句号 = 词元游标::新("签到 。");
        assert_eq!(只有句号.下一个(), Some("签到"));
        assert!(!只有句号.有实义剩余());
    }
}
