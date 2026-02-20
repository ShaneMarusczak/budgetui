use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Option<i64>,
    pub account_id: i64,
    pub date: String,
    pub description: String,
    pub original_description: String,
    pub amount: Decimal,
    pub category_id: Option<i64>,
    pub notes: String,
    pub is_transfer: bool,
    pub import_hash: String,
    pub created_at: String,
}

impl Transaction {
    pub fn is_income(&self) -> bool {
        self.amount > Decimal::ZERO
    }

    pub fn is_expense(&self) -> bool {
        self.amount < Decimal::ZERO
    }

    pub fn abs_amount(&self) -> Decimal {
        self.amount.abs()
    }
}
