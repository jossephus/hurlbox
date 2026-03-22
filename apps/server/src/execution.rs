use crate::models::*;
use hurl::runner::{self, AssertResult, RunnerOptionsBuilder, Value, VariableSet};
use hurl::util::logger::LoggerOptionsBuilder;
use hurl_core::error::{DisplaySourceError, OutputFormat};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

pub struct ExecutionHandle {
    // Placeholder for managing running executions
}

#[derive(Clone)]
enum LastExecutionCommand {
    RunEntry {
        content: String,
        entry_index: usize,
        env: Option<HashMap<String, String>>,
    },
    RunToEnd {
        content: String,
        entry_index: usize,
        env: Option<HashMap<String, String>>,
    },
    RunFromBegin {
        content: String,
        entry_index: usize,
        env: Option<HashMap<String, String>>,
    },
    RunSelection {
        content: String,
        selection: SelectionRange,
        env: Option<HashMap<String, String>>,
    },
    RunFile {
        content: String,
        env: Option<HashMap<String, String>>,
    },
}

static LAST_EXECUTION: OnceLock<Mutex<Option<LastExecutionCommand>>> = OnceLock::new();

fn last_execution_store() -> &'static Mutex<Option<LastExecutionCommand>> {
    LAST_EXECUTION.get_or_init(|| Mutex::new(None))
}

fn remember_last(command: LastExecutionCommand) {
    let mut guard = last_execution_store()
        .lock()
        .expect("last execution mutex poisoned");
    *guard = Some(command);
}

fn last_result_or_error(
    mut results: Vec<ExecutionResult>,
    command_name: &str,
) -> Result<ExecutionResult, String> {
    results
        .pop()
        .ok_or_else(|| format!("{} produced no execution results", command_name))
}

fn format_runner_errors(result: &hurl::runner::HurlResult, content: &str) -> Option<String> {
    let errors = result
        .errors()
        .into_iter()
        .map(|(e, _)| format_runner_error(e, content))
        .collect::<Vec<_>>();

    if errors.is_empty() {
        None
    } else {
        Some(errors.join(", "))
    }
}

fn format_entry_errors(entry: &hurl::runner::EntryResult, content: &str) -> Option<String> {
    let errors = entry
        .errors
        .iter()
        .map(|e| format_runner_error(e, content))
        .collect::<Vec<_>>();

    if errors.is_empty() {
        None
    } else {
        Some(errors.join(", "))
    }
}

fn format_runner_error(error: &hurl::runner::RunnerError, content: &str) -> String {
    error.render("", content, None, OutputFormat::Plain)
}

pub fn parse_hurl_entries(content: &str) -> Result<Vec<EntryInfo>, String> {
    let hurl_file = hurl_core::parser::parse_hurl_file(content)
        .map_err(|e| format!("line {}, col {}: {:?}", e.pos.line, e.pos.column, e.kind))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut search_from = 0usize;

    let entries: Vec<EntryInfo> = hurl_file
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let method = format_method(&entry.request.method);
            let url = entry.request.url.to_string();
            let source_line = entry.request.source_info.start.line;
            let actual_start_idx =
                find_entry_start_line(&lines, &method, &url, source_line, search_from)
                    .unwrap_or_else(|| source_line.saturating_sub(1));

            search_from = actual_start_idx.saturating_add(1);

            EntryInfo {
                index,
                start_line: actual_start_idx.saturating_add(1) as u32,
                end_line: entry.request.source_info.end.line as u32,
                method,
                url,
            }
        })
        .collect();

    Ok(entries)
}

fn format_method(method: &hurl_core::ast::Method) -> String {
    // Method implements Display, returns the HTTP method string
    method.to_string().to_uppercase()
}

fn find_entry_start_line(
    lines: &[&str],
    method: &str,
    url: &str,
    source_line: usize,
    search_from: usize,
) -> Option<usize> {
    if lines.is_empty() {
        return None;
    }

    let method_upper = method.to_uppercase();
    let preferred_start = source_line.saturating_sub(1).max(search_from);

    // Prefer nearest line from parser-provided start to keep alignment stable.
    for idx in preferred_start..lines.len() {
        if line_matches_request(lines[idx], &method_upper, Some(url)) {
            return Some(idx);
        }
    }

    // Fallback: if URL representation differs, match method only from same starting point.
    for idx in preferred_start..lines.len() {
        if line_matches_request(lines[idx], &method_upper, None) {
            return Some(idx);
        }
    }

    // Last resort: search full remaining range from previous entry onward.
    for idx in search_from..preferred_start.min(lines.len()) {
        if line_matches_request(lines[idx], &method_upper, Some(url)) {
            return Some(idx);
        }
    }

    for idx in search_from..preferred_start.min(lines.len()) {
        if line_matches_request(lines[idx], &method_upper, None) {
            return Some(idx);
        }
    }

    None
}

fn line_matches_request(line: &str, method_upper: &str, url: Option<&str>) -> bool {
    let trimmed = line.trim();
    let trimmed_upper = trimmed.to_uppercase();
    if !trimmed_upper.starts_with(method_upper) {
        return false;
    }

    if let Some(target_url) = url {
        let normalized_url = target_url.trim();
        if !normalized_url.is_empty() {
            return trimmed.contains(normalized_url);
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{parse_hurl_entries, run_entry, run_selection};
    use crate::models::SelectionRange;

    #[test]
    fn parses_last_entry_without_missing_run_line() {
        let content = r#"GET https://jsonplaceholder.typicode.com/todos/
HTTP 200
[Asserts]
header \"Content-Type\" contains \"application/json\"

GET https://jsonplaceholder.typicode.com/todos/1
HTTP 200
[Asserts]
header \"Content-Type\" contains \"application/json\"

GET https://jsonplaceholder.typicode.com/todos/1
HTTP 200
[Asserts]
header \"Content-Type\" contains \"application/json\"

GET https://jsonplaceholder.typicode.com/todos/1
HTTP 200
[Asserts]
header \"Content-Type\" contains \"application/json\""#;

        let entries = parse_hurl_entries(content).expect("expected parse success");
        let starts: Vec<u32> = entries.iter().map(|entry| entry.start_line).collect();

        assert_eq!(starts, vec![1, 6, 12, 18]);
    }

    #[test]
    fn run_selection_rejects_invalid_range_order() {
        let content = "GET https://example.com\nHTTP 200";
        let err = run_selection(
            content,
            SelectionRange {
                start_line: 3,
                end_line: 2,
            },
            None,
        )
        .expect_err("expected invalid range to error");

        assert!(err.contains("start_line"));
    }

    #[test]
    fn run_selection_rejects_start_outside_file() {
        let content = "GET https://example.com\nHTTP 200";
        let err = run_selection(
            content,
            SelectionRange {
                start_line: 10,
                end_line: 10,
            },
            None,
        )
        .expect_err("expected out-of-range selection to error");

        assert!(err.contains("outside file"));
    }

    #[test]
    fn run_entry_surfaces_undefined_variable_error() {
        let content = "GET {{base_url}}/api/files?path=examples\nHTTP 200";
        let err = run_entry(content, 0, None).expect_err("expected undefined variable error");

        assert!(err.contains("Undefined") || err.contains("base_url"));
    }
}

pub fn run_entry(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<ExecutionResult, String> {
    let result = run_entry_inner(content, entry_index, env.clone())?;
    remember_last(LastExecutionCommand::RunEntry {
        content: content.to_string(),
        entry_index,
        env,
    });
    Ok(result)
}

pub fn build_assertions_for_entry(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<BuildAssertionsResponse, String> {
    let result = run_entry_inner(content, entry_index, env)?;
    let assertions = generate_json_assertions(&result.body);
    let (updated_content, assertions_added) =
        inject_assertions_into_entry(content, entry_index, &assertions)?;

    Ok(BuildAssertionsResponse {
        content: updated_content,
        assertions_added,
    })
}

fn generate_json_assertions(body: &str) -> Vec<String> {
    let Ok(json) = serde_json::from_str::<JsonValue>(body) else {
        return Vec::new();
    };

    let mut assertions = Vec::new();
    let mut seen = HashSet::new();
    collect_json_assertions(&json, "$", 0, &mut assertions, &mut seen);
    assertions
}

fn collect_json_assertions(
    value: &JsonValue,
    path: &str,
    depth: usize,
    assertions: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    if assertions.len() >= 40 {
        return;
    }

    match value {
        JsonValue::Null => push_unique(assertions, seen, format!("jsonpath \"{}\" == null", path)),
        JsonValue::Bool(v) => {
            push_unique(assertions, seen, format!("jsonpath \"{}\" == {}", path, v))
        }
        JsonValue::Number(v) => {
            push_unique(assertions, seen, format!("jsonpath \"{}\" == {}", path, v))
        }
        JsonValue::String(v) => push_unique(
            assertions,
            seen,
            format!("jsonpath \"{}\" == \"{}\"", path, escape_hurl_string(v)),
        ),
        JsonValue::Array(items) => {
            push_unique(
                assertions,
                seen,
                format!("jsonpath \"{}\" count == {}", path, items.len()),
            );
            push_unique(assertions, seen, format!("jsonpath \"{}\" exists", path));
            if depth < 2 {
                if let Some(first) = items.first() {
                    collect_json_assertions(
                        first,
                        &format!("{}[0]", path),
                        depth + 1,
                        assertions,
                        seen,
                    );
                }
            }
        }
        JsonValue::Object(map) => {
            if path != "$" {
                push_unique(assertions, seen, format!("jsonpath \"{}\" exists", path));
            }
            if depth < 2 {
                for (key, val) in map {
                    let key_path = append_jsonpath(path, key);
                    collect_json_assertions(val, &key_path, depth + 1, assertions, seen);
                    if assertions.len() >= 40 {
                        break;
                    }
                }
            }
        }
    }
}

fn push_unique(assertions: &mut Vec<String>, seen: &mut HashSet<String>, line: String) {
    if seen.insert(line.clone()) {
        assertions.push(line);
    }
}

fn append_jsonpath(base: &str, key: &str) -> String {
    if key
        .chars()
        .next()
        .map(|c| c == '_' || c.is_ascii_alphabetic())
        .unwrap_or(false)
        && key.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        format!("{}.{}", base, key)
    } else {
        format!("{}[\"{}\"]", base, escape_hurl_string(key))
    }
}

fn escape_hurl_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn inject_assertions_into_entry(
    content: &str,
    entry_index: usize,
    assertions: &[String],
) -> Result<(String, usize), String> {
    if assertions.is_empty() {
        return Ok((content.to_string(), 0));
    }

    let mut ordered = parse_hurl_entries(content)?;
    ordered.sort_by_key(|entry| entry.start_line);
    let target_pos = ordered
        .iter()
        .position(|entry| entry.index == entry_index)
        .ok_or_else(|| "Entry ordering failure".to_string())?;
    let target_start_line = ordered[target_pos].start_line;

    let start = target_start_line.saturating_sub(1) as usize;
    let end_exclusive = if target_pos + 1 < ordered.len() {
        ordered[target_pos + 1].start_line.saturating_sub(1) as usize
    } else {
        content.lines().count()
    };

    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let block = lines[start..end_exclusive].to_vec();
    let asserts_header = block
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[Asserts]"));

    let added = if let Some(header_idx) = asserts_header {
        let mut section_end = block.len();
        for (i, line) in block.iter().enumerate().skip(header_idx + 1) {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section_end = i;
                break;
            }
        }

        let existing = block[header_idx + 1..section_end]
            .iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect::<HashSet<_>>();
        let to_add = assertions
            .iter()
            .filter(|line| !existing.contains(line.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let insert_at = start + section_end;
        lines.splice(insert_at..insert_at, to_add.clone());
        to_add.len()
    } else {
        let http_line = block
            .iter()
            .position(|line| line.trim_start().to_uppercase().starts_with("HTTP "))
            .ok_or_else(|| "Unable to locate HTTP status line for entry".to_string())?;

        let insert_at = start + http_line + 1;
        let mut to_insert = Vec::with_capacity(assertions.len() + 1);
        to_insert.push("[Asserts]".to_string());
        to_insert.extend(assertions.iter().cloned());
        lines.splice(insert_at..insert_at, to_insert);
        assertions.len()
    };

    if added == 0 {
        return Ok((content.to_string(), 0));
    }

    Ok((lines.join("\n"), added))
}

fn run_entry_inner(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<ExecutionResult, String> {
    let entries = parse_hurl_entries(content)?;
    let _entry = entries
        .get(entry_index)
        .ok_or("Entry index out of bounds")?;

    // For now, run the entire file and return the requested entry
    // In a more sophisticated implementation, we would extract just the target entry
    let results = run_file(content, env)?;
    let mut first_error: Option<String> = None;
    for result in results {
        if result.entry_index == entry_index {
            return Ok(result);
        }
        if first_error.is_none() {
            first_error = result.error;
        }
    }

    if let Some(err) = first_error {
        return Err(err);
    }

    Err("Entry execution failed".to_string())
}

pub fn run_to_end(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<ExecutionResult>, String> {
    let results = run_to_end_inner(content, entry_index, env.clone())?;
    remember_last(LastExecutionCommand::RunToEnd {
        content: content.to_string(),
        entry_index,
        env,
    });
    Ok(results)
}

fn run_to_end_inner(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<ExecutionResult>, String> {
    let results = run_file_inner(content, env)?;
    Ok(results
        .into_iter()
        .filter(|r| r.entry_index >= entry_index)
        .collect())
}

pub fn run_from_begin(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<ExecutionResult>, String> {
    let results = run_from_begin_inner(content, entry_index, env.clone())?;
    remember_last(LastExecutionCommand::RunFromBegin {
        content: content.to_string(),
        entry_index,
        env,
    });
    Ok(results)
}

fn run_from_begin_inner(
    content: &str,
    entry_index: usize,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<ExecutionResult>, String> {
    let results = run_file_inner(content, env)?;
    Ok(results
        .into_iter()
        .filter(|r| r.entry_index <= entry_index)
        .collect())
}

pub fn run_selection(
    content: &str,
    selection: SelectionRange,
    env: Option<HashMap<String, String>>,
) -> Result<ExecutionResult, String> {
    let result = run_selection_inner(content, selection.clone(), env.clone())?;
    remember_last(LastExecutionCommand::RunSelection {
        content: content.to_string(),
        selection,
        env,
    });
    Ok(result)
}

fn run_selection_inner(
    content: &str,
    selection: SelectionRange,
    env: Option<HashMap<String, String>>,
) -> Result<ExecutionResult, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err("Cannot run selection on empty content".to_string());
    }

    if selection.start_line == 0 || selection.end_line == 0 {
        return Err("Selection lines are 1-based and must be greater than 0".to_string());
    }

    if selection.start_line > selection.end_line {
        return Err("Selection start_line must be <= end_line".to_string());
    }

    let start_idx = (selection.start_line as usize).saturating_sub(1);
    let end_idx = (selection.end_line as usize).min(lines.len());

    if start_idx >= lines.len() {
        return Err(format!(
            "Selection start_line {} is outside file ({} lines)",
            selection.start_line,
            lines.len()
        ));
    }

    if start_idx >= end_idx {
        return Err("Selection resolved to an empty range".to_string());
    }

    let selected_content = lines[start_idx..end_idx].join("\n");

    // Run the selected content as a single entry
    let runner_options = build_runner_options(&env);
    let variables = build_variables(&env);
    let logger_options = LoggerOptionsBuilder::new().build();

    let result = runner::run(
        &selected_content,
        None,
        &runner_options,
        &variables,
        &logger_options,
    )
    .map_err(|e| format!("Failed to run selection: {:?}", e))?;

    convert_hurl_result_to_execution_result(&result, 0, &selected_content)
}

pub fn run_file(
    content: &str,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<ExecutionResult>, String> {
    let results = run_file_inner(content, env.clone())?;
    remember_last(LastExecutionCommand::RunFile {
        content: content.to_string(),
        env,
    });
    Ok(results)
}

fn run_file_inner(
    content: &str,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<ExecutionResult>, String> {
    let runner_options = build_runner_options(&env);
    let variables = build_variables(&env);
    let logger_options = LoggerOptionsBuilder::new().build();

    let result = runner::run(content, None, &runner_options, &variables, &logger_options)
        .map_err(|e| format!("Failed to run Hurl file: {:?}", e))?;

    let mut results = Vec::new();

    for (index, entry) in result.entries.iter().enumerate() {
        match convert_entry_to_execution_result(entry, index, content) {
            Ok(exec_result) => results.push(exec_result),
            Err(e) => {
                results.push(ExecutionResult {
                    entry_index: index,
                    status: 0,
                    headers: HashMap::new(),
                    body: String::new(),
                    timing: None,
                    assertions: Vec::new(),
                    error: Some(e),
                });
            }
        }
    }

    if results.is_empty() {
        if let Some(error_msg) = format_runner_errors(&result, content) {
            return Err(error_msg);
        }

        return Err("No executable entries found".to_string());
    }

    Ok(results)
}

pub fn test_file(
    content: &str,
    env: Option<HashMap<String, String>>,
) -> Result<TestFileResponse, String> {
    let results = run_file_inner(content, env)?;

    let mut total_assertions = 0;
    let mut passed_assertions = 0;
    let mut failed_assertions = 0;

    for result in &results {
        for assertion in &result.assertions {
            total_assertions += 1;
            if assertion.passed {
                passed_assertions += 1;
            } else {
                failed_assertions += 1;
            }
        }
    }

    let has_execution_errors = results.iter().any(|result| result.error.is_some());

    Ok(TestFileResponse {
        overall_pass: failed_assertions == 0 && !has_execution_errors,
        total_assertions,
        passed_assertions,
        failed_assertions,
        results,
    })
}

pub fn rerun_last() -> Result<ExecutionResult, String> {
    let command = {
        let guard = last_execution_store()
            .lock()
            .map_err(|_| "Last execution state unavailable".to_string())?;
        guard.clone()
    }
    .ok_or_else(|| "No previous run available to rerun".to_string())?;

    match command {
        LastExecutionCommand::RunEntry {
            content,
            entry_index,
            env,
        } => run_entry_inner(&content, entry_index, env),
        LastExecutionCommand::RunSelection {
            content,
            selection,
            env,
        } => run_selection_inner(&content, selection, env),
        LastExecutionCommand::RunToEnd {
            content,
            entry_index,
            env,
        } => {
            let results = run_to_end_inner(&content, entry_index, env)?;
            last_result_or_error(results, "run-to-end")
        }
        LastExecutionCommand::RunFromBegin {
            content,
            entry_index,
            env,
        } => {
            let results = run_from_begin_inner(&content, entry_index, env)?;
            last_result_or_error(results, "run-from-begin")
        }
        LastExecutionCommand::RunFile { content, env } => {
            let results = run_file_inner(&content, env)?;
            last_result_or_error(results, "run-file")
        }
    }
}

pub fn cancel_execution(run_id: &str) -> Result<CancelResponse, String> {
    // Placeholder implementation
    Ok(CancelResponse {
        success: true,
        message: format!("Cancellation requested for run ID: {}", run_id),
    })
}

// Helper functions
fn build_runner_options(_env: &Option<HashMap<String, String>>) -> runner::RunnerOptions {
    RunnerOptionsBuilder::new().build()
}

fn build_variables(env: &Option<HashMap<String, String>>) -> VariableSet {
    let mut variables = VariableSet::new();
    if let Some(values) = env {
        for (key, value) in values {
            variables.insert(key.clone(), Value::String(value.clone()));
        }
    }
    variables
}

fn convert_hurl_result_to_execution_result(
    result: &hurl::runner::HurlResult,
    entry_index: usize,
    content: &str,
) -> Result<ExecutionResult, String> {
    let entry = result
        .entries
        .first()
        .ok_or_else(|| "No entry found in Hurl result".to_string())?;

    convert_entry_to_execution_result(entry, entry_index, content)
}

fn convert_entry_to_execution_result(
    entry: &hurl::runner::EntryResult,
    entry_index: usize,
    content: &str,
) -> Result<ExecutionResult, String> {
    let call = entry.calls.first().ok_or_else(|| {
        format_entry_errors(entry, content).unwrap_or_else(|| "No call found in entry".to_string())
    })?;

    let status = u16::try_from(call.response.status).unwrap_or(0);

    let headers: HashMap<String, String> = call
        .response
        .headers
        .iter()
        .map(|h| (h.name.clone(), h.value.clone()))
        .collect();

    let body = String::from_utf8_lossy(&call.response.body).into_owned();

    let duration_ms = (call.timings.total.as_secs_f64() * 1000.0) as u64;
    let connect_ms = None; // Simplified for now
    let tls_ms = None; // Simplified for now
    let transfer_ms = None; // Simplified for now

    let assertions = entry
        .asserts
        .iter()
        .map(to_assertion_result)
        .collect::<Vec<_>>();

    Ok(ExecutionResult {
        entry_index,
        status,
        headers,
        body,
        timing: Some(TimingInfo {
            duration_ms,
            connect_time_ms: connect_ms,
            tls_time_ms: tls_ms,
            transfer_time_ms: transfer_ms,
        }),
        assertions,
        error: entry
            .errors
            .first()
            .map(|e| format_runner_error(e, content)),
    })
}

fn to_assertion_result(assert: &AssertResult) -> AssertionResult {
    match assert {
        AssertResult::ImplicitVersion {
            actual, expected, ..
        } => AssertionResult {
            query: "version".to_string(),
            predicate: "==".to_string(),
            expected: expected.clone(),
            actual: actual.clone(),
            passed: actual == expected,
        },
        AssertResult::ImplicitStatus {
            actual, expected, ..
        } => AssertionResult {
            query: "status".to_string(),
            predicate: "==".to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
            passed: actual == expected,
        },
        AssertResult::ImplicitHeader {
            actual, expected, ..
        } => {
            let (actual_value, passed) = match actual {
                Ok(value) => (value.clone(), value == expected),
                Err(e) => (format!("error: {:?}", e.kind), false),
            };
            AssertionResult {
                query: "header".to_string(),
                predicate: "==".to_string(),
                expected: expected.clone(),
                actual: actual_value,
                passed,
            }
        }
        AssertResult::ImplicitBody {
            actual, expected, ..
        } => {
            let expected_value = match expected {
                Ok(value) => format!("{value:?}"),
                Err(e) => format!("error: {:?}", e.kind),
            };
            let (actual_value, passed) = match actual {
                Ok(value) => {
                    let actual_formatted = format!("{value:?}");
                    let is_equal = matches!(expected, Ok(exp) if value == exp);
                    (actual_formatted, is_equal)
                }
                Err(e) => (format!("error: {:?}", e.kind), false),
            };

            AssertionResult {
                query: "body".to_string(),
                predicate: "==".to_string(),
                expected: expected_value,
                actual: actual_value,
                passed,
            }
        }
        AssertResult::Explicit {
            actual,
            predicate_result,
            ..
        } => {
            let actual_value = match actual {
                Ok(Some(value)) => format!("{value:?}"),
                Ok(None) => "null".to_string(),
                Err(e) => format!("error: {:?}", e.kind),
            };
            let (passed, expected_value) = match predicate_result {
                Some(Ok(())) => (true, "predicate passed".to_string()),
                Some(Err(e)) => (false, format!("{:?}", e.kind)),
                None => (false, "predicate unavailable".to_string()),
            };

            AssertionResult {
                query: "explicit".to_string(),
                predicate: "custom".to_string(),
                expected: expected_value,
                actual: actual_value,
                passed,
            }
        }
    }
}
