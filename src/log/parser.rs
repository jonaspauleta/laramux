#![allow(dead_code)]

/// Detect if a line is part of a stack trace
pub fn is_stack_trace_line(line: &str) -> bool {
    let trimmed = line.trim();

    // Laravel/PHP stack trace patterns
    trimmed.starts_with("#")
        && trimmed
            .chars()
            .nth(1)
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        || trimmed.starts_with("Stack trace:")
        || trimmed.contains(" at ")
            && (trimmed.contains(".php:") || trimmed.contains("vendor/"))
        || trimmed.starts_with("in ") && trimmed.contains(".php")
        || trimmed.starts_with("at ") && trimmed.contains("::")
}

/// Detect error level from Laravel log line
pub fn detect_error_level(line: &str) -> Option<&str> {
    // Laravel log format: [YYYY-MM-DD HH:MM:SS] environment.LEVEL: message
    if let Some(bracket_end) = line.find(']') {
        let after_bracket = &line[bracket_end + 1..];
        if let Some(colon) = after_bracket.find(':') {
            let level_part = after_bracket[..colon].trim();
            if let Some(dot) = level_part.rfind('.') {
                let level = &level_part[dot + 1..];
                return Some(level);
            }
        }
    }
    None
}

/// Check if a log line indicates an error
pub fn is_error_line(line: &str) -> bool {
    if let Some(level) = detect_error_level(line) {
        let level_lower = level.to_lowercase();
        matches!(
            level_lower.as_str(),
            "error" | "critical" | "alert" | "emergency"
        )
    } else {
        // Fallback to content-based detection
        let lower = line.to_lowercase();
        lower.contains("exception")
            || lower.contains("error:")
            || lower.contains("fatal")
            || lower.contains("[error]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_error_level() {
        let line = "[2024-01-15 10:30:45] local.ERROR: Test error message";
        assert_eq!(detect_error_level(line), Some("ERROR"));

        let line = "[2024-01-15 10:30:45] production.INFO: Application started";
        assert_eq!(detect_error_level(line), Some("INFO"));
    }

    #[test]
    fn test_is_stack_trace_line() {
        assert!(is_stack_trace_line("#0 /var/www/app/Http/Controller.php(45)"));
        assert!(is_stack_trace_line("Stack trace:"));
        assert!(!is_stack_trace_line("Normal log message"));
    }

    #[test]
    fn test_is_error_line() {
        assert!(is_error_line(
            "[2024-01-15 10:30:45] local.ERROR: Test error"
        ));
        assert!(!is_error_line(
            "[2024-01-15 10:30:45] local.INFO: Test info"
        ));
    }
}
