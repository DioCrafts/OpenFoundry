use crate::models::{category::CategoryDefinition, listing::ListingDefinition};

pub fn score_listing(listing: &ListingDefinition, query: &str) -> f64 {
    let query = query.to_lowercase();
    let mut score = 0.4;
    if listing.name.to_lowercase().contains(&query) {
        score += 0.25;
    }
    if listing.summary.to_lowercase().contains(&query) {
        score += 0.2;
    }
    if listing
        .tags
        .iter()
        .any(|tag| tag.to_lowercase().contains(&query))
    {
        score += 0.1;
    }
    (score + listing.average_rating / 10.0).min(0.99)
}

pub fn featured_categories(listings: &[ListingDefinition]) -> Vec<CategoryDefinition> {
    let mut categories = vec![
        CategoryDefinition::new(
            "connectors",
            "Connectors",
            "Operational and SaaS integrations",
        ),
        CategoryDefinition::new(
            "transforms",
            "Transforms",
            "Reusable data transformation packages",
        ),
        CategoryDefinition::new("widgets", "Widgets", "UI widgets and dashboards"),
        CategoryDefinition::new(
            "templates",
            "App Templates",
            "Starter apps and composition templates",
        ),
        CategoryDefinition::new(
            "ml-models",
            "ML Models",
            "Packaged models and inference adapters",
        ),
        CategoryDefinition::new("ai-agents", "AI Agents", "Agent workflows and copilots"),
    ];

    for category in &mut categories {
        category.listing_count = listings
            .iter()
            .filter(|listing| listing.category_slug == category.slug)
            .count();
    }

    categories
}
