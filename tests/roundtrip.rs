//! Integration tests: byte-for-byte round-trip on fixtures, ratio acceptance,
//! corrupt-map handling (SPEC.md A1–A4).

use std::path::Path;

use token_compress_cli::map::{parse_inline, write_inline};
use token_compress_cli::{compress_text, est_tokens, expand_text, Options};

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("missing fixture {name}: {e}"))
}

fn assert_roundtrip(input: &str, opts: &Options, label: &str) {
    // Inline (footer) mode.
    let (body, map) = compress_text(input, opts);
    let framed = write_inline(&body, &map).unwrap();
    let (body2, map2) = parse_inline(&framed).unwrap();
    let restored = expand_text(&body2, &map2).unwrap();
    assert_eq!(restored, input, "inline round-trip failed: {label}");

    // Sidecar mode (body without footer).
    let restored2 = expand_text(&body, &map).unwrap();
    assert_eq!(restored2, input, "sidecar round-trip failed: {label}");
}

const FIXTURES: &[&str] = &[
    "prose.txt",
    "code.rs",
    "log.txt",
    "chat.json",
    "markers.txt",
    "cjk.txt",
    "filler.txt",
    "crlf.txt",
];

#[test]
fn a1_roundtrip_all_stages() {
    let opts = Options::default();
    for f in FIXTURES {
        assert_roundtrip(&fixture(f), &opts, f);
    }
}

#[test]
fn a1_roundtrip_no_stages() {
    let opts = Options {
        ws: false,
        dedup: false,
        urls: false,
        filler: false,
    };
    for f in FIXTURES {
        assert_roundtrip(&fixture(f), &opts, f);
    }
}

#[test]
fn a1_roundtrip_each_stage_isolated() {
    for f in FIXTURES {
        let input = fixture(f);
        for stage in ["ws", "dedup", "urls", "filler"] {
            let opts = Options {
                ws: stage == "ws",
                dedup: stage == "dedup",
                urls: stage == "urls",
                filler: stage == "filler",
            };
            assert_roundtrip(&input, &opts, &format!("{f} [{stage}]"));
        }
    }
}

#[test]
fn a2_log_ratio_at_least_1_3() {
    let input = fixture("log.txt");
    let (body, map) = compress_text(&input, &Options::default());
    let framed = write_inline(&body, &map).unwrap();
    let ratio = est_tokens(&input) as f64 / est_tokens(&framed) as f64;
    assert!(
        ratio >= 1.3,
        "log fixture ratio {ratio:.2}× below 1.3× acceptance (A2)"
    );
}

#[test]
fn a3_echo_pipe_roundtrip() {
    let input = "x\n";
    assert_roundtrip(input, &Options::default(), "echo x");
    let input2 = "hello world";
    assert_roundtrip(input2, &Options::default(), "no trailing newline");
}

#[test]
fn a4_corrupt_map_errors() {
    // REP marker without a following line.
    let bad_body = "⟦REP 3×⟧";
    let mut map = token_compress_cli::TcMap::new();
    map.stages.push("dedup".into());
    assert!(expand_text(bad_body, &map).is_err());

    // Unknown stage name.
    let mut map2 = token_compress_cli::TcMap::new();
    map2.stages.push("bogus".into());
    assert!(expand_text("anything", &map2).is_err());

    // URL id outside map.
    let mut map3 = token_compress_cli::TcMap::new();
    map3.stages.push("urls".into());
    assert!(expand_text("see ⟦U5⟧ here", &map3).is_err());

    // Missing footer entirely.
    assert!(parse_inline("no footer at all").is_err());
}
