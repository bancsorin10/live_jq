use std::iter::once;

use xq::{
    module_loader::PreludeLoader,
    run_query as xq_run_query,
    util::SharedIterator,
    InputError, Value,
};

/// Runs a jq query on the given JSON input string.
/// Returns pretty-printed JSON output on success, or an error message on failure.
pub fn run_query(json_input: &str, query: &str) -> Result<String, String> {
    let trimmed = json_input.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let value: Value =
        serde_json::from_str(json_input).map_err(|e| format!("Failed to parse JSON input: {e}"))?;

    if query.trim().is_empty() {
        return serde_json::to_string_pretty(&value)
            .map_err(|e| format!("Failed to format output: {e}"));
    }

    let input_iter = once(Ok::<Value, InputError>(value));
    let shared = SharedIterator::from(input_iter);
    let loader = PreludeLoader();

    let result_iter =
        xq_run_query(query, shared.clone(), shared, &loader).map_err(|e| format!("{e}"))?;

    let mut results = Vec::new();
    for item in result_iter {
        match item {
            Ok(v) => {
                let formatted = serde_json::to_string_pretty(&v)
                    .map_err(|e| format!("Failed to format output: {e}"))?;
                results.push(formatted);
            }
            Err(e) => return Err(format!("Runtime error: {e}")),
        }
    }

    Ok(results.join("\n"))
}

#[cfg(test)]
mod tests {
    use crate::query;

    #[test]
    fn test_empty_json() {
        let result = query::run_query("", "");
        assert!(result.is_ok());
    }

    #[test]
    fn test_identity_query() {
        let json = r#"{"name": "Alice", "age": 30}"#;
        let result = query::run_query(json, ".").unwrap();
        assert!(result.contains("Alice"));
        assert!(result.contains("30"));
    }

    #[test]
    fn test_field_access() {
        let json = r#"{"name": "Alice", "age": 30}"#;
        let result = query::run_query(json, ".name").unwrap();
        assert!(result.contains("Alice"));
    }

    #[test]
    fn test_array_index() {
        let json = r#"[10, 20, 30]"#;
        let result = query::run_query(json, ".[0]").unwrap();
        assert!(result.contains("10"));
    }

    #[test]
    fn test_invalid_json() {
        let result = query::run_query("not json", ".");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_query() {
        let json = r#"{"a": 1}"#;
        let result = query::run_query(json, "???");
        assert!(result.is_err());
    }
}
