use super::*;

/// Admin category row with approved-tool count.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCategoryView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub sort_order: i32,
    pub tool_count: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CategoryInput {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub sort_order: i32,
}

impl CategoryInput {
    fn id(&self) -> &str {
        self.id.trim()
    }
    fn label(&self) -> &str {
        self.label.trim()
    }
    fn icon(&self) -> &str {
        self.icon.trim()
    }
    fn description(&self) -> &str {
        self.description.trim()
    }
}

/// Validate category id/label/icon/description for admin CRUD.
pub(crate) fn validate_category_input(input: &CategoryInput) -> Result<(), &'static str> {
    validate_category_id(input.id())?;
    validate_category_label(input.label())?;
    validate_category_icon(input.icon())?;
    validate_category_description(input.description())?;
    validate_category_sort_order(input.sort_order)
}

fn validate_category_id(id: &str) -> Result<(), &'static str> {
    validate_text_len(id, 2, 32, "category id must be 2–32 characters")?;
    category_id_chars_allowed(id)
        .then_some(())
        .ok_or("category id must be lowercase letters, digits, or hyphens")
}

fn category_id_chars_allowed(id: &str) -> bool {
    id.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn validate_category_label(label: &str) -> Result<(), &'static str> {
    validate_text_len(label, 1, 100, "label must be 1–100 characters")
}

fn validate_category_icon(icon: &str) -> Result<(), &'static str> {
    validate_text_len(icon, 1, 32, "icon must be 1–32 characters")?;
    icon_chars_allowed(icon)
        .then_some(())
        .ok_or("icon may only contain letters, numbers, and hyphens")
}

fn icon_chars_allowed(icon: &str) -> bool {
    icon.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn validate_category_description(description: &str) -> Result<(), &'static str> {
    validate_text_len(description, 1, 500, "description must be 1–500 characters")
}

fn validate_text_len(
    value: &str,
    min_len: usize,
    max_len: usize,
    message: &'static str,
) -> Result<(), &'static str> {
    (min_len..=max_len)
        .contains(&value.len())
        .then_some(())
        .ok_or(message)
}

fn validate_category_sort_order(sort_order: i32) -> Result<(), &'static str> {
    (0..=9999)
        .contains(&sort_order)
        .then_some(())
        .ok_or("sort order must be 0–9999")
}
