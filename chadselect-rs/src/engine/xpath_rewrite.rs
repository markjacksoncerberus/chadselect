//! XPath expression pre-processing to work around `xrust` XPath-conformance
//! bugs and the recursive parser's stack use.
//!
//! ## Positional predicates
//!
//! xrust mishandles positional predicates in three compounding ways:
//!  * a predicate whose value is a *number* is coerced with `to_bool()`
//!    (non-zero → true) instead of being compared to `position()` (XPath 1.0
//!    §2.4), so `[1]`, `[2]` keep **every** node;
//!  * inside a predicate `last()` returns `1` (it is evaluated against a
//!    single-item context), so `[last()]` is wrong;
//!  * a step's predicate is applied over the **flattened** node-set of all
//!    context nodes rather than per parent, so `//tr/td[1]` returns only the
//!    very first `td` instead of the first `td` of each row.
//!
//! All three are avoided by expressing positional predicates with **sibling
//! counting**, which each node evaluates relative to its own parent and which
//! uses only axes and `count()` that xrust evaluates correctly:
//!
//! ```text
//!   name[N]        ->  name[count(preceding-sibling::name)=N-1]
//!   name[last()]   ->  name[not(following-sibling::name)]
//!   name[last()-K] ->  name[count(following-sibling::name)=K]
//! ```
//!
//! This is applied only when the predicate body is purely positional and the
//! step has a simple node test (`name`/`*`) reached via `/` or `//`. A bare
//! numeric predicate with no such node test (e.g. a chained `a[@x][2]`) falls
//! back to `position()=N` (correct for a single-parent context); other forms
//! are left unchanged. Boolean predicates are never touched.
//!
//! ## Parser stack depth
//!
//! xrust's parser-combinators recurse ~one frame (~130 KiB) per nesting level,
//! overflowing small stacks (e.g. a 2 MiB tokio worker) at ~15 levels.
//! [`nesting_depth`] lets the caller route deep expressions to a large-stack
//! thread instead of crashing the process.

/// Rewrite positional predicates into per-parent sibling-counting form.
/// Operates structurally (respecting quotes and nested brackets).
pub fn rewrite_positional_predicates(expr: &str) -> String {
    let chars: Vec<char> = expr.chars().collect();
    rewrite(&chars)
}

/// The positional meaning of a predicate body, if any.
enum Positional {
    /// `[N]` / `[position()=N]` — the N-th (1-based).
    Nth(u64),
    /// `[last()]` / `[position()=last()]`.
    Last,
    /// `[last()-K]` — the K-th from the end.
    LastMinus(u64),
}

fn rewrite(cs: &[char]) -> String {
    let mut out = String::with_capacity(cs.len() + 24);
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\'' | '"' => {
                out.push(c);
                i += 1;
                while i < cs.len() {
                    let d = cs[i];
                    out.push(d);
                    i += 1;
                    if d == c {
                        break;
                    }
                }
            }
            '[' => {
                let start = i + 1;
                let mut depth = 1usize;
                let mut j = start;
                let mut quote: Option<char> = None;
                while j < cs.len() {
                    let cj = cs[j];
                    match quote {
                        Some(q) => {
                            if cj == q {
                                quote = None;
                            }
                        }
                        None => match cj {
                            '\'' | '"' => quote = Some(cj),
                            '[' => depth += 1,
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        },
                    }
                    j += 1;
                }
                let end = j.min(cs.len());
                // Recurse so nested predicates (e.g. `a[b[1]]`) are handled too.
                let inner = rewrite(&cs[start..end]);
                out.push_str(&rewrite_predicate(&inner, &out));
                i = if j < cs.len() { j + 1 } else { j };
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Produce the replacement bracket group for one predicate whose (already
/// rewritten) body is `inner`, given the output emitted so far (`prefix`),
/// which ends with the step this predicate attaches to.
fn rewrite_predicate(inner: &str, prefix: &str) -> String {
    let Some(kind) = classify(inner.trim()) else {
        return format!("[{inner}]"); // boolean predicate — leave as-is
    };
    match (trailing_node_test(prefix), kind) {
        (Some(nt), Positional::Nth(n)) => {
            format!("[count(preceding-sibling::{nt})={}]", n - 1)
        }
        (Some(nt), Positional::Last) => format!("[not(following-sibling::{nt})]"),
        (Some(nt), Positional::LastMinus(k)) => {
            format!("[count(following-sibling::{nt})={k}]")
        }
        // No simple node test (e.g. chained predicate): best-effort numeric
        // fallback (correct for a single-parent context); leave last()-forms.
        (None, Positional::Nth(n)) => format!("[position()={n}]"),
        (None, _) => format!("[{inner}]"),
    }
}

/// Classify a predicate body as a positional form, or `None` if not purely
/// positional (i.e. a boolean predicate that must be left untouched).
fn classify(body: &str) -> Option<Positional> {
    let s = compact(body);
    if let Some(n) = parse_pos_int(&s) {
        return Some(Positional::Nth(n));
    }
    if let Some(rest) = s.strip_prefix("position()=") {
        if let Some(n) = parse_pos_int(rest) {
            return Some(Positional::Nth(n));
        }
        if rest == "last()" {
            return Some(Positional::Last);
        }
        return None;
    }
    if s == "last()" {
        return Some(Positional::Last);
    }
    if let Some(rest) = s.strip_prefix("last()-") {
        if let Some(k) = parse_pos_int(rest) {
            return Some(Positional::LastMinus(k));
        }
    }
    None
}

/// Parse a strictly-positive integer (`"0"` is not positional — `[0]` selects
/// nothing — so it is rejected and left for xrust to handle).
fn parse_pos_int(s: &str) -> Option<u64> {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        match s.parse::<u64>() {
            Ok(n) if n >= 1 => Some(n),
            _ => None,
        }
    } else {
        None
    }
}

fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// If `prefix` ends with a step bearing a simple node test (`…/name` or `…/*`,
/// single or double slash, no trailing predicate or function call), return that
/// node test. Returns `None` for parenthesised expressions (`(…)[1]` is global,
/// not per-parent), chained predicates, etc.
fn trailing_node_test(prefix: &str) -> Option<String> {
    let b: Vec<char> = prefix.chars().collect();
    let is_nt = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '*');

    let end = b.len();
    let mut start = end;
    while start > 0 && is_nt(b[start - 1]) {
        start -= 1;
    }
    if start == end || start == 0 || b[start - 1] != '/' {
        return None;
    }
    Some(b[start..end].iter().collect())
}

/// Maximum simultaneous nesting depth of `(` and `[` (outside string literals).
pub fn nesting_depth(expr: &str) -> usize {
    let mut depth = 0usize;
    let mut max = 0usize;
    let mut quote: Option<char> = None;
    for c in expr.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                '\'' | '"' => quote = Some(c),
                '(' | '[' => {
                    depth += 1;
                    max = max.max(depth);
                }
                ')' | ']' => depth = depth.saturating_sub(1),
                _ => {}
            },
        }
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nth_uses_preceding_sibling_count() {
        assert_eq!(
            rewrite_positional_predicates("//tr/td[1]"),
            "//tr/td[count(preceding-sibling::td)=0]"
        );
        assert_eq!(
            rewrite_positional_predicates("//tr/td[2]/text()"),
            "//tr/td[count(preceding-sibling::td)=1]/text()"
        );
        assert_eq!(
            rewrite_positional_predicates("//ul/li[position()=3]"),
            "//ul/li[count(preceding-sibling::li)=2]"
        );
        assert_eq!(
            rewrite_positional_predicates("//div//*[2]"),
            "//div//*[count(preceding-sibling::*)=1]"
        );
    }

    #[test]
    fn last_uses_following_sibling() {
        assert_eq!(
            rewrite_positional_predicates("//div/span[last()]"),
            "//div/span[not(following-sibling::span)]"
        );
        assert_eq!(
            rewrite_positional_predicates("//ul/li[last()-1]"),
            "//ul/li[count(following-sibling::li)=1]"
        );
        assert_eq!(
            rewrite_positional_predicates("//a/b[position()=last()]"),
            "//a/b[not(following-sibling::b)]"
        );
    }

    #[test]
    fn fallback_when_no_simple_node_test() {
        // Chained predicate: node test not adjacent → best-effort position().
        assert_eq!(
            rewrite_positional_predicates("//span[@x][1]"),
            "//span[@x][position()=1]"
        );
        // Parenthesised (global) positional — node test not adjacent.
        assert_eq!(
            rewrite_positional_predicates("(//span)[1]"),
            "(//span)[position()=1]"
        );
    }

    #[test]
    fn leaves_boolean_predicates_untouched() {
        for q in [
            "//div[@class='2']",
            "//a[@id=\"1\"]",
            "//span[contains(., '1')]",
            "//li[text()='3']",
            "//x[@n>1]",
            "//a[@t='x[1]']",
        ] {
            assert_eq!(rewrite_positional_predicates(q), q, "must not rewrite: {q}");
        }
    }

    #[test]
    fn depth_counts_nesting() {
        assert_eq!(nesting_depth("//a/b/c"), 0);
        assert_eq!(nesting_depth("//a[1]"), 1);
        assert_eq!(nesting_depth("//a[b[c[1]]]"), 3);
        assert_eq!(nesting_depth("//a[foo(bar(1))]"), 3);
        assert_eq!(nesting_depth("//a[@x='[[[']"), 1);
    }
}
