//! Find where chadpath's recursive parser actually overflows a given stack, so
//! the inline-depth bound in xpath.rs is a measured number, not a guess.
//!
//! Usage: `parser_depth_probe <nesting_depth> <stack_bytes>`
//! Prints "OK ..." and exits 0 if it parses without overflowing; a stack
//! overflow aborts the process (non-zero) with "has overflowed its stack".

use std::rc::Rc;
use scraper::Html;
use chadselect::engine::xpath_eval;

fn main() {
    let depth: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let stack: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(2 * 1024 * 1024);

    let mode = std::env::args().nth(3).unwrap_or_default();
    let expr = if mode == "flat" {
        // A long but UN-nested path: `depth` steps. If `*`-repetition is
        // iterative (as combinator grammars usually compile it), this should
        // never overflow regardless of length → proves length is not a
        // recursion driver and the `len>N` heuristic is bogus.
        let mut e = String::from("//a");
        for _ in 0..depth {
            e.push_str("/a");
        }
        e
    } else {
        // //a[.[.[ … ]]] — `depth` nested predicates, each re-entering the full
        // expression grammar (the worst case for a combinator parser).
        let mut e = String::from("//a");
        for _ in 0..depth {
            e.push_str("[.");
        }
        e.push_str("[.]");
        for _ in 0..depth {
            e.push(']');
        }
        e
    };

    let h = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let doc = Rc::new(Html::parse_document("<html><body><a/></body></html>"));
            let r = xpath_eval::evaluate(&doc, &expr);
            println!("OK depth={depth} stack={stack} matches={}", r.len());
        })
        .unwrap();
    let _ = h.join();
}
