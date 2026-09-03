//! Compression pipeline. Order: escape -> ws -> dedup -> urls -> filler.

use crate::map::TcMap;
use crate::{dedup, escape, filler, urls, ws};

#[derive(Debug, Clone)]
pub struct Options {
    pub ws: bool,
    pub dedup: bool,
    pub urls: bool,
    pub filler: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            ws: true,
            dedup: true,
            urls: true,
            filler: true,
        }
    }
}

/// Compress `input`, returning the compressed body (without footer) and the map.
pub fn compress_text(input: &str, opts: &Options) -> (String, TcMap) {
    let mut map = TcMap::new();
    let escaped = escape::escape(input);
    let mut segs: Vec<String> = escaped.split('\n').map(String::from).collect();

    if opts.ws {
        let (s, m) = ws::compress(segs);
        segs = s;
        map.ws = m;
        map.stages.push("ws".into());
    }
    if opts.dedup {
        segs = dedup::compress(segs);
        map.stages.push("dedup".into());
    }
    if opts.urls {
        segs = urls::compress(segs, &mut map.urls);
        map.stages.push("urls".into());
    }
    if opts.filler {
        segs = filler::compress(segs, &mut map.filler_used);
        map.stages.push("filler".into());
    }

    (segs.join("\n"), map)
}
