#[derive(Debug, Clone)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
}

impl Category {
    pub fn new(name: String) -> Self {
        Self { id: None, name }
    }

    /// Find a category by name (case-insensitive) in a slice.
    pub fn find_by_name<'a>(categories: &'a [Category], name: &str) -> Option<&'a Category> {
        let lower = name.to_lowercase();
        categories.iter().find(|c| c.name.to_lowercase() == lower)
    }

    /// Find a category by ID in a slice.
    pub fn find_by_id(categories: &[Category], id: i64) -> Option<&Category> {
        categories.iter().find(|c| c.id == Some(id))
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
