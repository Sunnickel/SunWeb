//! A lightweight Jinja2-inspired template engine for the SunWeb framework.
//!
//! You should not depend on this crate directly — use [`sunweb`] with the
//! `templating` feature instead, which re-exports everything from here.
//!
//! # Template Syntax
//!
//! | Syntax | Description |
//! |---|---|
//! | `{{ name }}` | Inserts a variable (HTML-escaped by default) |
//! | `{{ name \| safe }}` | Inserts a variable without escaping |
//! | `{{ name \| upper }}` | Applies a filter |
//! | `{{ name \| default("x") }}` | Filter with argument |
//! | `{# comment #}` | Comment — stripped from output |
//! | `{% if cond %}` ... `{% endif %}` | Conditional block |
//! | `{% if cond %}` ... `{% else %}` ... `{% endif %}` | If/else |
//! | `{% if cond %}` ... `{% elif cond2 %}` ... `{% endif %}` | If/elif/else |
//! | `{% for x in list %}` ... `{% endfor %}` | Loop over a list |
//! | `{% for x in list %}` ... `{% else %}` ... `{% endfor %}` | Loop with empty fallback |
//! | `{{ loop.index }}` | 1-based loop index (inside for) |
//! | `{{ loop.index0 }}` | 0-based loop index (inside for) |
//! | `{{ loop.first }}` | `"true"` on first iteration |
//! | `{{ loop.last }}` | `"true"` on last iteration |
//! | `{{ loop.length }}` | Total number of items |
//! | `{% set x = value %}` | Set a variable in current scope |
//! | `{% raw %}` ... `{% endraw %}` | Output block literally, no processing |
//! | `{%- tag -%}` | Strip whitespace around a tag |
//!
//! # Filters
//!
//! `upper`, `lower`, `capitalize`, `trim`, `length`, `wordcount`,
//! `reverse`, `escape`, `safe`, `default("fallback")`,
//! `replace("a","b")`, `truncate(n)`, `join("-")`, `abs`, `round`
//!
//! Filters can be chained: `{{ name | upper | truncate(10) }}`
//!
//! # Conditions
//!
//! Conditions support `==`, `!=`, `>`, `<`, `>=`, `<=`, `and`, `or`, `not`.

use std::collections::HashMap;
use sunweb_core::response_types::{HtmlResponse, TextResponse};

// ── Value ────────────────────────────────────────────────────────────────────

/// A value that can be inserted into a [`Context`] and used in templates.
#[derive(Debug, Clone)]
pub enum Value {
    /// A string value, rendered (HTML-escaped by default) into the template.
    Str(String),
    /// A boolean value, used in `{% if %}` conditions.
    Bool(bool),
    /// A numeric value (stored as f64).
    Num(f64),
    /// A list of row maps, used in `{% for %}` loops.
    List(Vec<HashMap<String, Value>>),
}

impl Value {
    /// Truthiness following Jinja2 rules.
    fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Str(s) => !s.is_empty(),
            Value::Num(n) => *n != 0.0,
            Value::List(l) => !l.is_empty(),
        }
    }

    fn render_raw(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Num(n) => format_num(*n),
            Value::List(_) => "[list]".to_string(),
        }
    }
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
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Num(n)
    }
}
impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Num(n as f64)
    }
}
impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Num(n as f64)
    }
}
impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Num(n as f64)
    }
}

/// A map of variable names to [`Value`]s passed into a template.
pub type Context = HashMap<String, Value>;

// ── AST ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum Node {
    Text(String),
    Variable {
        key: String,
        filters: Vec<Filter>,
    },
    Comment,
    IfBlock {
        branches: Vec<(String, Vec<Node>)>,
    }, // ("" condition = else branch)
    ForBlock {
        var: String,
        iterable: String,
        body: Vec<Node>,
        else_body: Vec<Node>,
    },
    SetBlock {
        name: String,
        value: String,
    },
}

// ── Filters ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Filter {
    Upper,
    Lower,
    Capitalize,
    Trim,
    Length,
    Wordcount,
    Reverse,
    Escape,
    Safe,
    Abs,
    Round,
    Default(String),
    Replace(String, String),
    Truncate(usize),
    Join(String),
}

fn parse_filter(name: &str) -> Option<Filter> {
    let name = name.trim();
    if let Some(inner) = name
        .strip_prefix("default(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return Some(Filter::Default(strip_quotes(inner).to_string()));
    }
    if let Some(inner) = name
        .strip_prefix("replace(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts = split_args(inner);
        if parts.len() == 2 {
            return Some(Filter::Replace(
                strip_quotes(&parts[0]).to_string(),
                strip_quotes(&parts[1]).to_string(),
            ));
        }
    }
    if let Some(inner) = name
        .strip_prefix("truncate(")
        .and_then(|s| s.strip_suffix(')'))
    {
        if let Ok(n) = inner.trim().parse::<usize>() {
            return Some(Filter::Truncate(n));
        }
    }
    if let Some(inner) = name.strip_prefix("join(").and_then(|s| s.strip_suffix(')')) {
        return Some(Filter::Join(strip_quotes(inner).to_string()));
    }
    match name {
        "upper" => Some(Filter::Upper),
        "lower" => Some(Filter::Lower),
        "capitalize" => Some(Filter::Capitalize),
        "trim" => Some(Filter::Trim),
        "length" | "count" => Some(Filter::Length),
        "wordcount" => Some(Filter::Wordcount),
        "reverse" => Some(Filter::Reverse),
        "escape" | "e" => Some(Filter::Escape),
        "safe" => Some(Filter::Safe),
        "abs" => Some(Filter::Abs),
        "round" => Some(Filter::Round),
        _ => None,
    }
}

fn apply_filter(s: &str, filter: &Filter, already_safe: bool) -> (String, bool) {
    match filter {
        Filter::Upper => (s.to_uppercase(), already_safe),
        Filter::Lower => (s.to_lowercase(), already_safe),
        Filter::Trim => (s.trim().to_string(), already_safe),
        Filter::Reverse => (s.chars().rev().collect(), already_safe),
        Filter::Escape => (html_escape(s), true),
        Filter::Safe => (s.to_string(), true),
        Filter::Length => (s.chars().count().to_string(), true),
        Filter::Wordcount => (s.split_whitespace().count().to_string(), true),
        Filter::Abs => (format_num(s.parse::<f64>().unwrap_or(0.0).abs()), true),
        Filter::Round => (format_num(s.parse::<f64>().unwrap_or(0.0).round()), true),
        Filter::Capitalize => {
            let mut c = s.chars();
            let out = match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase(),
            };
            (out, already_safe)
        }
        Filter::Default(d) => {
            if s.is_empty() {
                (d.clone(), already_safe)
            } else {
                (s.to_string(), already_safe)
            }
        }
        Filter::Replace(from, to) => (s.replace(from.as_str(), to.as_str()), already_safe),
        Filter::Truncate(n) => {
            if s.chars().count() <= *n {
                (s.to_string(), already_safe)
            } else {
                let t: String = s.chars().take(n.saturating_sub(3)).collect();
                (format!("{}...", t), already_safe)
            }
        }
        Filter::Join(_) => {
            (s.to_string(), already_safe)
        }
    }
}

fn apply_filters(val: &Value, filters: &[Filter]) -> (String, bool) {
    // Special case: join on a List value
    if filters.iter().any(|f| matches!(f, Filter::Join(_))) {
        if let Value::List(items) = val {
            let sep = filters
                .iter()
                .find_map(|f| {
                    if let Filter::Join(s) = f {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("");
            let joined: Vec<String> = items
                .iter()
                .flat_map(|m| m.values().map(|v| v.render_raw()))
                .collect();
            let s = joined.join(sep);
            // Apply remaining filters after join
            let remaining: Vec<&Filter> = filters
                .iter()
                .skip_while(|f| !matches!(f, Filter::Join(_)))
                .skip(1)
                .collect();
            let mut current = s;
            let mut safe = false;
            for f in remaining {
                let (next, s2) = apply_filter(&current, f, safe);
                current = next;
                safe = s2;
            }
            return (current, safe);
        }
    }

    let mut current = val.render_raw();
    let mut safe = false;
    for f in filters {
        let (next, s) = apply_filter(&current, f, safe);
        current = next;
        safe = s;
    }
    (current, safe)
}

// ── Lexer tokens ──────────────────────────────────────────────────────────────

#[derive(Debug)]
enum Token {
    Text(String),
    Variable(String),   // {{ expr }}
    Comment,            // {# ... #}
    BlockStart(String), // {% tag %}  — not starting with "end"
    BlockEnd(String),   // {% endtag %}
}

fn lex(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut rest = input;
    let mut raw_mode = false;
    let mut raw_buf = String::new();

    while !rest.is_empty() {
        let var_pos = rest.find("{{").unwrap_or(usize::MAX);
        let block_pos = rest.find("{%").unwrap_or(usize::MAX);
        let comment_pos = rest.find("{#").unwrap_or(usize::MAX);
        let first = var_pos.min(block_pos).min(comment_pos);

        if first == usize::MAX {
            if raw_mode {
                raw_buf.push_str(rest);
            } else {
                push_text(&mut tokens, rest);
            }
            break;
        }

        let before = &rest[..first];

        if first == block_pos {
            let end = rest[first..].find("%}").expect("unclosed {%") + first;
            let inner = &rest[first + 2..end];
            let strip_left = inner.starts_with('-');
            let strip_right = inner.ends_with('-');
            let expr = inner.trim_matches('-').trim();

            if expr == "raw" {
                let text = if strip_left {
                    before.trim_end()
                } else {
                    before
                };
                if raw_mode {
                    raw_buf.push_str(text);
                } else {
                    push_text(&mut tokens, text);
                }
                raw_mode = true;
                rest = &rest[end + 2..];
                if strip_right {
                    rest = rest.trim_start();
                }
                continue;
            }
            if expr == "endraw" {
                let text = if strip_left {
                    before.trim_end()
                } else {
                    before
                };
                raw_buf.push_str(text);
                tokens.push(Token::Text(raw_buf.clone()));
                raw_buf.clear();
                raw_mode = false;
                rest = &rest[end + 2..];
                if strip_right {
                    rest = rest.trim_start();
                }
                continue;
            }

            if raw_mode {
                raw_buf.push_str(before);
                raw_buf.push_str(&rest[first..end + 2]);
                rest = &rest[end + 2..];
                continue;
            }

            let text = if strip_left {
                before.trim_end()
            } else {
                before
            };
            push_text(&mut tokens, text);

            if expr.starts_with("end") {
                tokens.push(Token::BlockEnd(expr.to_string()));
            } else {
                tokens.push(Token::BlockStart(expr.to_string()));
            }

            rest = &rest[end + 2..];
            if strip_right {
                rest = rest.trim_start();
            }
        } else if first == var_pos {
            if raw_mode {
                raw_buf.push_str(before);
                let end = rest[first..].find("}}").expect("unclosed {{") + first;
                raw_buf.push_str(&rest[first..end + 2]);
                rest = &rest[end + 2..];
                continue;
            }
            push_text(&mut tokens, before);
            let end = rest[first..].find("}}").expect("unclosed {{") + first;
            tokens.push(Token::Variable(rest[first + 2..end].trim().to_string()));
            rest = &rest[end + 2..];
        } else {
            // comment
            if raw_mode {
                raw_buf.push_str(before);
                let end = rest[first..].find("#}").expect("unclosed {#") + first;
                raw_buf.push_str(&rest[first..end + 2]);
                rest = &rest[end + 2..];
                continue;
            }
            push_text(&mut tokens, before);
            let end = rest[first..].find("#}").expect("unclosed {#") + first;
            tokens.push(Token::Comment);
            rest = &rest[end + 2..];
        }
    }

    tokens
}

fn push_text(tokens: &mut Vec<Token>, s: &str) {
    if !s.is_empty() {
        tokens.push(Token::Text(s.to_string()));
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

fn parse_variable_node(expr: &str) -> Node {
    let parts: Vec<&str> = expr.splitn(2, '|').collect();
    let key = parts[0].trim().to_string();
    let filters = if parts.len() > 1 {
        split_pipes(parts[1])
            .iter()
            .filter_map(|f| parse_filter(f))
            .collect()
    } else {
        vec![]
    };
    Node::Variable { key, filters }
}

fn split_pipes(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            '|' if depth == 0 => {
                parts.push(cur.trim().to_string());
                cur = String::new();
            }
            _ => {
                cur.push(c);
            }
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}

fn parse(tokens: &[Token]) -> Vec<Node> {
    parse_slice(tokens).0
}

fn parse_slice(tokens: &[Token]) -> (Vec<Node>, usize) {
    let mut nodes = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Text(t) => nodes.push(Node::Text(t.clone())),
            Token::Variable(v) => nodes.push(parse_variable_node(v)),
            Token::Comment => nodes.push(Node::Comment),

            Token::BlockStart(expr) => {
                if expr.starts_with("if ") {
                    let (node, consumed) = parse_if(&tokens[i..]);
                    nodes.push(node);
                    i += consumed - 1;
                } else if expr.starts_with("for ") {
                    let (node, consumed) = parse_for(&tokens[i..]);
                    nodes.push(node);
                    i += consumed - 1;
                } else if expr.starts_with("set ") {
                    let rest = expr["set ".len()..].trim();
                    if let Some(eq) = rest.find('=') {
                        let name = rest[..eq].trim().to_string();
                        let value = rest[eq + 1..].trim().to_string();
                        nodes.push(Node::SetBlock { name, value });
                    }
                }
                // Unknown block tags are silently skipped
            }

            Token::BlockEnd(_) => {
                // Signal to caller: stop here
                return (nodes, i);
            }
        }
        i += 1;
    }

    (nodes, i)
}

/// Parse `{% if %}` ... optional `{% elif %}` ... optional `{% else %}` ... `{% endif %}`.
/// Returns (node, tokens consumed including `endif`).
fn parse_if(tokens: &[Token]) -> (Node, usize) {
    let first_cond = match &tokens[0] {
        Token::BlockStart(e) => e["if ".len()..].trim().to_string(),
        _ => panic!("parse_if: not an if token"),
    };

    let mut branches: Vec<(String, Vec<Node>)> = Vec::new();
    let mut current_cond = first_cond;
    let mut body_start = 1usize;
    let mut i = 1usize;
    let mut depth = 0usize;

    loop {
        if i >= tokens.len() {
            panic!("unclosed {{% if %}}");
        }

        match &tokens[i] {
            Token::BlockStart(e) if e.starts_with("if ") || e.starts_with("for ") => depth += 1,

            Token::BlockEnd(e) if e.trim() == "endfor" || e.trim() == "endif" => {
                if depth > 0 {
                    depth -= 1;
                } else if e.trim() == "endif" {
                    branches.push((current_cond, parse(&tokens[body_start..i])));
                    i += 1;
                    break;
                }
            }

            Token::BlockStart(e) if depth == 0 && (e.starts_with("elif ") || e == "else") => {
                branches.push((current_cond.clone(), parse(&tokens[body_start..i])));
                current_cond = if e == "else" {
                    String::new()
                } else {
                    e["elif ".len()..].trim().to_string()
                };
                i += 1;
                body_start = i;
                continue;
            }

            _ => {}
        }
        i += 1;
    }

    (Node::IfBlock { branches }, i)
}

/// Parse `{% for %}` ... optional `{% else %}` ... `{% endfor %}`.
/// Returns (node, tokens consumed including `endfor`).
fn parse_for(tokens: &[Token]) -> (Node, usize) {
    let (var, iterable) = match &tokens[0] {
        Token::BlockStart(e) => {
            let rest = e["for ".len()..].trim();
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            (
                parts[0].to_string(),
                parts.get(2).unwrap_or(&"").to_string(),
            )
        }
        _ => panic!("parse_for: not a for token"),
    };

    let mut i = 1usize;
    let mut depth = 0usize;
    let mut else_pos: Option<usize> = None;
    let end_pos: Option<usize>;

    loop {
        if i >= tokens.len() {
            panic!("unclosed {{% for %}}");
        }

        match &tokens[i] {
            Token::BlockStart(e) if e.starts_with("if ") || e.starts_with("for ") => depth += 1,

            Token::BlockEnd(e) if e.trim() == "endfor" => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    end_pos = Some(i);
                    i += 1;
                    break;
                }
            }
            Token::BlockEnd(e) if e.trim() == "endif" => {
                if depth > 0 {
                    depth -= 1;
                }
            }

            Token::BlockStart(e) if depth == 0 && e == "else" => {
                else_pos = Some(i);
            }

            _ => {}
        }
        i += 1;
    }

    let end = end_pos.unwrap();
    let body_end = else_pos.unwrap_or(end);
    let body = parse(&tokens[1..body_end]);
    let else_body = else_pos
        .map(|ep| parse(&tokens[ep + 1..end]))
        .unwrap_or_default();

    (
        Node::ForBlock {
            var,
            iterable,
            body,
            else_body,
        },
        i,
    )
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Renders a template string with the given context and returns an [`HtmlResponse`].
pub fn render_response(template: &str, ctx: &Context) -> HtmlResponse {
    HtmlResponse::ok(render(template, ctx))
}

/// Renders a template string with the given context and returns the output as a [`String`].
///
/// # Panics
/// Panics on unclosed or mismatched template tags.
pub fn render(template: &str, ctx: &Context) -> String {
    let tokens = lex(template);
    let nodes = parse(&tokens);
    render_nodes(&nodes, ctx)
}

fn render_nodes(nodes: &[Node], ctx: &Context) -> String {
    let mut out = String::new();
    let mut local: Context = ctx.clone();

    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Comment => {}

            Node::Variable { key, filters } => {
                let val = resolve_value(key, &local);
                let (s, safe) = apply_filters(&val, filters);
                if safe {
                    out.push_str(&s);
                } else {
                    out.push_str(&html_escape(&s));
                }
            }

            Node::SetBlock { name, value } => {
                let v = parse_literal_or_lookup(value, &local);
                local.insert(name.clone(), v);
            }

            Node::IfBlock { branches } => {
                for (cond, body) in branches {
                    let truthy = if cond.is_empty() {
                        true
                    } else {
                        eval_condition(cond, &local)
                    };
                    if truthy {
                        out.push_str(&render_nodes(body, &local));
                        break;
                    }
                }
            }

            Node::ForBlock {
                var,
                iterable,
                body,
                else_body,
            } => match local.get(iterable.as_str()) {
                Some(Value::List(items)) if !items.is_empty() => {
                    let len = items.len();
                    let items = items.clone();
                    for (idx, item) in items.iter().enumerate() {
                        let mut inner = local.clone();
                        inner.insert("loop.index".to_string(), Value::Num((idx + 1) as f64));
                        inner.insert("loop.index0".to_string(), Value::Num(idx as f64));
                        inner.insert("loop.first".to_string(), Value::Bool(idx == 0));
                        inner.insert("loop.last".to_string(), Value::Bool(idx == len - 1));
                        inner.insert("loop.length".to_string(), Value::Num(len as f64));
                        for (k, v) in item {
                            inner.insert(format!("{}.{}", var, k), v.clone());
                        }
                        out.push_str(&render_nodes(body, &inner));
                    }
                }
                _ => {
                    if !else_body.is_empty() {
                        out.push_str(&render_nodes(else_body, &local));
                    }
                }
            },
        }
    }

    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn resolve_value(key: &str, ctx: &Context) -> Value {
    ctx.get(key).cloned().unwrap_or(Value::Str(String::new()))
}

fn parse_literal_or_lookup(s: &str, ctx: &Context) -> Value {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Value::Str(s[1..s.len() - 1].to_string())
    } else if let Ok(n) = s.parse::<f64>() {
        Value::Num(n)
    } else if s == "true" || s == "True" {
        Value::Bool(true)
    } else if s == "false" || s == "False" {
        Value::Bool(false)
    } else {
        ctx.get(s).cloned().unwrap_or(Value::Str(String::new()))
    }
}

/// Evaluate a boolean condition expression.
/// Supports: `and`, `or`, `not`, `==`, `!=`, `>`, `<`, `>=`, `<=`, bare variable.
fn eval_condition(cond: &str, ctx: &Context) -> bool {
    let cond = cond.trim();

    if let Some(idx) = find_kw(cond, " and ") {
        return eval_condition(&cond[..idx], ctx) && eval_condition(&cond[idx + 5..], ctx);
    }
    if let Some(idx) = find_kw(cond, " or ") {
        return eval_condition(&cond[..idx], ctx) || eval_condition(&cond[idx + 4..], ctx);
    }
    if let Some(rest) = cond.strip_prefix("not ") {
        return !eval_condition(rest.trim(), ctx);
    }

    for op in &["==", "!=", ">=", "<=", ">", "<"] {
        if let Some(idx) = cond.find(op) {
            let lhs = resolve_token(cond[..idx].trim(), ctx);
            let rhs = resolve_token(cond[idx + op.len()..].trim(), ctx);
            return match *op {
                "==" => lhs == rhs,
                "!=" => lhs != rhs,
                ">=" => num_cmp(&lhs, &rhs) >= 0,
                "<=" => num_cmp(&lhs, &rhs) <= 0,
                ">" => num_cmp(&lhs, &rhs) > 0,
                "<" => num_cmp(&lhs, &rhs) < 0,
                _ => false,
            };
        }
    }

    resolve_value(cond, ctx).is_truthy()
}

fn find_kw(s: &str, kw: &str) -> Option<usize> {
    s.find(kw)
}

fn resolve_token(token: &str, ctx: &Context) -> String {
    let t = token.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t[1..t.len() - 1].to_string()
    } else {
        resolve_value(t, ctx).render_raw()
    }
}

fn num_cmp(a: &str, b: &str) -> i32 {
    let an: f64 = a.parse().unwrap_or(f64::NAN);
    let bn: f64 = b.parse().unwrap_or(f64::NAN);
    if an < bn {
        -1
    } else if an > bn {
        1
    } else {
        0
    }
}

fn format_num(n: f64) -> String {
    if n == n.floor() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn split_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut qc = '"';
    for c in s.chars() {
        if in_q {
            if c == qc {
                in_q = false;
            }
            cur.push(c);
        } else if c == '"' || c == '\'' {
            in_q = true;
            qc = c;
            cur.push(c);
        } else if c == ',' {
            args.push(cur.trim().to_string());
            cur = String::new();
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        args.push(cur.trim().to_string());
    }
    args
}
