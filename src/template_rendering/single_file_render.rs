use tera::{Context, Tera, Result};
use crate::template_config::TemplateConfig;

// Take a template configuration for a single file and render it out

pub fn render_single_file(tera: &Tera, config: &TemplateConfig, template_name: &str, context: Option<&Context>) -> Result<String> {
    let mut final_context = config.get_render_context();
    if let Some(override_context) = context {
        final_context.extend(override_context.clone());
    }

    tera.render(template_name, &final_context)
}
