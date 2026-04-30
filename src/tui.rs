use std::collections::HashMap;

use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::context::Param;

pub fn prompt(parameters: &[Param]) -> anyhow::Result<(String, HashMap<String, String>)> {
    let theme = ColorfulTheme::default();

    let name: String = Input::with_theme(&theme)
        .with_prompt("Name")
        .interact_text()?;

    let mut params = HashMap::new();
    for param in parameters {
        match param.kind.as_str() {
            "string" => {
                let value: String = Input::with_theme(&theme)
                    .with_prompt(&param.name)
                    .allow_empty(true)
                    .interact_text()?;
                if !value.is_empty() {
                    params.insert(param.name.clone(), value);
                }
            }
            "enum" => {
                if let Some(values) = &param.values {
                    let idx = Select::with_theme(&theme)
                        .with_prompt(&param.name)
                        .items(values)
                        .default(0)
                        .interact()?;
                    params.insert(param.name.clone(), values[idx].clone());
                }
            }
            _ => {}
        }
    }

    Ok((name, params))
}
