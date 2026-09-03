//! Expansion pipeline: inverse stages in reverse order, then unescape.

use crate::map::TcMap;
use crate::TcError;
use crate::{dedup, escape, filler, urls, ws};

pub fn expand_text(body: &str, map: &TcMap) -> Result<String, TcError> {
    let mut segs: Vec<String> = body.split('\n').map(String::from).collect();

    for stage in map.stages.iter().rev() {
        match stage.as_str() {
            "filler" => segs = filler::expand(segs)?,
            "urls" => segs = urls::expand(segs, &map.urls)?,
            "dedup" => segs = dedup::expand(segs)?,
            "ws" => segs = ws::expand(segs, &map.ws)?,
            other => {
                return Err(TcError::new(format!(
                    "corrupt map: unknown stage '{other}'"
                )))
            }
        }
    }

    Ok(escape::unescape(&segs.join("\n")))
}
