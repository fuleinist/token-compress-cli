//! Filler-phrase stage (reversible).
//!
//! Well-known LLM filler phrases are replaced with `⟦F n⟧` markers. The phrase
//! list is fixed for map format v1, so expand needs no per-phrase map entries;
//! `filler_used` is recorded for stats only.

use crate::TcError;

/// Fixed phrase list for map v1. NEVER reorder or edit after release;
/// append only, and bump the map version for incompatible changes.
pub const PHRASES: &[&str] = &[
    "Great question! ",
    "I'd be happy to help! ",
    "I'd be happy to help. ",
    "Certainly! ",
    "Sure thing! ",
    "Absolutely! ",
    "I hope this helps! ",
    "I hope this helps. ",
    "Let me know if you have any questions! ",
    "Let me know if you have any questions. ",
    "As an AI language model, ",
    "Thanks for asking! ",
];

pub fn marker(id: usize) -> String {
    format!("⟦F{id}⟧")
}

pub fn parse_marker(frag: &str) -> Option<usize> {
    let rest = frag.strip_prefix("⟦F")?.strip_suffix('⟧')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

pub fn compress(segs: Vec<String>, used: &mut Vec<u32>) -> Vec<String> {
    segs.into_iter()
        .map(|seg| compress_one(&seg, used))
        .collect()
}

fn compress_one(seg: &str, used: &mut Vec<u32>) -> String {
    let mut out = String::with_capacity(seg.len());
    let mut rest = seg;
    while !rest.is_empty() {
        // Find the earliest phrase occurrence.
        let mut best: Option<(usize, usize)> = None; // (position, phrase id)
        for (id, phrase) in PHRASES.iter().enumerate() {
            if let Some(p) = rest.find(phrase) {
                match best {
                    Some((bp, _)) if p > bp => {}
                    _ => best = Some((p, id)),
                }
            }
        }
        match best {
            Some((pos, id)) => {
                out.push_str(&rest[..pos]);
                out.push_str(&marker(id));
                if !used.contains(&(id as u32)) {
                    used.push(id as u32);
                }
                rest = &rest[pos + PHRASES[id].len()..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

pub fn expand(segs: Vec<String>) -> Result<Vec<String>, TcError> {
    segs.into_iter()
        .map(|seg| {
            let mut out = String::with_capacity(seg.len());
            let mut rest = seg.as_str();
            while let Some(pos) = rest.find("⟦F") {
                out.push_str(&rest[..pos]);
                let tail = &rest[pos..];
                let end = tail.find('⟧').ok_or_else(|| {
                    TcError::new("corrupt map: unterminated filler marker".to_string())
                })?;
                let frag = &tail[..end + '⟧'.len_utf8()];
                match parse_marker(frag) {
                    Some(id) => {
                        let phrase = PHRASES.get(id).ok_or_else(|| {
                            TcError::new(format!(
                                "corrupt map: filler id {id} outside known list (v1 has {} phrases)",
                                PHRASES.len()
                            ))
                        })?;
                        out.push_str(phrase);
                        rest = &tail[end + '⟧'.len_utf8()..];
                    }
                    None => {
                        out.push('⟦');
                        rest = &tail['⟦'.len_utf8()..];
                    }
                }
            }
            out.push_str(rest);
            Ok(out)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(s: &str) {
        let segs: Vec<String> = s.split('\n').map(String::from).collect();
        let mut used = Vec::new();
        let comp = compress(segs.clone(), &mut used);
        let back = expand(comp).unwrap();
        assert_eq!(back.join("\n"), s, "input: {s:?}");
    }

    #[test]
    fn roundtrips() {
        rt("Great question! The answer is 42.");
        rt("Certainly! Certainly! Certainly!");
        rt("no filler here");
    }

    #[test]
    fn compresses_filler() {
        let mut used = Vec::new();
        let out = compress(vec!["Great question! x".to_string()], &mut used);
        assert_eq!(out, vec!["⟦F0⟧x"]);
        assert_eq!(used, vec![0]);
    }
}
