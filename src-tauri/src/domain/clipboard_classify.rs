use super::{parse_capture, ClipboardKindHint};

/// Lightweight local rule classifier for clipboard smart actions.
pub fn classify_clipboard_text(text: &str, timezone: &str) -> ClipboardKindHint {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ClipboardKindHint::Plain;
    }
    if looks_like_error(trimmed) {
        return ClipboardKindHint::Error;
    }
    if looks_like_code(trimmed) {
        return ClipboardKindHint::Code;
    }
    if looks_like_url(trimmed) {
        return ClipboardKindHint::Url;
    }
    if looks_like_email(trimmed) {
        return ClipboardKindHint::Email;
    }
    if looks_like_phone(trimmed) {
        return ClipboardKindHint::Phone;
    }
    if looks_like_date(trimmed, timezone) {
        return ClipboardKindHint::Date;
    }
    ClipboardKindHint::Plain
}

fn looks_like_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    const MARKERS: &[&str] = &[
        "error:",
        "exception",
        "traceback",
        "panic!",
        "panic at",
        "stack overflow",
        "failed",
        "errno",
        "econnrefused",
        "segmentation fault",
        "uncaught",
        "at line ",
        " Caused by:",
    ];
    MARKERS.iter().any(|m| lower.contains(m))
        || text.lines().any(|line| {
            line.trim_start().starts_with("at ")
                && line.contains(':')
                && line.contains('.')
        })
}

fn looks_like_code(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() >= 2 {
        let indented = lines
            .iter()
            .filter(|l| l.starts_with("  ") || l.starts_with('\t'))
            .count();
        if indented >= 2 && indented * 2 >= lines.len() {
            return true;
        }
    }
    let lower = text.to_lowercase();
    const KEYWORDS: &[&str] = &[
        "function ",
        "class ",
        "import ",
        "export ",
        "def ",
        "const ",
        "let ",
        "var ",
        "=>",
        "public ",
        "private ",
        "#include",
        "fn ",
        "impl ",
    ];
    if KEYWORDS.iter().any(|k| lower.contains(k)) {
        return true;
    }
    let specials = text
        .chars()
        .filter(|c| "{}();[]".contains(*c))
        .count();
    specials >= 4 && text.len() >= 24
}

fn looks_like_url(text: &str) -> bool {
    let first = text.lines().next().unwrap_or(text).trim();
    first.starts_with("http://")
        || first.starts_with("https://")
        || first.starts_with("www.")
        || (first.contains("://") && !first.contains(' '))
}

fn looks_like_email(text: &str) -> bool {
    let first = text.lines().next().unwrap_or(text).trim();
    if first.contains(' ') {
        return false;
    }
    let Some(at) = first.find('@') else {
        return false;
    };
    at > 0 && first[at + 1..].contains('.')
}

fn looks_like_phone(text: &str) -> bool {
    let compact: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    if compact.starts_with("+86") && compact.len() >= 13 {
        return true;
    }
    if compact.len() == 11 && compact.starts_with('1') {
        return true;
    }
    compact.starts_with('+') && compact.len() >= 10 && compact.len() <= 16
}

fn looks_like_date(text: &str, timezone: &str) -> bool {
    if parse_capture(text, timezone).due_date.is_some() {
        return true;
    }
    const TOKENS: &[&str] = &["明天", "后天", "今天", "下周", "周一", "周二", "周三", "周四", "周五"];
    TOKENS.iter().any(|t| text.contains(t))
        || text
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-' || *c == '/')
            .count()
            >= 8
            && (text.contains('-') || text.contains('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_url_email_code_error() {
        assert_eq!(
            classify_clipboard_text("https://example.com/docs", "Asia/Shanghai"),
            ClipboardKindHint::Url
        );
        assert_eq!(
            classify_clipboard_text("dev@example.com", "Asia/Shanghai"),
            ClipboardKindHint::Email
        );
        assert_eq!(
            classify_clipboard_text("fn main() {\n  println!(\"hi\");\n}", "Asia/Shanghai"),
            ClipboardKindHint::Code
        );
        assert_eq!(
            classify_clipboard_text("Error: connection refused\n  at foo.bar:12", "Asia/Shanghai"),
            ClipboardKindHint::Error
        );
    }

    #[test]
    fn classifies_date_via_nl() {
        assert_eq!(
            classify_clipboard_text("明天下午开会", "Asia/Shanghai"),
            ClipboardKindHint::Date
        );
    }
}
