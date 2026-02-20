#[derive(Debug, Clone)]
pub struct ImportRule {
    pub id: Option<i64>,
    pub pattern: String,
    pub category_id: i64,
    pub is_regex: bool,
    pub priority: i32,
}

impl ImportRule {
    pub fn new_contains(pattern: String, category_id: i64) -> Self {
        Self {
            id: None,
            pattern,
            category_id,
            is_regex: false,
            priority: 0,
        }
    }

    pub fn new_regex(pattern: String, category_id: i64) -> Self {
        Self {
            id: None,
            pattern,
            category_id,
            is_regex: true,
            priority: 0,
        }
    }
}
