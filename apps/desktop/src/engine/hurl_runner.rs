use super::model::{ActualResponse, KeyValue};
use hurl::runner::{self, RunnerOptionsBuilder, VariableSet};
use hurl::util::logger::LoggerOptionsBuilder;
use std::path::Path;

#[allow(dead_code)]
pub fn run_hurl(file_path: &Path) -> Result<Vec<ActualResponse>, String> {
    let content =
        std::fs::read_to_string(file_path).map_err(|e| format!("failed to read hurl file: {e}"))?;
    run_hurl_content(&content)
}

pub fn run_hurl_content(content: &str) -> Result<Vec<ActualResponse>, String> {
    let runner_options = RunnerOptionsBuilder::new().build();
    let variables = VariableSet::new();
    let logger_options = LoggerOptionsBuilder::new().build();

    let result = runner::run(content, None, &runner_options, &variables, &logger_options)
        .map_err(|e| format!("failed to run hurl content: {e}"))?;

    to_actual_responses(&result)
}

fn to_actual_responses(result: &hurl::runner::HurlResult) -> Result<Vec<ActualResponse>, String> {
    let mut responses = Vec::new();

    for entry in &result.entries {
        for call in &entry.calls {
            let status = u16::try_from(call.response.status).unwrap_or(0);
            let headers = call
                .response
                .headers
                .iter()
                .map(|h| KeyValue::new(&h.name, &h.value))
                .collect();
            let size_bytes = call.response.body.len() as u64;
            let body = String::from_utf8_lossy(&call.response.body).into_owned();
            let time_ms = call.timings.total.as_secs_f64() * 1000.0;

            responses.push(ActualResponse {
                status,
                headers,
                body,
                time_ms,
                size_bytes,
            });
        }
    }

    if responses.is_empty() {
        if let Some((error, _)) = result.errors().into_iter().next() {
            return Err(format!("{:?}", error.kind));
        }
        return Err("no response received".to_string());
    }

    Ok(responses)
}
