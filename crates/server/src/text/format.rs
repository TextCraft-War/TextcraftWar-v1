//! 排版辅助：数值展示。
//!
//! **一切数值展示都走 [`千分位`]**（impl.md 第七节）——业务代码里不许
//! 手写 `format!("{}", 数值)`。

/// 整数千分位：`1500000` → `1,500,000`；负数保留负号，逗号只进数字部分。
pub(crate) fn 千分位(值: i64) -> String {
    let 符号 = if 值 < 0 { "-" } else { "" };
    let 数字 = 值.unsigned_abs().to_string();
    let 位数 = 数字.len();
    let 逗号数 = 位数.saturating_sub(1) / 3;

    let mut 结果 = String::with_capacity(位数.saturating_add(逗号数));
    for (序号, 字符) in 数字.chars().enumerate() {
        if 序号 > 0 && 位数.saturating_sub(序号) % 3 == 0 {
            结果.push(',');
        }
        结果.push(字符);
    }

    format!("{符号}{结果}")
}

#[cfg(test)]
mod 测试 {
    use super::千分位;

    #[test]
    fn 千分位_三位一组_负号在外() {
        assert_eq!(千分位(0), "0");
        assert_eq!(千分位(999), "999");
        assert_eq!(千分位(1_000), "1,000");
        assert_eq!(千分位(1_500_000), "1,500,000");
        assert_eq!(千分位(1_234_567_890), "1,234,567,890");
        assert_eq!(千分位(-50), "-50");
        assert_eq!(千分位(-1_234_567), "-1,234,567");
    }
}
