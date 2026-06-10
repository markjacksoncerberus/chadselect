//! XPath expression analysis used to keep the recursive parser off small stacks.
//!
//! xrust's parser-combinators recurse ~one frame per nested predicate /
//! function-arg level, overflowing small stacks (e.g. a 2 MiB tokio worker) at
//! ~15 levels. [`nesting_depth`] lets the caller route deep expressions to a
//! large-stack thread instead of crashing the process.
//!
//! (Positional-predicate rewriting used to live here too, but the forked xrust
//! engine now evaluates positional predicates correctly, so it was removed.)

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
    fn depth_counts_nesting() {
        assert_eq!(nesting_depth("//a/b/c"), 0);
        assert_eq!(nesting_depth("//a[1]"), 1);
        assert_eq!(nesting_depth("//a[b[c[1]]]"), 3);
        assert_eq!(nesting_depth("//a[foo(bar(1))]"), 3);
        // Brackets inside a string literal don't count.
        assert_eq!(nesting_depth("//a[@x='[[[']"), 1);
    }
}
