use std::collections::HashMap;
use sunweb_core::Response;

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

#[derive(Debug)]
enum Token<'a> {
    Text(&'a str),
    Variable(&'a str),   // {{ name }}
    BlockStart(&'a str), // {% if condition %}
    BlockEnd(&'a str),   // {% endif %}
}

#[derive(Debug, Clone)]
pub enum Value {
    Str(String),
    Bool(bool),
    List(Vec<HashMap<String, Value>>),
}

pub type Context = HashMap<String, Value>;

pub fn render_response(template: &str, ctx: &Context) -> Response {
    let body = render(template, ctx);
    let mut response = Response::ok();
    response.set_body(Vec::from(body));
    response.set_html();
    response
}

pub fn render(template: &str, ctx: &Context) -> String {
    let tokens = lex(template);
    let nodes = parse(&tokens);
    render_nodes(&nodes, ctx)
}

pub fn render_nodes(nodes: &[Node], ctx: &Context) -> String {
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
                        // Merge loop variable into context
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

fn lex(input: &str) -> Vec<Token> {
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
                if expr.starts_with("if ") {
                    let condition = expr[3..].trim().to_string();
                    // collect body tokens until endif
                    let (body_tokens, consumed) = collect_until(&tokens[i + 1..], "endif");
                    nodes.push(Node::IfBlock {
                        condition,
                        body: parse(body_tokens),
                    });
                    i += consumed;
                } else if expr.starts_with("for ") {
                    // "for item in items"
                    let parts: Vec<&str> = expr[4..].splitn(3, ' ').collect();
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
            // Track nesting — if there's an inner if/for, we need to skip its end tag
            Token::BlockStart(expr) => {
                if expr.starts_with("if ") || expr.starts_with("for ") {
                    depth += 1;
                }
            }
            Token::BlockEnd(expr) => {
                if expr.trim() == end_tag {
                    if depth == 0 {
                        // Found our matching end tag
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
