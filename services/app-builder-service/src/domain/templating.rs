use crate::models::{app::AppTemplateDefinition, widget::WidgetDefinition};
use uuid::Uuid;

pub fn instantiate_template_definition(
    definition: &AppTemplateDefinition,
) -> AppTemplateDefinition {
    let mut pages = definition.pages.clone();
    let mut home_page_id = definition.settings.home_page_id.clone();

    for page in &mut pages {
        let previous_id = page.id.clone();
        page.id = Uuid::now_v7().to_string();
        if home_page_id.as_ref() == Some(&previous_id) {
            home_page_id = Some(page.id.clone());
        }
        assign_widget_ids(&mut page.widgets);
    }

    let mut settings = definition.settings.clone();
    settings.home_page_id = home_page_id;

    AppTemplateDefinition {
        pages,
        theme: definition.theme.clone(),
        settings,
    }
}

fn assign_widget_ids(widgets: &mut [WidgetDefinition]) {
    for widget in widgets {
        widget.id = Uuid::now_v7().to_string();
        assign_widget_ids(&mut widget.children);
    }
}
