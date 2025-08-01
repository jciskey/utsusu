use std::io;
use std::io::Write;
use crate::template_config::TemplateConfig;

/// Prompts the user for input, then returns their input, with trailing whitespace (including
/// newlines) removed.
///
/// Returns:
/// - None if the user provided only whitespace as input (including just pressing <Return>).
/// - Some otherwise.
pub fn get_user_input(prompt: &str) -> Option<String> {
    let mut input = String::new();
    print!("{}", prompt);
    let _ = io::stdout().flush();  // Ensure the message is displayed to the user before requesting input
    let _ = io::stdin().read_line(&mut input);
    let trimmed_input = input.trim();
    if trimmed_input == "" {
        None
    } else {
        Some(trimmed_input.to_string())
    }
}

/// Iterates through the variables defined in the template and prompts the user for values for each
/// of them.
///
/// Returns a Tera Context with the values that were explicitly overridden by the user. Values left
/// as the default are not included in the context.
pub fn get_user_variable_choices(config: &TemplateConfig) -> tera::Context {
    let mut user_variables_context: tera::Context = tera::Context::new();

    for (var_name, default_var_value) in config.get_variable_items() {
        let prompt = format!("{} [{}]: ", var_name, default_var_value);
        if let Some(trimmed_input) = get_user_input(&prompt) {
            user_variables_context.insert(var_name, &trimmed_input);
        };
    }

    user_variables_context
}
