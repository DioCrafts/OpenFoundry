use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct CatalogTagFacet {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CatalogOwnerFacet {
    pub owner_id: Uuid,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct CatalogFacets {
    pub tags: Vec<CatalogTagFacet>,
    pub owners: Vec<CatalogOwnerFacet>,
}

pub async fn fetch_catalog_facets(pool: &sqlx::PgPool) -> Result<CatalogFacets, sqlx::Error> {
    let tags = sqlx::query_as::<_, CatalogTagFacet>(
        r#"SELECT tag AS value, COUNT(*) AS count
		   FROM datasets, unnest(tags) AS tag
		   GROUP BY tag
		   ORDER BY count DESC, tag ASC"#,
    )
    .fetch_all(pool)
    .await?;

    let owners = sqlx::query_as::<_, CatalogOwnerFacet>(
        r#"SELECT owner_id, COUNT(*) AS count
		   FROM datasets
		   GROUP BY owner_id
		   ORDER BY count DESC, owner_id ASC"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(CatalogFacets { tags, owners })
}
