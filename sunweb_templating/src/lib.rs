//! A lightweight template engine for the SunWeb framework.
//!
//! You should not depend on this crate directly — use [`sunweb`] with the
//! `templating` feature instead, which re-exports everything from here.
//!
//! # Template Syntax
//!
//! | Syntax | Description |
//! |---|---|
//! | `{{ name }}` | Inserts a variable from the context |
//! | `{% if condition %}` ... `{% endif %}` | Conditional block |
//! | `{% for item in list %}` ... `{% endfor %}` | Loop over a list |
//!
//! # Example
//!
//! ```rust,ignore
//! use sunweb::{Context, Value, render};
//!
//! let mut ctx = Context::new();
//! ctx.insert("title".into(), Value::from("Hello"));
//! ctx.insert("show_footer".into(), Value::Bool(true));
//! ctx.insert("users".into(), Value::List(vec![
//!     [("user.name".into(), Value::from("Alice"))].into(),
//!     [("user.name".into(), Value::from("Bob"))].into(),
//! ]));
//!
//! // template: "<h1>{{ title }}</h1>"
//! //           "{% if show_footer %}<footer>bye</footer>{% endif %}"
//! //           "{% for user in users %}<p>{{ user.name }}</p>{% endfor %}"
//! ```

use std::collections::HashMap;
use sunweb_core::response_types::{HtmlResponse, TextResponse};

// ── AST ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum Node {
    Text(String),
    Variable(String),
    IfBlock {
        condition: String,
        body: Vec<Node>,
    },
    ForBlock {
        var: String,
        iterable: String,
        body: Vec<Node>,
    },
}

// ── Lexer tokens ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum Token<'a> {
    Text(&'a str),
    Variable(&'a str),   // {{ name }}
    BlockStart(&'a str), // {% if / for ... %}
    BlockEnd(&'a str),   // {% endif / endfor %}
}

// ── Public API ───────────────────────────────────────────────────────────────

/// A value that can be inserted into a [`Context`] and used in templates.
///
/// # Example
/// ```rust,ignore
/// use sunweb::Value;
///
/// let s = Value::from("hello");
/// let b = Value::Bool(true);
/// let list = Value::List(vec![
///     [("item.name".into(), Value::from("Alice"))].into(),
/// ]);
/// ```
#[derive(Debug, Clone)]
pub enum Value {
    /// A string value, rendered directly into the template.
    Str(String),
    /// A boolean value, used in `{% if %}` conditions.
    Bool(bool),
    /// A list of row maps, used in `{% for %}` loops.
    ///
    /// Each entry is a map of `"var.field"` keys to [`Value`]s,
    /// accessible in templates as `{{ var.field }}`.
    List(Vec<HashMap<String, Value>>),
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

/// A map of variable names to [`Value`]s passed into a template.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{Context, Value};
///
/// let mut ctx = Context::new();
/// ctx.insert("name".into(), Value::from("Alice"));
/// ctx.insert("logged_in".into(), Value::Bool(true));
/// ```
pub type Context = HashMap<String, Value>;

/// Renders a template string with the given context and returns an [`HtmlResponse`].
///
/// You typically don't call this directly — use the [`render!`] macro instead,
/// which calls this function and returns the response from your route handler.
///
/// # Panics
/// Panics if the template contains unclosed `{{`, `{%`, or mismatched
/// `{% if %}`/`{% for %}` blocks.
pub fn render_response(template: &str, ctx: &Context) -> HtmlResponse {
    let body = render(template, ctx);
    HtmlResponse::ok(body)
}

/// Renders a template string with the given context and returns the output as a [`String`].
///
/// Prefer [`render_response`] or the [`render!`] macro in route handlers.
///
/// # Panics
/// Panics if the template contains unclosed `{{`, `{%`, or mismatched
/// `{% if %}`/`{% for %}` blocks.
pub fn render(template: &str, ctx: &Context) -> String {
    let tokens = lex(template);
    let nodes = parse(&tokens);
    render_nodes(&nodes, ctx)
}

// ── Internals ────────────────────────────────────────────────────────────────

fn render_nodes(nodes: &[Node], ctx: &Context) -> String {
    let mut out = String::new();

    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),

            Node::Variable(key) => {
                if let Some(Value::Str(s)) = ctx.get(key) {
                    out.push_str(s);
                }
            }

            Node::IfBlock { condition, body } => {
                let truthy = match ctx.get(condition.as_str()) {
                    Some(Value::Bool(b)) => *b,
                    Some(Value::Str(s)) => !s.is_empty(),
                    _ => false,
                };
                if truthy {
                    out.push_str(&render_nodes(body, ctx));
                }
            }

            Node::ForBlock {
                var,
                iterable,
                body,
            } => {
                if let Some(Value::List(items)) = ctx.get(iterable.as_str()) {
                    for item in items {
                        let mut inner_ctx = ctx.clone();
                        inner_ctx.extend(
                            item.iter()
                                .map(|(k, v)| (format!("{}.{}", var, k), v.clone())),
                        );
                        out.push_str(&render_nodes(body, &inner_ctx));
                    }
                }
            }
        }
    }

    out
}

fn lex(input: &'_ str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut rest = input;

    while !rest.is_empty() {
        if let Some(start) = rest.find("{{") {
            if start > 0 {
                tokens.push(Token::Text(&rest[..start]));
            }
            let end = rest[start..].find("}}").expect("unclosed {{") + start;
            let expr = rest[start + 2..end].trim();
            tokens.push(Token::Variable(expr));
            rest = &rest[end + 2..];
        } else if let Some(start) = rest.find("{%") {
            if start > 0 {
                tokens.push(Token::Text(&rest[..start]));
            }
            let end = rest[start..].find("%}").expect("unclosed {%") + start;
            let expr = rest[start + 2..end].trim();
            if expr.starts_with("end") {
                tokens.push(Token::BlockEnd(expr));
            } else {
                tokens.push(Token::BlockStart(expr));
            }
            rest = &rest[end + 2..];
        } else {
            tokens.push(Token::Text(rest));
            break;
        }
    }

    tokens
}

fn parse(tokens: &[Token]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Text(t) => nodes.push(Node::Text(t.to_string())),
            Token::Variable(v) => nodes.push(Node::Variable(v.to_string())),
            Token::BlockStart(expr) => {
                if let Some(stripped) = expr.strip_prefix("for ") {
                    let condition = stripped.trim().to_string();
                    let (body_tokens, consumed) = collect_until(&tokens[i + 1..], "endif");
                    nodes.push(Node::IfBlock {
                        condition,
                        body: parse(body_tokens),
                    });
                    i += consumed;
                } else if let Some(stripped) = expr.strip_prefix("for ") {
                    let parts: Vec<&str> = stripped.splitn(3, ' ').collect();
                    let var = parts[0].to_string();
                    let iterable = parts[2].to_string();
                    let (body_tokens, consumed) = collect_until(&tokens[i + 1..], "endfor");
                    nodes.push(Node::ForBlock {
                        var,
                        iterable,
                        body: parse(body_tokens),
                    });
                    i += consumed;
                }
            }
            Token::BlockEnd(_) => break,
        }
        i += 1;
    }

    nodes
}

fn collect_until<'a>(tokens: &'a [Token<'a>], end_tag: &str) -> (&'a [Token<'a>], usize) {
    let mut depth = 0;

    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::BlockStart(expr) => {
                if expr.starts_with("if ") || expr.starts_with("for ") {
                    depth += 1;
                }
            }
            Token::BlockEnd(expr) => {
                if expr.trim() == end_tag {
                    if depth == 0 {
                        return (&tokens[..i], i + 1);
                    } else {
                        depth -= 1;
                    }
                }
            }
            _ => {}
        }
    }

    panic!("unclosed block: expected {{{{% {end_tag} %}}}}");
}
