#[derive(Debug, Clone)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub icon: String,
    pub color: String,
}

impl Category {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            icon: String::new(),
            color: String::new(),
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
