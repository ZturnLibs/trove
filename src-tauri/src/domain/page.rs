use serde::{Deserialize, Serialize};

pub fn page_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(200).clamp(1, 1000)
}

pub fn page_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub has_more: bool,
}

impl<T> PagedResult<T> {
    pub fn new(items: Vec<T>, total: i64, offset: i64) -> Self {
        let has_more = offset + (items.len() as i64) < total;
        Self {
            items,
            total,
            has_more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_limit_defaults_and_clamps() {
        assert_eq!(page_limit(None), 200);
        assert_eq!(page_limit(Some(0)), 1);
        assert_eq!(page_limit(Some(5000)), 1000);
    }

    #[test]
    fn paged_result_has_more() {
        let page = PagedResult::new(vec![1, 2], 5, 0);
        assert!(page.has_more);
        let last = PagedResult::new(vec![4, 5], 5, 3);
        assert!(!last.has_more);
    }
}
