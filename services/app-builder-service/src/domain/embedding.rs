use crate::models::app::AppEmbedInfo;

pub fn build_embed_info(public_base_url: &str, slug: &str) -> AppEmbedInfo {
    let base = public_base_url.trim_end_matches('/');
    let url = format!("{base}/{slug}");
    let iframe_html = format!(
        r#"<iframe src="{url}" title="{slug}" loading="lazy" style="width: 100%; min-height: 900px; border: 0; border-radius: 24px;"></iframe>"#,
    );

    AppEmbedInfo { url, iframe_html }
}
