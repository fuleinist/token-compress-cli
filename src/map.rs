//! Map format v1: JSON reversibility map + inline footer framing.

use crate::error::TcError;
use crate::ws::WsMap;
use serde::{Deserialize, Serialize};

pub const MAP_VERSION: u32 = 1;
pub const FOOTER_OPEN_PREFIX: &str = "⟦TCMAP v1 stages=";
pub const FOOTER_CLOSE: &str = "⟦/TCMAP⟧";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcMap {
    pub v: u32,
    /// Stage names in compress order; expand applies inverses in reverse.
    /// Known: ws, dedup, urls, filler.
    pub stages: Vec<String>,
    #[serde(default, skip_serializing_if = "WsMap::is_empty")]
    pub ws: WsMap,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<String>,
    /// Filler phrase ids used (stats only; phrase list is fixed for v1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filler_used: Vec<u32>,
    /// Separator added before the inline footer ("" or "\n"); inline mode only.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sep: String,
}

impl TcMap {
    pub fn new() -> Self {
        Self {
            v: MAP_VERSION,
            stages: Vec::new(),
            ws: WsMap::default(),
            urls: Vec::new(),
            filler_used: Vec::new(),
            sep: String::new(),
        }
    }
}

impl Default for TcMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Append the inline footer to the compressed body.
pub fn write_inline(body: &str, map: &TcMap) -> Result<String, TcError> {
    let sep = if body.is_empty() || body.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let mut m = map.clone();
    m.sep = sep.to_string();
    let json = serde_json::to_string(&m)?;
    Ok(format!(
        "{body}{sep}{}{}⟧\n{json}\n{FOOTER_CLOSE}\n",
        FOOTER_OPEN_PREFIX,
        m.stages.join(",")
    ))
}

/// Split inline-framed text into (body, map). Errors if the footer is absent
/// or malformed (exit-code-2 territory; never silent data loss).
pub fn parse_inline(text: &str) -> Result<(String, TcMap), TcError> {
    let close_pos = text.rfind(FOOTER_CLOSE).ok_or_else(|| {
        TcError::new("no inline map found: missing ⟦/TCMAP⟧ footer (use -m with a map file?)")
    })?;
    let open_pos = text[..close_pos]
        .rfind(FOOTER_OPEN_PREFIX)
        .ok_or_else(|| TcError::new("no inline map found: missing ⟦TCMAP v1⟧ header"))?;

    let header_end = text[open_pos..]
        .find('⟧')
        .ok_or_else(|| TcError::new("corrupt map: unterminated footer header"))?
        + open_pos;
    let _stages_str = &text[open_pos + FOOTER_OPEN_PREFIX.len()..header_end];

    let json_start = header_end + '⟧'.len_utf8();
    let json_start = text[json_start..]
        .strip_prefix('\n')
        .map(|_| json_start + 1)
        .unwrap_or(json_start);
    let json_str = &text[json_start..close_pos];
    let json_str = json_str.strip_suffix('\n').unwrap_or(json_str);

    let map: TcMap =
        serde_json::from_str(json_str).map_err(|e| TcError::new(format!("corrupt map: {e}")))?;
    if map.v != MAP_VERSION {
        return Err(TcError::new(format!(
            "unsupported map version {} (this build supports v{MAP_VERSION})",
            map.v
        )));
    }

    // Body: everything before the footer, minus the separator we added.
    if open_pos < map.sep.len() {
        return Err(TcError::new("corrupt map: footer overlaps body"));
    }
    let body = text[..open_pos - map.sep.len()].to_string();

    Ok((body, map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_roundtrip() {
        let mut map = TcMap::new();
        map.stages.push("ws".into());
        for body in ["hello", "hello\n", ""] {
            let framed = write_inline(body, &map).unwrap();
            let (got_body, got_map) = parse_inline(&framed).unwrap();
            assert_eq!(got_body, body);
            assert_eq!(got_map.stages, map.stages);
        }
    }

    #[test]
    fn rejects_missing_footer() {
        assert!(parse_inline("just some text").is_err());
    }
}
