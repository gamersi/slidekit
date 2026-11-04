//! Typst content (serialized to JSON) → HTML for speaker notes and plain text for titles.
//!
//! Covers the subset of markup that appears in notes: text, spaces, breaks, lists, strong,
//! emph, raw, smart quotes, simple math and links. Unknown elements fall back to their
//! `body`, `children` or `text` field.

use serde_json::Value;

/// Renders content as HTML.
pub fn to_html(content: &Value) -> String {
    let mut out = String::new();
    Renderer { open_quote: false }.render(content, &mut out);
    out.trim().to_owned()
}

/// Renders content as plain text (tags removed, whitespace collapsed).
pub fn to_text(content: &Value) -> String {
    let html = to_html(content);
    let mut text = String::with_capacity(html.len());
    let mut rest = html.as_str();
    // Escaped text never contains a raw `<`, so every `<` starts one of our own tags.
    while let Some(start) = rest.find('<') {
        text.push_str(&rest[..start]);
        let end = rest[start..]
            .find('>')
            .map_or(rest.len(), |i| start + i + 1);
        if matches!(&rest[start..end], "<br>" | "<li>") {
            text.push(' ');
        }
        rest = &rest[end..];
    }
    text.push_str(rest);
    let text = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

struct Renderer {
    /// Whether the last double smart quote opened a quotation.
    open_quote: bool,
}

impl Renderer {
    fn render(&mut self, c: &Value, out: &mut String) {
        match c {
            Value::Null => {}
            Value::String(s) => out.push_str(&escape(s)),
            Value::Array(items) => items.iter().for_each(|x| self.render(x, out)),
            Value::Object(map) => {
                let field = |key: &str| map.get(key).unwrap_or(&Value::Null);
                let text = || field("text").as_str().unwrap_or_default();
                match map.get("func").and_then(Value::as_str).unwrap_or_default() {
                    "sequence" => self.sequence(field("children"), out),
                    "text" | "symbol" => out.push_str(&escape(text())),
                    "space" => out.push(' '),
                    "linebreak" => out.push_str("<br>"),
                    "parbreak" => out.push_str("<br><br>"),
                    "item" | "enum.item" | "list.item" => {
                        out.push_str("<ul><li>");
                        self.render(field("body"), out);
                        out.push_str("</li></ul>");
                    }
                    "strong" => self.wrap("b", field("body"), out),
                    "emph" | "equation" => self.wrap("i", field("body"), out),
                    "raw" => {
                        out.push_str("<code>");
                        out.push_str(&escape(text()));
                        out.push_str("</code>");
                    }
                    "smartquote" => {
                        if field("double").as_bool().unwrap_or(true) {
                            self.open_quote = !self.open_quote;
                            out.push(if self.open_quote { '„' } else { '“' });
                        } else {
                            out.push_str("&#39;");
                        }
                    }
                    "frac" => {
                        self.render(field("num"), out);
                        out.push('/');
                        self.render(field("denom"), out);
                    }
                    "attach" => {
                        self.render(field("base"), out);
                        for (key, tag) in [("b", "sub"), ("t", "sup")] {
                            if !field(key).is_null() {
                                self.wrap(tag, field(key), out);
                            }
                        }
                    }
                    _ => {
                        if let Some(inner) = ["body", "children", "text"]
                            .iter()
                            .find_map(|k| map.get(*k))
                        {
                            self.render(inner, out);
                        }
                    }
                }
            }
            Value::Bool(_) | Value::Number(_) => {}
        }
    }

    fn wrap(&mut self, tag: &str, c: &Value, out: &mut String) {
        out.push_str(&format!("<{tag}>"));
        self.render(c, out);
        out.push_str(&format!("</{tag}>"));
    }

    /// Renders children, grouping consecutive list items into one `<ul>`.
    fn sequence(&mut self, children: &Value, out: &mut String) {
        let mut items = String::new();
        for child in children.as_array().into_iter().flatten() {
            let func = child
                .get("func")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if matches!(func, "item" | "enum.item" | "list.item") {
                items.push_str("<li>");
                self.render(child.get("body").unwrap_or(&Value::Null), &mut items);
                items.push_str("</li>");
                continue;
            }
            if !items.is_empty() && matches!(func, "space" | "parbreak") {
                continue;
            }
            if !items.is_empty() {
                out.push_str("<ul>");
                out.push_str(&std::mem::take(&mut items));
                out.push_str("</ul>");
            }
            self.render(child, out);
        }
        if !items.is_empty() {
            out.push_str("<ul>");
            out.push_str(&items);
            out.push_str("</ul>");
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn text(s: &str) -> Value {
        json!({"func": "text", "text": s})
    }

    fn seq(children: Vec<Value>) -> Value {
        json!({"func": "sequence", "children": children})
    }

    #[test]
    fn plain_and_escaped() {
        assert_eq!(to_html(&Value::Null), "");
        assert_eq!(to_html(&json!("a < b")), "a &lt; b");
        assert_eq!(to_html(&text("R&D")), "R&amp;D");
    }

    #[test]
    fn inline_markup() {
        let c = seq(vec![
            json!({"func": "strong", "body": text("bold")}),
            json!({"func": "space"}),
            json!({"func": "emph", "body": text("it")}),
            json!({"func": "linebreak"}),
            json!({"func": "raw", "text": "x<y", "block": false}),
            json!({"func": "parbreak"}),
            json!({"func": "link", "dest": "https://x", "body": text("link")}),
        ]);
        assert_eq!(
            to_html(&c),
            "<b>bold</b> <i>it</i><br><code>x&lt;y</code><br><br>link"
        );
    }

    #[test]
    fn lists_are_grouped() {
        let item = |s| json!({"func": "item", "body": text(s)});
        let c = seq(vec![
            text("Intro"),
            json!({"func": "parbreak"}),
            item("one"),
            json!({"func": "space"}),
            item("two"),
            json!({"func": "parbreak"}),
            text("After"),
        ]);
        assert_eq!(
            to_html(&c),
            "Intro<br><br><ul><li>one</li><li>two</li></ul>After"
        );
        assert_eq!(to_html(&item("solo")), "<ul><li>solo</li></ul>");
        assert_eq!(to_html(&seq(vec![item("end")])), "<ul><li>end</li></ul>");
    }

    #[test]
    fn smart_quotes_alternate() {
        let q = || json!({"func": "smartquote", "double": true});
        let c = seq(vec![
            q(),
            text("a"),
            q(),
            json!({"func": "space"}),
            q(),
            text("b"),
            q(),
        ]);
        assert_eq!(to_html(&c), "„a“ „b“");
        assert_eq!(
            to_html(&json!({"func": "smartquote", "double": false})),
            "&#39;"
        );
    }

    #[test]
    fn math() {
        let frac = json!({"func": "frac", "num": text("1"), "denom": text("2")});
        let attach = json!({"func": "attach", "base": text("x"), "t": text("2")});
        let eq = json!({"func": "equation", "block": false, "body": seq(vec![frac, attach])});
        assert_eq!(to_html(&eq), "<i>1/2x<sup>2</sup></i>");
    }

    #[test]
    fn unknown_elements_fall_back() {
        assert_eq!(to_html(&json!({"func": "heading", "body": text("H")})), "H");
        assert_eq!(
            to_html(&json!({"func": "box", "children": [text("a"), text("b")]})),
            "ab"
        );
        assert_eq!(to_html(&json!({"func": "image", "source": "x.png"})), "");
    }

    #[test]
    fn titles_as_text() {
        let c = seq(vec![
            json!({"func": "strong", "body": text("A & B")}),
            json!({"func": "linebreak"}),
            text("next  line"),
        ]);
        assert_eq!(to_text(&c), "A & B next line");
        assert_eq!(to_text(&json!("<tag>")), "<tag>");
    }
}
