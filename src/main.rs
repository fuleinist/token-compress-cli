//! token-compress CLI. Exit codes: 0 ok, 2 error.

use std::io::{Read, Write};
use std::process::ExitCode;

use token_compress_cli::map::{parse_inline, write_inline, TcMap};
use token_compress_cli::{compress_text, est_tokens, expand_text, Options, TcError};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str =
    "token-compress — reversible token compression for LLM prompts, logs, and outputs

USAGE:
    token-compress compress [FILE] [OPTIONS]
    token-compress expand   [FILE] [OPTIONS]
    token-compress stats    [FILE]
    token-compress --version | --help

compress reads FILE (or stdin), writes compressed text to stdout (or -o FILE).
The reversibility map is embedded as an inline footer by default; use -m FILE
to write it to a sidecar instead (body stays footer-free).
expand restores the original text byte-for-byte.

OPTIONS:
    -o, --output <FILE>   write output to FILE instead of stdout
    -m, --map <FILE>      compress: write map to FILE; expand: read map from FILE
        --no-ws           disable whitespace-squeeze stage
        --no-dedup        disable repeat-dedup stage
        --no-urls         disable URL-shortening stage
        --no-filler       disable filler-phrase stage
        --inline          force inline footer map (default already)
        --json            print stats as JSON to stderr

Stats are printed to stderr on compress; stdout stays pipe-clean.";

struct Args {
    file: Option<String>,
    output: Option<String>,
    map: Option<String>,
    no_ws: bool,
    no_dedup: bool,
    no_urls: bool,
    no_filler: bool,
    json: bool,
}

fn parse_args(rest: &[String]) -> Result<Args, TcError> {
    let mut a = Args {
        file: None,
        output: None,
        map: None,
        no_ws: false,
        no_dedup: false,
        no_urls: false,
        no_filler: false,
        json: false,
    };
    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        match arg.as_str() {
            "-o" | "--output" => {
                i += 1;
                a.output = Some(
                    rest.get(i)
                        .ok_or_else(|| TcError::new("missing value for --output"))?
                        .clone(),
                );
            }
            "-m" | "--map" => {
                i += 1;
                a.map = Some(
                    rest.get(i)
                        .ok_or_else(|| TcError::new("missing value for --map"))?
                        .clone(),
                );
            }
            "--no-ws" => a.no_ws = true,
            "--no-dedup" => a.no_dedup = true,
            "--no-urls" => a.no_urls = true,
            "--no-filler" => a.no_filler = true,
            "--inline" => {} // default behavior; accepted for compatibility
            "--json" => a.json = true,
            other => {
                if other.starts_with('-') {
                    return Err(TcError::new(format!("unknown option: {other}")));
                }
                if a.file.is_some() {
                    return Err(TcError::new(format!("unexpected argument: {other}")));
                }
                a.file = Some(other.to_string());
            }
        }
        i += 1;
    }
    Ok(a)
}

fn read_input(file: &Option<String>) -> Result<String, TcError> {
    let bytes = match file {
        Some(path) => {
            std::fs::read(path).map_err(|e| TcError::new(format!("cannot read {path}: {e}")))?
        }
        None => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .map_err(|e| TcError::new(format!("cannot read stdin: {e}")))?;
            buf
        }
    };
    String::from_utf8(bytes).map_err(|_| TcError::new("input is not valid UTF-8"))
}

fn write_output(out: &Option<String>, content: &str) -> Result<(), TcError> {
    match out {
        Some(path) => std::fs::write(path, content)
            .map_err(|e| TcError::new(format!("cannot write {path}: {e}")))?,
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(content.as_bytes())
                .map_err(|e| TcError::new(format!("cannot write stdout: {e}")))?;
        }
    }
    Ok(())
}

fn print_stats(input: &str, transmitted: &str, stages: &str, json: bool) {
    let (ic, oc) = (input.len(), transmitted.len());
    let (it, ot) = (est_tokens(input), est_tokens(transmitted));
    if json {
        let ratio = if ot == 0 { 0.0 } else { it as f64 / ot as f64 };
        eprintln!(
            "{}",
            serde_json::json!({
                "in_chars": ic, "out_chars": oc,
                "est_tokens_in": it, "est_tokens_out": ot,
                "token_ratio": ratio, "stages": stages,
            })
        );
    } else {
        eprintln!("token-compress: {ic}→{oc} chars, ~{it}→~{ot} est tokens (stages: {stages})");
    }
}

fn run_compress(a: Args) -> Result<(), TcError> {
    let input = read_input(&a.file)?;
    let opts = Options {
        ws: !a.no_ws,
        dedup: !a.no_dedup,
        urls: !a.no_urls,
        filler: !a.no_filler,
    };
    let (body, map) = compress_text(&input, &opts);

    let transmitted = match &a.map {
        Some(path) => {
            let json = serde_json::to_string(&map)?;
            std::fs::write(path, json)
                .map_err(|e| TcError::new(format!("cannot write map {path}: {e}")))?;
            body.clone()
        }
        None => write_inline(&body, &map)?,
    };

    write_output(&a.output, &transmitted)?;
    print_stats(&input, &transmitted, &map.stages.join(","), a.json);
    Ok(())
}

fn run_expand(a: Args) -> Result<(), TcError> {
    let input = read_input(&a.file)?;
    let (body, map) = match &a.map {
        Some(path) => {
            let json = std::fs::read_to_string(path)
                .map_err(|e| TcError::new(format!("cannot read map {path}: {e}")))?;
            let map: TcMap = serde_json::from_str(&json)?;
            (input, map)
        }
        None => parse_inline(&input)?,
    };
    let restored = expand_text(&body, &map)?;
    write_output(&a.output, &restored)?;
    Ok(())
}

fn run_stats(a: Args) -> Result<(), TcError> {
    let input = read_input(&a.file)?;
    let t = est_tokens(&input);
    if a.json {
        eprintln!(
            "{}",
            serde_json::json!({"chars": input.len(), "est_tokens": t})
        );
    } else {
        eprintln!("token-compress: {} chars, ~{} est tokens", input.len(), t);
    }
    Ok(())
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let result = match argv.first().map(String::as_str) {
        Some("--version") => {
            println!("token-compress {VERSION}");
            return ExitCode::SUCCESS;
        }
        Some("--help") | Some("-h") | None => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some("compress") => parse_args(&argv[1..]).and_then(run_compress),
        Some("expand") => parse_args(&argv[1..]).and_then(run_expand),
        Some("stats") => parse_args(&argv[1..]).and_then(run_stats),
        Some(other) => Err(TcError::new(format!("unknown command: {other}\n\n{USAGE}"))),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("token-compress: error: {}", e.msg);
            ExitCode::from(2)
        }
    }
}
