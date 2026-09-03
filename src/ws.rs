//! Whitespace stage (reversible).
//!
//! Compress order: (1) squeeze blank-line runs, (2) strip trailing whitespace,
//! (3) collapse interior space runs of 2+ to one space.
//! Expand applies the exact inverse in reverse order.
//!
//! The segment model: text split on '\n' (joined back with '\n'), so CRLF
//! survives as a trailing '\r' inside a segment.

use serde::{Deserialize, Serialize};

/// Blank line = segment consisting only of spaces/tabs/\r (possibly empty).
fn is_blank(seg: &str) -> bool {
    seg.chars().all(|c| c == ' ' || c == '\t' || c == '\r')
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WsMap {
    /// Blank-line runs removed by squeezing: (index of 2nd kept segment in the
    /// output, removed segments verbatim).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blanks: Vec<(usize, Vec<String>)>,
    /// Trailing whitespace stripped per segment: (segment index, stripped string).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trailing: Vec<(usize, String)>,
    /// Interior space runs collapsed: (segment index, byte offset of the kept
    /// space, original run length).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<(usize, usize, usize)>,
}

impl WsMap {
    pub fn is_empty(&self) -> bool {
        self.blanks.is_empty() && self.trailing.is_empty() && self.runs.is_empty()
    }
}

pub fn compress(segs: Vec<String>) -> (Vec<String>, WsMap) {
    let mut map = WsMap::default();

    // (1) Squeeze blank runs: keep first 2 segments of a run of 3+.
    let mut out: Vec<String> = Vec::with_capacity(segs.len());
    let mut i = 0usize;
    while i < segs.len() {
        if is_blank(&segs[i]) {
            let start = i;
            while i < segs.len() && is_blank(&segs[i]) {
                i += 1;
            }
            let run_len = i - start;
            if run_len >= 3 {
                // Keep the first two segments of the run.
                out.push(segs[start].clone());
                out.push(segs[start + 1].clone());
                let removed: Vec<String> = segs[start + 2..i].to_vec();
                map.blanks.push((out.len() - 1, removed));
            } else {
                for seg in &segs[start..i] {
                    out.push(seg.clone());
                }
            }
        } else {
            out.push(segs[i].clone());
            i += 1;
        }
    }

    // (2)+(3) per segment: strip trailing ws, then collapse interior runs.
    for (idx, seg) in out.iter_mut().enumerate() {
        // Split off an optional CRLF '\r' terminator so it is preserved.
        let (content, term) = match seg.strip_suffix('\r') {
            Some(c) => (c.to_string(), "\r"),
            None => (std::mem::take(seg), ""),
        };

        let trimmed_end = content.trim_end_matches([' ', '\t']);
        if trimmed_end.len() < content.len() {
            map.trailing
                .push((idx, content[trimmed_end.len()..].to_string()));
        }

        // Collapse runs of 2+ spaces in the trimmed content.
        let bytes = trimmed_end.as_bytes();
        let mut collapsed = String::with_capacity(trimmed_end.len());
        let mut j = 0usize;
        while j < bytes.len() {
            if bytes[j] == b' ' {
                let start = j;
                while j < bytes.len() && bytes[j] == b' ' {
                    j += 1;
                }
                let run_len = j - start;
                let kept_off = collapsed.len();
                collapsed.push(' ');
                if run_len >= 2 {
                    map.runs.push((idx, kept_off, run_len));
                }
            } else {
                // Space is ASCII, so byte-wise copy up to next space is UTF-8 safe.
                let start = j;
                while j < bytes.len() && bytes[j] != b' ' {
                    j += 1;
                }
                collapsed.push_str(&trimmed_end[start..j]);
            }
        }

        *seg = collapsed + term;
    }

    (out, map)
}

pub fn expand(mut segs: Vec<String>, map: &WsMap) -> Result<Vec<String>, crate::TcError> {
    // Inverse of (3): uncollapse runs. Indices refer to current (ws-output) vec.
    for &(idx, off, len) in map.runs.iter().rev() {
        let seg = segs.get_mut(idx).ok_or_else(|| {
            crate::TcError::new(format!("corrupt map: ws run segment {idx} missing"))
        })?;
        if len < 2 {
            return Err(crate::TcError::new(format!(
                "corrupt map: ws run length {len} < 2"
            )));
        }
        let bytes = seg.as_bytes();
        if off >= bytes.len() || bytes[off] != b' ' {
            return Err(crate::TcError::new(format!(
                "corrupt map: ws run at segment {idx} offset {off} does not point at a space"
            )));
        }
        let mut replacement = " ".repeat(len);
        if off + 1 < bytes.len()
            && bytes[off + 1..].starts_with(b"\r")
            && off + 1 == bytes.len() - 1
        {
            // kept space directly before a lone trailing '\r'
            replacement.push('\r');
            *seg = format!("{}{}{}", &seg[..off], replacement, "");
        } else {
            *seg = format!("{}{}{}", &seg[..off], replacement, &seg[off + 1..]);
        }
    }

    // Inverse of (2): restore trailing whitespace before optional '\r'.
    for (idx, stripped) in map.trailing.iter().rev() {
        let idx = *idx;
        let seg = segs.get_mut(idx).ok_or_else(|| {
            crate::TcError::new(format!("corrupt map: ws trailing segment {idx} missing"))
        })?;
        match seg.strip_suffix('\r') {
            Some(content) => *seg = format!("{content}{stripped}\r"),
            None => seg.push_str(stripped),
        }
    }

    // Inverse of (1): re-insert removed blank segments (descending to keep
    // indices valid).
    for (idx, removed) in map.blanks.iter().rev() {
        if *idx >= segs.len() {
            return Err(crate::TcError::new(format!(
                "corrupt map: ws blanks index {idx} out of range"
            )));
        }
        for (k, r) in removed.iter().enumerate() {
            segs.insert(idx + 1 + k, r.clone());
        }
    }

    Ok(segs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(s: &str) {
        let segs: Vec<String> = s.split('\n').map(String::from).collect();
        let (comp, map) = compress(segs.clone());
        let back = expand(comp, &map).unwrap();
        assert_eq!(back.join("\n"), s, "input: {s:?}");
    }

    #[test]
    fn roundtrips() {
        rt("");
        rt("a  b");
        rt("a     b   c");
        rt("trailing   \nnext");
        rt("a\n\n\n\n\nb");
        rt("\n\n\n\n");
        rt("a\t\tb  \r\nc\r\n");
        rt("   \n   \n   \n   \nx");
        rt("single space stays");
    }
}
