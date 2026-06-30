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

/// List all categories with tool counts (admin).
#[server(ListAdminCategories, "/api")]
pub async fn list_admin_categories() -> Result<Vec<AdminCategoryView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, i32, i64)>(
        r#"
        SELECT c.id, c.label, c.icon, c.description, c.sort_order,
               COUNT(t.id) AS tool_count
        FROM categories c
        LEFT JOIN tools t ON t.function = c.id
          AND t.approval_status = 'approved'
          AND t.quarantined_at IS NULL
        GROUP BY c.id, c.label, c.icon, c.description, c.sort_order
        ORDER BY c.sort_order ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to list categories: {e}")))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, label, icon, description, sort_order, tool_count)| AdminCategoryView {
                id,
                label,
                icon,
                description,
                sort_order,
                tool_count,
            },
        )
        .collect())
}

/// Create a function category (admin).
#[server(CreateCategory, "/api")]
pub async fn create_category(input: CategoryInput) -> Result<Category, ServerFnError> {
    if let Err(msg) = validate_category_input(&input) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (id, label, icon, description, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.id())
    .bind(input.label())
    .bind(input.icon())
    .bind(input.description())
    .bind(input.sort_order)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to create category: {e}")))?;

    Ok(category)
}

/// Update a function category (admin).
#[server(UpdateCategory, "/api")]
pub async fn update_category(input: CategoryInput) -> Result<Category, ServerFnError> {
    if let Err(msg) = validate_category_input(&input) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET label = $2, icon = $3, description = $4, sort_order = $5
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(input.id())
    .bind(input.label())
    .bind(input.icon())
    .bind(input.description())
    .bind(input.sort_order)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to update category: {e}")))?
    .ok_or_else(|| ServerFnError::new(format!("category not found: {}", input.id())))?;

    Ok(category)
}

/// Delete a category when no tools reference it (admin).
#[server(DeleteCategory, "/api")]
pub async fn delete_category(id: String) -> Result<(), ServerFnError> {
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err(ServerFnError::new("category id required"));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let tool_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tools WHERE function = $1")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("tool count failed: {e}")))?;

    if tool_count > 0 {
        return Err(ServerFnError::new(
            "cannot delete category with linked tools — reassign tools first",
        ));
    }

    let result = sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to delete category: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(format!("category not found: {id}")));
    }

    Ok(())
}
