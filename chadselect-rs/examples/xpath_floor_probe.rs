//! Confirm the per-query XPath floor: does a *trivial* selector cost the same
//! ~30 ms as a complex one on a big page? If yes, the cost is per-query
//! document overhead (ENode::root_of → build_order), not selector complexity.

use std::time::Instant;
use chadselect::ChadSelect;

fn page(cards: usize) -> String {
    let mut s = String::from("<!doctype html><html><body><div id=root>");
    for i in 0..cards {
        s.push_str(&format!(
            "<div class=card><span class=label>X</span><span class=value>{i}</span>\
             <ul><li>a</li><li>b</li><li>c</li></ul><p>txt {i}</p></div>"
        ));
    }
    s.push_str("</div></body></html>");
    s
}

fn bench(cs: &ChadSelect, q: &str, iters: u32) -> f64 {
    let _ = cs.query(-1, q); // warm
    let t = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(cs.query(-1, q));
    }
    t.elapsed().as_secs_f64() * 1e3 / iters as f64
}

fn main() {
    for cards in [500usize, 1000, 2000, 4000] {
        let html = page(cards);
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        let _ = cs.query(-1, "css:#root");
        let nodes = cards * 8; // rough
        println!("\n── {cards} cards (~{nodes} nodes, {} bytes) ──", html.len());
        println!("  css:title (no xpath)          {:>8.3} ms", bench(&cs, "css:title", 50));
        println!("  css:.value (deep css)         {:>8.3} ms", bench(&cs, "css:.value", 50));
        println!("  xpath://title (TRIVIAL)       {:>8.3} ms", bench(&cs, "xpath://title", 20));
        println!("  xpath:/html/body (2 steps)    {:>8.3} ms", bench(&cs, "xpath:/html/body", 20));
        println!("  xpath://span[@class='value']  {:>8.3} ms", bench(&cs, "xpath://span[@class='value']", 20));
        println!("  xpath://*[text()='a']/.. (cmplx) {:>5.3} ms", bench(&cs, "xpath://*[text()='a']/following-sibling::*", 20));
    }
}
