#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountType {
    Checking,
    Savings,
    CreditCard,
    Investment,
    Cash,
    Loan,
    Other,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Checking => "Checking",
            Self::Savings => "Savings",
            Self::CreditCard => "Credit Card",
            Self::Investment => "Investment",
            Self::Cash => "Cash",
            Self::Loan => "Loan",
            Self::Other => "Other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "checking" => Self::Checking,
            "savings" => Self::Savings,
            "credit card" | "creditcard" | "credit" => Self::CreditCard,
            "investment" => Self::Investment,
            "cash" => Self::Cash,
            "loan" => Self::Loan,
            _ => Self::Other,
        }
    }

    pub fn all() -> &'static [AccountType] {
        &[
            Self::Checking,
            Self::Savings,
            Self::CreditCard,
            Self::Investment,
            Self::Cash,
            Self::Loan,
            Self::Other,
        ]
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: Option<i64>,
    pub name: String,
    pub account_type: AccountType,
    pub institution: String,
    pub currency: String,
    pub notes: String,
    pub created_at: String,
}

impl Account {
    pub fn new(name: String, account_type: AccountType, institution: String) -> Self {
        Self {
            id: None,
            name,
            account_type,
            institution,
            currency: "USD".to_string(),
            notes: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
