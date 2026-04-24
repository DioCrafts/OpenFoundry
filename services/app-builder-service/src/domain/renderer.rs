use crate::{
    domain::embedding::build_embed_info,
    models::{
        app::{App, AppPreviewResponse, PublishedAppResponse},
        version::AppVersion,
        widget_type::widget_catalog,
    },
};

pub fn build_preview_response(app: App, public_base_url: &str) -> AppPreviewResponse {
    AppPreviewResponse {
        embed: build_embed_info(public_base_url, &app.slug),
        widget_catalog: widget_catalog(),
        app,
    }
}

pub fn build_published_response(
    app: App,
    version: AppVersion,
    public_base_url: &str,
) -> PublishedAppResponse {
    let snapshot = version.app_snapshot.clone();

    let published_app = App {
        id: app.id,
        name: snapshot.name,
        slug: snapshot.slug,
        description: snapshot.description,
        status: snapshot.status,
        pages: snapshot.pages,
        theme: snapshot.theme,
        settings: snapshot.settings,
        template_key: snapshot.template_key,
        created_by: app.created_by,
        published_version_id: Some(version.id),
        created_at: app.created_at,
        updated_at: version.published_at.unwrap_or(app.updated_at),
    };

    PublishedAppResponse {
        embed: build_embed_info(public_base_url, &published_app.slug),
        app: published_app,
        published_version_number: version.version_number,
        published_at: version.published_at.unwrap_or(version.created_at),
    }
}
