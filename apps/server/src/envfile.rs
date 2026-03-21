use std::collections::HashMap;

pub fn parse_env_file(path: &str) -> Result<HashMap<String, String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read env file {}: {}", path, e))?;

    let mut vars = HashMap::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key_raw, value_raw)) = line.split_once('=') else {
            return Err(format!(
                "Invalid env file format at line {}: expected KEY=VALUE",
                idx + 1
            ));
        };

        let key = key_raw.trim();
        if key.is_empty() {
            return Err(format!(
                "Invalid env file format at line {}: empty key",
                idx + 1
            ));
        }

        let value = value_raw.trim();
        let value = unquote(value);
        vars.insert(key.to_string(), value);
    }

    Ok(vars)
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[value.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}
