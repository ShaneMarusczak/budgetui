use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct Budget {
    pub id: Option<i64>,
    pub category_id: i64,
    /// Format: "YYYY-MM"
    pub month: String,
    pub limit_amount: Decimal,
}

impl Budget {
    pub fn new(category_id: i64, month: String, limit_amount: Decimal) -> Self {
        Self {
            id: None,
            category_id,
            month,
            limit_amount,
        }
    }
}
