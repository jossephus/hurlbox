use std::fmt;
use std::path::PathBuf;

/// HTTP methods supported by Hurl
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
            Method::Patch => write!(f, "PATCH"),
            Method::Options => write!(f, "OPTIONS"),
            Method::Head => write!(f, "HEAD"),
        }
    }
}

impl Method {
    pub fn from_str(s: &str) -> Option<Method> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Method::Get),
            "POST" => Some(Method::Post),
            "PUT" => Some(Method::Put),
            "DELETE" => Some(Method::Delete),
            "PATCH" => Some(Method::Patch),
            "OPTIONS" => Some(Method::Options),
            "HEAD" => Some(Method::Head),
            _ => None,
        }
    }

    pub fn all() -> &'static [Method] {
        &[
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Delete,
            Method::Patch,
            Method::Options,
            Method::Head,
        ]
    }
}

/// A key-value pair used for headers, query params, form data, cookies
#[derive(Debug, Clone, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

impl KeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

/// Body content types
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Text(String),
    Json(String),
    Xml(String),
    #[allow(dead_code)]
    File(String),
    #[allow(dead_code)]
    Base64(String),
}

/// A complete Hurl file with one or more entries
#[derive(Debug, Clone, PartialEq)]
pub struct HurlFile {
    pub path: Option<PathBuf>,
    pub entries: Vec<Entry>,
}

impl Default for HurlFile {
    fn default() -> Self {
        Self {
            path: None,
            entries: vec![Entry::default()],
        }
    }
}

/// A single request-response pair in a Hurl file
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub comment: Option<String>,
    pub request: Request,
    pub response_spec: Option<ResponseSpec>,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            comment: None,
            request: Request::default(),
            response_spec: None,
        }
    }
}

/// The request specification
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub query_params: Vec<KeyValue>,
    pub form_params: Vec<KeyValue>,
    pub cookies: Vec<KeyValue>,
    pub body: Option<Body>,
    pub options: Vec<KeyValue>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: Method::Get,
            url: String::new(),
            headers: Vec::new(),
            query_params: Vec::new(),
            form_params: Vec::new(),
            cookies: Vec::new(),
            body: None,
            options: Vec::new(),
        }
    }
}

/// Expected response specification (assertions)
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseSpec {
    pub status: Option<u16>,
    pub headers: Vec<KeyValue>,
    pub captures: Vec<Capture>,
    pub asserts: Vec<Assert>,
}

/// A capture definition: extract a value from the response
#[derive(Debug, Clone, PartialEq)]
pub struct Capture {
    pub name: String,
    pub query: Query,
}

/// An assertion: query + predicate
#[derive(Debug, Clone, PartialEq)]
pub struct Assert {
    pub query: Query,
    pub predicate: Predicate,
}

/// Query types for captures and assertions
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Status,
    Url,
    Header(String),
    Cookie(String),
    Body,
    XPath(String),
    JsonPath(String),
    Regex(String),
    Variable(String),
    Duration,
    Bytes,
    Sha256,
    Md5,
    Certificate(String),
    Ip,
}

/// Predicate types for assertions
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Equal(Value),
    NotEqual(Value),
    GreaterThan(Value),
    GreaterThanOrEqual(Value),
    LessThan(Value),
    LessThanOrEqual(Value),
    Contains(Value),
    StartsWith(Value),
    EndsWith(Value),
    Matches(String),
    Exists,
    NotExists,
    Includes(Value),
    IsInteger,
    IsFloat,
    IsBoolean,
    IsString,
    IsCollection,
    IsEmpty,
}

/// Values used in predicates
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
        }
    }
}

/// The actual response from running hurl (parsed from --json output)
#[derive(Debug, Clone, Default)]
pub struct ActualResponse {
    pub status: u16,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub time_ms: f64,
    pub size_bytes: u64,
}
