//! A small s-expression reader for solver output.
//!
//! It handles the lexical forms solvers use: `;` comments, `|quoted|`
//! symbols, `"strings"` with `""` escapes, and nested lists. Malformed input is
//! read as far as possible and never panics.

/// One node of a parsed s-expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SExpr {
    /// A symbol, numeral, decimal, keyword, or `#x`/`#b` literal. For a
    /// `|quoted|` symbol the bars are removed.
    Atom(String),
    /// A string literal, with `""` already unescaped.
    Str(String),
    /// A parenthesised list.
    List(Vec<SExpr>),
}

impl SExpr {
    /// Returns the symbol text if this node is an atom.
    pub(crate) fn atom(&self) -> Option<&str> {
        match self {
            SExpr::Atom(s) => Some(s),
            _ => None,
        }
    }

    /// Writes the node back as SMT-LIB2 text.
    pub(crate) fn to_text(&self) -> String {
        match self {
            SExpr::Atom(s) => {
                if s.is_empty() || s.contains(|c: char| c.is_whitespace() || "()|\"".contains(c)) {
                    format!("|{s}|")
                } else {
                    s.clone()
                }
            }
            SExpr::Str(s) => format!("\"{}\"", s.replace('"', "\"\"")),
            SExpr::List(items) => {
                let parts: Vec<String> = items.iter().map(SExpr::to_text).collect();
                format!("({})", parts.join(" "))
            }
        }
    }
}

/// Reads every top-level expression in `src`.
pub(crate) fn parse_all(src: &str) -> Vec<SExpr> {
    let mut stack: Vec<Vec<SExpr>> = vec![Vec::new()];
    let mut chars = src.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            ';' => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '(' => {
                chars.next();
                stack.push(Vec::new());
            }
            ')' => {
                chars.next();
                // An unmatched `)` at the top level is ignored.
                if stack.len() > 1 {
                    let finished = SExpr::List(stack.pop().unwrap_or_default());
                    push_into_current(&mut stack, finished);
                }
            }
            '"' => {
                chars.next();
                let s = read_string(&mut chars);
                push_into_current(&mut stack, SExpr::Str(s));
            }
            '|' => {
                chars.next();
                let mut name = String::new();
                for c in chars.by_ref() {
                    if c == '|' {
                        break;
                    }
                    name.push(c);
                }
                push_into_current(&mut stack, SExpr::Atom(name));
            }
            _ => {
                let mut atom = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || "();\"|".contains(c) {
                        break;
                    }
                    atom.push(c);
                    chars.next();
                }
                push_into_current(&mut stack, SExpr::Atom(atom));
            }
        }
    }

    // Close any lists left open by truncated input.
    while stack.len() > 1 {
        let finished = SExpr::List(stack.pop().unwrap_or_default());
        push_into_current(&mut stack, finished);
    }
    stack.pop().unwrap_or_default()
}

fn push_into_current(stack: &mut [Vec<SExpr>], node: SExpr) {
    if let Some(top) = stack.last_mut() {
        top.push(node);
    }
}

/// Reads a string literal body after its opening quote. `""` is one quote.
fn read_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            if chars.peek() == Some(&'"') {
                chars.next();
                out.push('"');
            } else {
                break;
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(s: &str) -> SExpr {
        SExpr::Atom(s.into())
    }

    #[test]
    fn reads_nested_lists() {
        let nodes = parse_all("(a (b c) ())");
        assert_eq!(
            nodes,
            vec![SExpr::List(vec![
                atom("a"),
                SExpr::List(vec![atom("b"), atom("c")]),
                SExpr::List(vec![]),
            ])]
        );
    }

    #[test]
    fn skips_comments_and_whitespace() {
        let nodes = parse_all("; header\n(x ; inline\n  1)\n");
        assert_eq!(nodes, vec![SExpr::List(vec![atom("x"), atom("1")])]);
    }

    #[test]
    fn reads_quoted_symbols_and_strings() {
        let nodes = parse_all(r#"|my var| "a""b" |with (paren)|"#);
        assert_eq!(
            nodes,
            vec![
                atom("my var"),
                SExpr::Str("a\"b".into()),
                atom("with (paren)")
            ]
        );
    }

    #[test]
    fn tolerates_truncated_and_unbalanced_input() {
        assert_eq!(
            parse_all("(define-fun x () Int 3"),
            vec![SExpr::List(vec![
                atom("define-fun"),
                atom("x"),
                SExpr::List(vec![]),
                atom("Int"),
                atom("3"),
            ])]
        );
        assert_eq!(parse_all(") (a)"), vec![SExpr::List(vec![atom("a")])]);
        assert_eq!(
            parse_all("\"unterminated"),
            vec![SExpr::Str("unterminated".into())]
        );
    }

    #[test]
    fn round_trips_to_text() {
        let src = "(define-fun |my var| () (_ BitVec 8) #xf0)";
        let node = &parse_all(src)[0];
        assert_eq!(node.to_text(), src);
    }
}
