use super::model::*;

pub fn serialize(file: &HurlFile) -> String {
    let mut out = String::new();
    for (i, entry) in file.entries.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        serialize_entry(entry, &mut out);
    }
    out
}

fn serialize_entry(entry: &Entry, out: &mut String) {
    if let Some(comment) = &entry.comment {
        out.push_str(&format!("# {}\n", comment));
    }
    serialize_request(&entry.request, out);
    if let Some(response) = &entry.response_spec {
        serialize_response(response, out);
    }
}

fn serialize_request(req: &Request, out: &mut String) {
    out.push_str(&format!("{} {}\n", req.method, req.url));

    for h in &req.headers {
        serialize_kv(h, out);
    }

    serialize_section("Query", &req.query_params, out);
    serialize_section("Form", &req.form_params, out);
    serialize_section("Cookies", &req.cookies, out);
    serialize_section("Options", &req.options, out);

    if let Some(body) = &req.body {
        serialize_body(body, out);
    }
}

fn serialize_section(name: &str, items: &[KeyValue], out: &mut String) {
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("[{}]\n", name));
    for kv in items {
        serialize_kv(kv, out);
    }
}

fn serialize_kv(kv: &KeyValue, out: &mut String) {
    if kv.enabled {
        out.push_str(&format!("{}: {}\n", kv.key, kv.value));
    } else {
        out.push_str(&format!("# {}: {}\n", kv.key, kv.value));
    }
}

fn serialize_body(body: &Body, out: &mut String) {
    match body {
        Body::Json(s) => out.push_str(&format!("{}\n", s)),
        Body::Xml(s) => out.push_str(&format!("{}\n", s)),
        Body::Text(s) => out.push_str(&format!("`{}`\n", s)),
        Body::File(path) => out.push_str(&format!("file,{};\n", path)),
        Body::Base64(content) => out.push_str(&format!("base64,{};\n", content)),
    }
}

fn serialize_response(resp: &ResponseSpec, out: &mut String) {
    if let Some(status) = resp.status {
        out.push_str(&format!("HTTP {}\n", status));
    }

    for h in &resp.headers {
        serialize_kv(h, out);
    }

    if !resp.captures.is_empty() {
        out.push_str("[Captures]\n");
        for cap in &resp.captures {
            out.push_str(&format!("{}: {}\n", cap.name, serialize_query(&cap.query)));
        }
    }

    if !resp.asserts.is_empty() {
        out.push_str("[Asserts]\n");
        for assert in &resp.asserts {
            out.push_str(&format!(
                "{} {}\n",
                serialize_query(&assert.query),
                serialize_predicate(&assert.predicate)
            ));
        }
    }
}

fn serialize_query(query: &Query) -> String {
    match query {
        Query::Status => "status".to_string(),
        Query::Header(name) => format!("header \"{}\"", name),
        Query::Cookie(name) => format!("cookie \"{}\"", name),
        Query::Body => "body".to_string(),
        Query::JsonPath(expr) => format!("jsonpath \"{}\"", expr),
        Query::XPath(expr) => format!("xpath \"{}\"", expr),
        Query::Regex(expr) => format!("regex \"{}\"", expr),
        Query::Variable(name) => format!("variable \"{}\"", name),
        Query::Duration => "duration".to_string(),
        Query::Url => "url".to_string(),
        Query::Bytes => "bytes".to_string(),
        Query::Sha256 => "sha256".to_string(),
        Query::Md5 => "md5".to_string(),
        Query::Certificate(attr) => format!("certificate \"{}\"", attr),
        Query::Ip => "ip".to_string(),
    }
}

fn serialize_predicate(pred: &Predicate) -> String {
    match pred {
        Predicate::Equal(v) => format!("== {}", v),
        Predicate::NotEqual(v) => format!("!= {}", v),
        Predicate::GreaterThan(v) => format!("> {}", v),
        Predicate::GreaterThanOrEqual(v) => format!(">= {}", v),
        Predicate::LessThan(v) => format!("< {}", v),
        Predicate::LessThanOrEqual(v) => format!("<= {}", v),
        Predicate::Contains(v) => format!("contains {}", v),
        Predicate::StartsWith(v) => format!("startsWith {}", v),
        Predicate::EndsWith(v) => format!("endsWith {}", v),
        Predicate::Matches(s) => format!("matches \"{}\"", s),
        Predicate::Exists => "exists".to_string(),
        Predicate::NotExists => "not exists".to_string(),
        Predicate::Includes(v) => format!("includes {}", v),
        Predicate::IsInteger => "isInteger".to_string(),
        Predicate::IsFloat => "isFloat".to_string(),
        Predicate::IsBoolean => "isBoolean".to_string(),
        Predicate::IsString => "isString".to_string(),
        Predicate::IsCollection => "isCollection".to_string(),
        Predicate::IsEmpty => "isEmpty".to_string(),
    }
}
