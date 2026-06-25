//! Category model — maps the `categories` table.

/// A function category row from the `categories` table.
///
/// `id` is a TEXT primary key (e.g. `"bridge"`), not a UUID.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: String,
    pub label: String,
    /// Lucide icon name (e.g. `"git-branch"`). Never an emoji.
    pub icon: String,
    pub description: String,
    pub sort_order: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_serde_round_trip() {
        let cat = Category {
            id: "bridge".into(),
            label: "Bridge & Cross-chain".into(),
            icon: "git-branch".into(),
            description: "Cross-chain transfers, bridging, wrapping".into(),
            sort_order: 1,
        };
        let json = serde_json::to_string(&cat).expect("serialize category");
        let back: Category = serde_json::from_str(&json).expect("deserialize category");
        assert_eq!(back.id, "bridge");
        assert_eq!(back.sort_order, 1);
    }
}
