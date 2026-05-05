use std::collections::HashMap;

use shared::QueueField;

pub fn normalize_fields(fields: Vec<QueueField>) -> Result<Vec<QueueField>, String> {
    let mut normalized = Vec::new();
    let mut seen = HashMap::new();

    for field in fields {
        let label = field.label.trim().to_string();
        if label.is_empty() {
            return Err("field labels cannot be empty".to_string());
        }

        let key = if field.key.trim().is_empty() {
            slugify(&label)
        } else {
            slugify(field.key.trim())
        };

        if key.is_empty() {
            return Err(format!("field label '{}' produced an empty key", label));
        }

        if seen.insert(key.clone(), true).is_some() {
            return Err(format!("duplicate field key '{}'", key));
        }

        normalized.push(QueueField {
            key,
            label,
            required: field.required,
        });
    }

    Ok(normalized)
}

pub fn normalize_email(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() || !normalized.contains('@') {
        return Err("a valid email is required".to_string());
    }
    Ok(normalized)
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .replace("__", "_")
}
