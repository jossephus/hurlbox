use serde_json::Value as JsonValue;

#[derive(Debug, Clone)]
pub struct ResponseSnapshot {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub include_content_type_assert: bool,
    pub max_json_field_asserts: usize,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            include_content_type_assert: true,
            max_json_field_asserts: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestedAssert {
    HeaderContains {
        header: String,
        value: String,
    },
    JsonPath {
        expression: String,
        predicate: SuggestedPredicate,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestedPredicate {
    Exists,
    IsInteger,
    IsFloat,
    IsBoolean,
    IsString,
    IsCollection,
    IsEmpty,
}

#[derive(Debug, Clone)]
pub struct SuggestedResponseSpec {
    pub status: u16,
    pub asserts: Vec<SuggestedAssert>,
}

pub fn build_response_spec(
    snapshot: &ResponseSnapshot,
    options: &BuildOptions,
) -> SuggestedResponseSpec {
    let mut asserts = Vec::new();

    if options.include_content_type_assert {
        if let Some((_, value)) = snapshot
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        {
            let mime = value.split(';').next().map(str::trim).unwrap_or("");
            if !mime.is_empty() {
                asserts.push(SuggestedAssert::HeaderContains {
                    header: "Content-Type".to_string(),
                    value: mime.to_string(),
                });
            }
        }
    }

    if options.max_json_field_asserts > 0 {
        if let Ok(value) = serde_json::from_str::<JsonValue>(&snapshot.body) {
            asserts.push(SuggestedAssert::JsonPath {
                expression: "$".to_string(),
                predicate: SuggestedPredicate::IsCollection,
            });

            let mut remaining = options.max_json_field_asserts;
            match value {
                JsonValue::Object(map) => {
                    for (key, field_value) in map {
                        if remaining == 0 {
                            break;
                        }
                        if let Some(path) = dot_path("$", &key) {
                            if let Some(predicate) = predicate_for_value(&field_value) {
                                asserts.push(SuggestedAssert::JsonPath {
                                    expression: path,
                                    predicate,
                                });
                                remaining -= 1;
                            }
                        }
                    }
                }
                JsonValue::Array(items) => {
                    if let Some(first) = items.first() {
                        if let Some(predicate) = predicate_for_value(first) {
                            asserts.push(SuggestedAssert::JsonPath {
                                expression: "$[0]".to_string(),
                                predicate,
                            });
                        }

                        if let JsonValue::Object(map) = first {
                            for (key, field_value) in map {
                                if remaining == 0 {
                                    break;
                                }
                                if let Some(path) = dot_path("$[0]", key) {
                                    if let Some(predicate) = predicate_for_value(field_value) {
                                        asserts.push(SuggestedAssert::JsonPath {
                                            expression: path,
                                            predicate,
                                        });
                                        remaining -= 1;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    SuggestedResponseSpec {
        status: snapshot.status,
        asserts,
    }
}

fn is_simple_json_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn dot_path(base: &str, key: &str) -> Option<String> {
    if is_simple_json_key(key) {
        Some(format!("{}.{}", base, key))
    } else {
        None
    }
}

fn predicate_for_value(value: &JsonValue) -> Option<SuggestedPredicate> {
    match value {
        JsonValue::Null => None,
        JsonValue::Bool(_) => Some(SuggestedPredicate::IsBoolean),
        JsonValue::Number(n) => {
            if n.is_i64() || n.is_u64() {
                Some(SuggestedPredicate::IsInteger)
            } else {
                Some(SuggestedPredicate::IsFloat)
            }
        }
        JsonValue::String(_) => Some(SuggestedPredicate::IsString),
        JsonValue::Array(items) => {
            if items.is_empty() {
                Some(SuggestedPredicate::IsEmpty)
            } else {
                Some(SuggestedPredicate::IsCollection)
            }
        }
        JsonValue::Object(map) => {
            if map.is_empty() {
                Some(SuggestedPredicate::IsEmpty)
            } else {
                Some(SuggestedPredicate::IsCollection)
            }
        }
    }
}
