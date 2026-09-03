//! Marker escaping.
//!
//! Every literal `⟦` (U+27E6) in user input is replaced with `⟦ESC⟧` before any
//! stage runs, so generated markers (`⟦REP n×⟧`, `⟦U n⟧`, `⟦F n⟧`, footer) can
//! never collide with user text. Expand resolves markers first, then unescapes.

pub const ESC: &str = "⟦ESC⟧";

pub fn escape(s: &str) -> String {
    s.replace('⟦', ESC)
}

/// Inverse of [`escape`]. Scans left to right; every `⟦ESC⟧` becomes `⟦`.
/// Any other `⟦` (which can only be a leftover generated marker) is kept as-is.
pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('⟦') {
        out.push_str(&rest[..i]);
        if rest[i..].starts_with(ESC) {
            out.push('⟦');
            rest = &rest[i + ESC.len()..];
        } else {
            out.push('⟦');
            rest = &rest[i + '⟦'.len_utf8()..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_plain() {
        for s in ["", "abc", "⟦", "⟦⟧", "⟦ESC⟧", "⟦U1⟧", "x⟦y⟧z", "⟦⟦⟦"] {
            assert_eq!(unescape(&escape(s)), s, "input: {s:?}");
        }
    }

    #[test]
    fn double_escape_roundtrip() {
        let once = escape("⟦ESC⟧");
        let twice = escape(&once);
        assert_eq!(unescape(&twice), once);
        assert_eq!(unescape(&unescape(&twice)), "⟦ESC⟧");
    }
}
