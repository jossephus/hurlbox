use super::model::*;

#[derive(Debug, Clone, PartialEq)]
enum Section {
    Query,
    Form,
    Cookies,
    Options,
    Captures,
    Asserts,
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Start,
    RequestHeaders,
    RequestSection(Section),
    RequestBody,
    ResponseHeaders,
    ResponseSection(Section),
}

pub fn parse(input: &str) -> HurlFile {
    let lines: Vec<&str> = input.lines().collect();
    let mut entries: Vec<Entry> = Vec::new();
    let mut state = State::Start;

    let mut current_comment: Option<String> = None;
    let mut current_request = Request::default();
    let mut current_response: Option<ResponseSpec> = None;
    let mut body_lines: Vec<String> = Vec::new();
    let mut has_request = false;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines contextually
        if trimmed.is_empty() {
            match state {
                State::RequestBody => {
                    body_lines.push(String::new());
                }
                _ => {}
            }
            i += 1;
            continue;
        }

        // Handle comments
        if trimmed.starts_with('#') {
            let comment_text = trimmed.trim_start_matches('#').trim().to_string();
            match current_comment {
                Some(ref mut c) => {
                    c.push('\n');
                    c.push_str(&comment_text);
                }
                None => {
                    current_comment = Some(comment_text);
                }
            }
            i += 1;
            continue;
        }

        // Check if this line is a new request method line
        if is_request_line(trimmed) {
            // Finalize previous entry if we have one
            if has_request {
                finalize_body(&mut current_request, &mut body_lines);
                entries.push(Entry {
                    comment: current_comment.take(),
                    request: current_request,
                    response_spec: current_response.take(),
                });
                current_request = Request::default();
                body_lines.clear();
            }

            let (method, url) = parse_request_line(trimmed);
            current_request.method = method;
            current_request.url = url;
            has_request = true;
            state = State::RequestHeaders;
            i += 1;
            continue;
        }

        // Check if this line is a response line
        if is_response_line(trimmed) {
            finalize_body(&mut current_request, &mut body_lines);
            let status = parse_response_line(trimmed);
            current_response = Some(ResponseSpec {
                status,
                headers: Vec::new(),
                captures: Vec::new(),
                asserts: Vec::new(),
            });
            state = State::ResponseHeaders;
            i += 1;
            continue;
        }

        // Check if this line is a section header
        if let Some(section) = parse_section_header(trimmed) {
            finalize_body(&mut current_request, &mut body_lines);
            match section {
                Section::Captures | Section::Asserts => {
                    if current_response.is_none() {
                        current_response = Some(ResponseSpec {
                            status: None,
                            headers: Vec::new(),
                            captures: Vec::new(),
                            asserts: Vec::new(),
                        });
                    }
                    state = State::ResponseSection(section);
                }
                _ => {
                    state = State::RequestSection(section);
                }
            }
            i += 1;
            continue;
        }

        // Process line based on current state
        match state {
            State::Start => {
                // Unexpected line, skip
            }
            State::RequestHeaders => {
                if let Some(kv) = parse_key_value(trimmed) {
                    current_request.headers.push(kv);
                } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    state = State::RequestBody;
                    body_lines.push(line.to_string());
                } else {
                    state = State::RequestBody;
                    body_lines.push(line.to_string());
                }
            }
            State::RequestSection(ref section) => {
                if let Some(kv) = parse_key_value(trimmed) {
                    match section {
                        Section::Query => current_request.query_params.push(kv),
                        Section::Form => current_request.form_params.push(kv),
                        Section::Cookies => current_request.cookies.push(kv),
                        Section::Options => current_request.options.push(kv),
                        _ => {}
                    }
                }
            }
            State::RequestBody => {
                body_lines.push(line.to_string());
            }
            State::ResponseHeaders => {
                if let Some(kv) = parse_key_value(trimmed) {
                    if let Some(ref mut resp) = current_response {
                        resp.headers.push(kv);
                    }
                }
            }
            State::ResponseSection(ref section) => match section {
                Section::Captures => {
                    if let Some(capture) = parse_capture(trimmed) {
                        if let Some(ref mut resp) = current_response {
                            resp.captures.push(capture);
                        }
                    }
                }
                Section::Asserts => {
                    if let Some(assert) = parse_assert(trimmed) {
                        if let Some(ref mut resp) = current_response {
                            resp.asserts.push(assert);
                        }
                    }
                }
                _ => {
                    // Other sections in response context treated as key-value headers
                    if let Some(kv) = parse_key_value(trimmed) {
                        if let Some(ref mut resp) = current_response {
                            resp.headers.push(kv);
                        }
                    }
                }
            },
        }

        i += 1;
    }

    // Finalize last entry
    if has_request {
        finalize_body(&mut current_request, &mut body_lines);
        entries.push(Entry {
            comment: current_comment.take(),
            request: current_request,
            response_spec: current_response.take(),
        });
    }

    HurlFile {
        path: None,
        entries,
    }
}

fn is_request_line(line: &str) -> bool {
    let first_word = line.split_whitespace().next().unwrap_or("");
    Method::from_str(first_word).is_some()
}

fn parse_request_line(line: &str) -> (Method, String) {
    let mut parts = line.splitn(2, char::is_whitespace);
    let method_str = parts.next().unwrap_or("GET");
    let url = parts.next().unwrap_or("").trim().to_string();
    let method = Method::from_str(method_str).unwrap_or(Method::Get);
    (method, url)
}

fn is_response_line(line: &str) -> bool {
    line.starts_with("HTTP/") || line.starts_with("HTTP ")
}

fn parse_response_line(line: &str) -> Option<u16> {
    // Formats: "HTTP 200", "HTTP/1.1 404", "HTTP *"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<u16>().ok()
    } else {
        None
    }
}

fn parse_section_header(line: &str) -> Option<Section> {
    match line {
        "[Query]" | "[QueryStringParams]" => Some(Section::Query),
        "[Form]" | "[FormParams]" => Some(Section::Form),
        "[Cookies]" => Some(Section::Cookies),
        "[Options]" => Some(Section::Options),
        "[Captures]" => Some(Section::Captures),
        "[Asserts]" => Some(Section::Asserts),
        _ => None,
    }
}

fn parse_key_value(line: &str) -> Option<KeyValue> {
    let colon_pos = line.find(':')?;
    let key = line[..colon_pos].trim();
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    let value = line[colon_pos + 1..].trim();
    Some(KeyValue::new(key, value))
}

fn finalize_body(request: &mut Request, body_lines: &mut Vec<String>) {
    if body_lines.is_empty() {
        return;
    }

    // Trim trailing empty lines
    while body_lines.last().map_or(false, |l| l.trim().is_empty()) {
        body_lines.pop();
    }

    if body_lines.is_empty() {
        return;
    }

    let text = body_lines.join("\n");
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return;
    }

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        request.body = Some(Body::Json(trimmed.to_string()));
    } else if trimmed.starts_with('<') {
        request.body = Some(Body::Xml(trimmed.to_string()));
    } else {
        request.body = Some(Body::Text(trimmed.to_string()));
    }

    body_lines.clear();
}

fn parse_capture(line: &str) -> Option<Capture> {
    // Format: name: query_type "expression"
    // or:     name: query_type (for body, status, duration, etc.)
    let colon_pos = line.find(':')?;
    let name = line[..colon_pos].trim().to_string();
    let rest = line[colon_pos + 1..].trim();

    let query = parse_query(rest)?;
    Some(Capture { name, query })
}

fn parse_query(input: &str) -> Option<Query> {
    let input = input.trim();

    // Simple keyword queries
    match input {
        "status" => return Some(Query::Status),
        "url" => return Some(Query::Url),
        "body" => return Some(Query::Body),
        "duration" => return Some(Query::Duration),
        "bytes" => return Some(Query::Bytes),
        "sha256" => return Some(Query::Sha256),
        "md5" => return Some(Query::Md5),
        "ip" => return Some(Query::Ip),
        _ => {}
    }

    // Queries with a string argument: jsonpath "expr", header "expr", etc.
    if let Some((keyword, rest)) = split_first_word(input) {
        let arg = extract_quoted_string(rest.trim())?;
        match keyword {
            "jsonpath" => return Some(Query::JsonPath(arg)),
            "header" => return Some(Query::Header(arg)),
            "xpath" => return Some(Query::XPath(arg)),
            "regex" => return Some(Query::Regex(arg)),
            "cookie" => return Some(Query::Cookie(arg)),
            "variable" => return Some(Query::Variable(arg)),
            "certificate" => return Some(Query::Certificate(arg)),
            _ => {}
        }
    }

    None
}

fn parse_assert(line: &str) -> Option<Assert> {
    let line = line.trim();

    // First, parse the query part, then the predicate
    // The query can be:
    //   "status" / "body" / "url" / "duration" / "bytes" / "sha256" / "md5" / "ip"
    //   "header \"Name\"" / "jsonpath \"$.path\"" / "xpath \"expr\"" / etc.

    let (query, rest) = parse_query_from_assert(line)?;
    let predicate = parse_predicate(rest.trim())?;

    Some(Assert { query, predicate })
}

fn parse_query_from_assert(line: &str) -> Option<(Query, &str)> {
    // Try simple keyword queries first
    let simple_keywords = [
        "status", "url", "body", "duration", "bytes", "sha256", "md5", "ip",
    ];

    for keyword in &simple_keywords {
        if line.starts_with(keyword) {
            let rest = &line[keyword.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                let query = match *keyword {
                    "status" => Query::Status,
                    "url" => Query::Url,
                    "body" => Query::Body,
                    "duration" => Query::Duration,
                    "bytes" => Query::Bytes,
                    "sha256" => Query::Sha256,
                    "md5" => Query::Md5,
                    "ip" => Query::Ip,
                    _ => unreachable!(),
                };
                return Some((query, rest.trim()));
            }
        }
    }

    // Try queries with quoted string arguments
    let parameterized = [
        "jsonpath",
        "header",
        "xpath",
        "regex",
        "cookie",
        "variable",
        "certificate",
    ];

    for keyword in &parameterized {
        if line.starts_with(keyword) {
            let after_keyword = &line[keyword.len()..];
            if after_keyword.starts_with(char::is_whitespace) {
                let after_keyword = after_keyword.trim_start();
                if after_keyword.starts_with('"') {
                    if let Some(end_quote) = find_closing_quote(after_keyword) {
                        let arg = after_keyword[1..end_quote].to_string();
                        let rest = &after_keyword[end_quote + 1..];
                        let query = match *keyword {
                            "jsonpath" => Query::JsonPath(arg),
                            "header" => Query::Header(arg),
                            "xpath" => Query::XPath(arg),
                            "regex" => Query::Regex(arg),
                            "cookie" => Query::Cookie(arg),
                            "variable" => Query::Variable(arg),
                            "certificate" => Query::Certificate(arg),
                            _ => unreachable!(),
                        };
                        return Some((query, rest.trim()));
                    }
                }
            }
        }
    }

    None
}

fn parse_predicate(input: &str) -> Option<Predicate> {
    let input = input.trim();

    if input.is_empty() {
        return None;
    }

    // Unary predicates (no value)
    match input {
        "exists" => return Some(Predicate::Exists),
        "not exists" => return Some(Predicate::NotExists),
        "isInteger" => return Some(Predicate::IsInteger),
        "isFloat" => return Some(Predicate::IsFloat),
        "isBoolean" => return Some(Predicate::IsBoolean),
        "isString" => return Some(Predicate::IsString),
        "isCollection" => return Some(Predicate::IsCollection),
        "isEmpty" => return Some(Predicate::IsEmpty),
        _ => {}
    }

    // Operator predicates: == value, != value, > value, etc.
    let operators: &[(&str, fn(Value) -> Predicate)] = &[
        ("==", Predicate::Equal),
        ("!=", Predicate::NotEqual),
        (">=", Predicate::GreaterThanOrEqual),
        ("<=", Predicate::LessThanOrEqual),
        (">", Predicate::GreaterThan),
        ("<", Predicate::LessThan),
    ];

    for (op, constructor) in operators {
        if input.starts_with(op) {
            let val_str = input[op.len()..].trim();
            if let Some(value) = parse_value(val_str) {
                return Some(constructor(value));
            }
        }
    }

    // Keyword predicates with a value
    let keyword_predicates: &[(&str, fn(Value) -> Predicate)] = &[
        ("contains", Predicate::Contains),
        ("startsWith", Predicate::StartsWith),
        ("endsWith", Predicate::EndsWith),
        ("includes", Predicate::Includes),
    ];

    for (keyword, constructor) in keyword_predicates {
        if input.starts_with(keyword) {
            let rest = &input[keyword.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                let val_str = rest.trim();
                if let Some(value) = parse_value(val_str) {
                    return Some(constructor(value));
                }
            }
        }
    }

    // "matches" takes a string pattern
    if input.starts_with("matches") {
        let rest = &input["matches".len()..];
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            let val_str = rest.trim();
            if let Some(s) = extract_quoted_string(val_str) {
                return Some(Predicate::Matches(s));
            }
        }
    }

    // "not exists" could also appear with different spacing
    if input.starts_with("not ") {
        let rest = input["not ".len()..].trim();
        if rest == "exists" {
            return Some(Predicate::NotExists);
        }
    }

    None
}

fn parse_value(input: &str) -> Option<Value> {
    let input = input.trim();

    if input.is_empty() {
        return None;
    }

    // Null
    if input == "null" {
        return Some(Value::Null);
    }

    // Booleans
    if input == "true" {
        return Some(Value::Bool(true));
    }
    if input == "false" {
        return Some(Value::Bool(false));
    }

    // Quoted string
    if input.starts_with('"') {
        if let Some(s) = extract_quoted_string(input) {
            return Some(Value::String(s));
        }
    }

    // Try integer first, then float
    if let Ok(i) = input.parse::<i64>() {
        return Some(Value::Integer(i));
    }

    if let Ok(f) = input.parse::<f64>() {
        return Some(Value::Float(f));
    }

    // Unquoted string fallback
    Some(Value::String(input.to_string()))
}

fn extract_quoted_string(input: &str) -> Option<String> {
    if !input.starts_with('"') {
        return None;
    }
    let end = find_closing_quote(input)?;
    Some(input[1..end].to_string())
}

fn find_closing_quote(input: &str) -> Option<usize> {
    // input starts with '"', find the matching closing quote
    let bytes = input.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2; // skip escaped character
            continue;
        }
        if bytes[i] == b'"' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn split_first_word(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let space_pos = input.find(char::is_whitespace)?;
    let word = &input[..space_pos];
    let rest = &input[space_pos..];
    Some((word, rest))
}
