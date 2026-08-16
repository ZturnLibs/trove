/// Extract unique `[[title]]` targets from memory body (order preserved, first occurrence).
pub fn parse_wikilink_titles(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else {
            break;
        };
        let title = after[..end].trim();
        if !title.is_empty() && seen.insert(title.to_ascii_lowercase()) {
            out.push(title.to_string());
        }
        rest = &after[end + 2..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unique_wikilinks() {
        assert_eq!(
            parse_wikilink_titles("见 [[Alpha]] 与 [[Beta]]，再次 [[Alpha]]"),
            vec!["Alpha".to_string(), "Beta".to_string()]
        );
    }
}
