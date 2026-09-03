//! URL-shortening stage (reversible via the map's url list).
//!
//! http(s) URLs longer than 24 chars become `⟦U n⟧`; identical URLs share one
//! index. Expand substitutes back from the list.

use crate::TcError;

const MIN_URL_LEN: usize = 24;

pub fn marker(id: usize) -> String {
    format!("⟦U{id}⟧")
}

pub fn parse_marker(seg_fragment: &str) -> Option<usize> {
    let rest = seg_fragment.strip_prefix("⟦U")?.strip_suffix('⟧')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn url_end(bytes: &[u8], start: usize) -> usize {
    let mut j = start;
    while j < bytes.len() {
        let b = bytes[j];
        if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
            break;
        }
        j += 1;
    }
    j
}

pub fn compress(segs: Vec<String>, urls: &mut Vec<String>) -> Vec<String> {
    segs.into_iter()
        .map(|seg| compress_one(&seg, urls))
        .collect()
}

fn compress_one(seg: &str, urls: &mut Vec<String>) -> String {
    let bytes = seg.as_bytes();
    let mut out = String::with_capacity(seg.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let scheme_len = if seg[i..].starts_with("https://") {
            8
        } else if seg[i..].starts_with("http://") {
            7
        } else {
            0
        };
        if scheme_len > 0 {
            // Require a boundary before the URL (start of segment or whitespace).
            let boundary_ok = i == 0
                || matches!(
                    bytes[i - 1],
                    b' ' | b'\t' | b'\r' | b'"' | b'\'' | b'(' | b'<' | b'['
                );
            let j = url_end(bytes, i);
            if boundary_ok && j - i > MIN_URL_LEN {
                let url = &seg[i..j];
                let id = match urls.iter().position(|u| u == url) {
                    Some(k) => k,
                    None => {
                        urls.push(url.to_string());
                        urls.len() - 1
                    }
                };
                out.push_str(&marker(id));
            } else {
                out.push_str(&seg[i..j]);
            }
            i = j;
        } else {
            let ch = seg[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

pub fn expand(segs: Vec<String>, urls: &[String]) -> Result<Vec<String>, TcError> {
    segs.into_iter().map(|seg| expand_one(&seg, urls)).collect()
}

fn expand_one(seg: &str, urls: &[String]) -> Result<String, TcError> {
    let mut out = String::with_capacity(seg.len());
    let mut rest = seg;
    while let Some(pos) = rest.find("⟦U") {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        let end = tail
            .find('⟧')
            .ok_or_else(|| TcError::new("corrupt map: unterminated URL marker".to_string()))?;
        let frag = &tail[..end + '⟧'.len_utf8()];
        match parse_marker(frag) {
            Some(id) => {
                let url = urls
                    .get(id)
                    .ok_or_else(|| TcError::new(format!("corrupt map: URL id {id} not in map")))?;
                out.push_str(url);
                rest = &tail[end + '⟧'.len_utf8()..];
            }
            None => {
                // Not a valid marker; keep literally.
                out.push_str(&tail[..pos + '⟦'.len_utf8()]);
                rest = &tail['⟦'.len_utf8()..];
                // Note: tail starts at "⟦U", so advance past the '⟦'.
                // (kept simple and literal)
                out.push('U');
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(s: &str) {
        let segs: Vec<String> = s.split('\n').map(String::from).collect();
        let mut urls = Vec::new();
        let comp = compress(segs.clone(), &mut urls);
        let back = expand(comp, &urls).unwrap();
        assert_eq!(back.join("\n"), s, "input: {s:?}");
    }

    #[test]
    fn roundtrips() {
        rt("see https://example.com/very/long/path/to/resource for details");
        rt("short http://a.io stays");
        rt("https://github.com/fuleinist/token-compress-cli/issues/123");
        rt("xhttps://not-a-boundary.example.com/long/enough/to/matter");
        rt("https://same.example.com/path/twice https://same.example.com/path/twice");
    }

    #[test]
    fn dedups_identical_urls() {
        let mut urls = Vec::new();
        let segs = vec![
            "https://example.com/abcdefghijk/lmn https://example.com/abcdefghijk/lmn".to_string(),
        ];
        let out = compress(segs, &mut urls);
        assert_eq!(urls.len(), 1);
        assert!(out[0].contains("⟦U0⟧"));
    }
}
