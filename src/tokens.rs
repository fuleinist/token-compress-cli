//! Heuristic token estimation (documented as an approximation).
//!
//! ~4 ASCII chars per token, ~1.8 CJK chars per token, ~3 other chars per
//! token. Not tokenizer-exact; for reporting only.

fn is_cjk(cp: u32) -> bool {
    matches!(cp,
        0x3040..=0x30FF      // Japanese kana
        | 0x3400..=0x4DBF    // CJK ext A
        | 0x4E00..=0x9FFF    // CJK unified
        | 0xAC00..=0xD7AF    // Hangul
        | 0xF900..=0xFAFF    // CJK compat
        | 0x20000..=0x2FA1F  // CJK ext B..F, compat supplement
    )
}

pub fn est_tokens(s: &str) -> u64 {
    let mut ascii = 0u64;
    let mut cjk = 0u64;
    let mut other = 0u64;
    for ch in s.chars() {
        let cp = ch as u32;
        if cp < 128 {
            ascii += 1;
        } else if is_cjk(cp) {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    let est = ascii as f64 / 4.0 + cjk as f64 / 1.8 + other as f64 / 3.0;
    est.ceil() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sane_estimates() {
        assert_eq!(est_tokens(""), 0);
        assert_eq!(est_tokens("abcd"), 1);
        assert!(est_tokens("你好世界") >= 2);
        assert!(est_tokens(&"x".repeat(400)) == 100);
    }
}
