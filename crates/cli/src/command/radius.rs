use plumb::radius::{Refusal, Report, Request, Root};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Failed {
    schema: &'static str,
    product: String,
    ok: bool,
    refusal: Refusal,
}

pub struct Input {
    pub roots: Vec<String>,
    pub product: String,
    pub candidate: String,
    pub json: bool,
}

pub fn run(input: Input) -> i32 {
    let named = input
        .roots
        .iter()
        .map(|held| name(held))
        .collect::<Vec<_>>();
    let roots = named
        .iter()
        .map(|(label, path)| Root {
            label,
            path: path.as_path(),
        })
        .collect::<Vec<_>>();
    let request = Request {
        roots: &roots,
        product: &input.product,
        candidate: &input.candidate,
    };
    match plumb::radius::check(request) {
        Ok(report) => render(report, input.json),
        Err(refusal) => rejected(&input.product, refusal, input.json),
    }
}

fn name(held: &str) -> (String, PathBuf) {
    if let Some((label, path)) = held.split_once('=') {
        return (label.to_string(), PathBuf::from(path));
    }
    let path = PathBuf::from(held);
    let label = Path::new(held)
        .file_name()
        .map(|seen| seen.to_string_lossy().to_string())
        .unwrap_or_else(|| held.to_string());
    (label, path)
}

fn render(report: Report, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("radius report should encode")
        );
        return 0;
    }
    println!("plumb radius {} {}", report.product, report.candidate);
    println!();
    for seat in &report.seats {
        println!(
            "  {:8} {:10} {}",
            if seat.behind { "behind" } else { "current" },
            seat.resolution,
            seat.label
        );
    }
    for held in &report.blind {
        println!("  blind    {} [{}]", held.label, held.reason);
    }
    if report.seats.is_empty() && report.blind.is_empty() {
        println!("  no seat declares {}", report.product);
    }
    println!();
    println!(
        "  {} of {} behind {}, {} blind",
        report.behind,
        report.seats.len(),
        report.candidate,
        report.blind.len()
    );
    0
}

fn rejected(product: &str, refusal: Refusal, json: bool) -> i32 {
    if json {
        let failed = Failed {
            schema: plumb::radius::SCHEMA,
            product: product.to_string(),
            ok: false,
            refusal,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&failed).expect("radius refusal should encode")
        );
    } else {
        eprintln!("plumb radius: {refusal}");
    }
    1
}
