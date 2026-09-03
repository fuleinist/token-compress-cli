//! Repeat-dedup stage (reversible, self-describing).
//!
//! Runs of N >= 3 consecutive identical segments become one copy preceded by a
//! `⟦REP N×⟧` marker segment. Expand restores the copies.

use crate::TcError;

fn marker(n: usize) -> String {
    format!("⟦REP {n}×⟧")
}

/// Parse `⟦REP N×⟧`; returns None for anything else.
pub fn parse_marker(seg: &str) -> Option<usize> {
    let rest = seg.strip_prefix("⟦REP ")?.strip_suffix("×⟧")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

pub fn compress(segs: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(segs.len());
    let mut i = 0usize;
    while i < segs.len() {
        let mut j = i + 1;
        while j < segs.len() && segs[j] == segs[i] {
            j += 1;
        }
        let run_len = j - i;
        if run_len >= 3 && !segs[i].is_empty() {
            out.push(marker(run_len));
            out.push(segs[i].clone());
        } else {
            for seg in &segs[i..j] {
                out.push(seg.clone());
            }
        }
        i = j;
    }
    out
}

pub fn expand(segs: Vec<String>) -> Result<Vec<String>, TcError> {
    let mut out: Vec<String> = Vec::with_capacity(segs.len());
    let mut i = 0usize;
    while i < segs.len() {
        if let Some(n) = parse_marker(&segs[i]) {
            if n < 3 {
                return Err(TcError::new(format!("corrupt map: REP count {n} < 3")));
            }
            let copy = segs.get(i + 1).ok_or_else(|| {
                TcError::new("corrupt map: REP marker without a following line".to_string())
            })?;
            for _ in 0..n {
                out.push(copy.clone());
            }
            i += 2;
        } else {
            out.push(segs[i].clone());
            i += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(s: &str) {
        let segs: Vec<String> = s.split('\n').map(String::from).collect();
        let back = expand(compress(segs.clone())).unwrap();
        assert_eq!(back.join("\n"), s, "input: {s:?}");
    }

    #[test]
    fn roundtrips() {
        rt("a\na\na");
        rt("a\na");
        rt("x\ny\ny\ny\ny\nz");
        rt("line\nline\nline\nline\nline");
        rt("a\na\na\nb\nb\nb");
        rt("⟦REP 3×⟧\n⟦REP 3×⟧\n⟦REP 3×⟧"); // escaped upstream, treated as data
    }

    #[test]
    fn compresses() {
        let segs: Vec<String> = vec!["l", "l", "l", "l"]
            .into_iter()
            .map(String::from)
            .collect();
        let out = compress(segs);
        assert_eq!(out, vec!["⟦REP 4×⟧", "l"]);
    }
}
