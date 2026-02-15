use crate::config::{Config, Template, TemplateMode, Values};
use anyhow::{Context, Result, anyhow};
use rand::seq::IteratorRandom;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static TEMPLATE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*([^\s]+)\s*\}\}").unwrap());
static REPEAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^<\{\s*repeat\s+([^\s]+)\s*\}>$").unwrap());
static ENDREPEAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^<\{\s*endrepeat\s*\}>$").unwrap());

pub fn generate(
    project_name: &str,
    values_name: Option<&str>,
    value_overrides: &[(String, String)],
    random_values: bool,
    template_name: Option<&str>,
    config: &Config,
) -> Result<()> {
    // Retrieve project
    let project = match config.projects.get(project_name) {
        Some(project) => project,
        None => {
            return Err(anyhow!("No project named '{}' found", project_name));
        }
    };

    // If 'values_name' is not passed, choose a random one
    let random_choice: &String;
    let values_name = if let Some(name) = values_name {
        name
    } else if random_values {
        random_choice = project
            .values
            .keys()
            .choose(&mut rand::rng())
            .ok_or_else(|| anyhow!("Project '{}' has no values", project_name))?;

        random_choice.as_str()
    } else {
        ""
    };

    // Retrieve values
    let mut values = if values_name.is_empty() {
        if value_overrides.is_empty() {
            return Err(anyhow!(
                "Either a values name (--values) or the random flag (--random) has to be passed"
            ));
        }

        Values {
            data: HashMap::new(),
            vars: HashMap::new(),
        }
    } else {
        project.values.get(values_name).cloned().ok_or_else(|| {
            anyhow!(
                "No values named '{}' found in project '{}'",
                values_name,
                project_name,
            )
        })?
    };

    // Override values
    for (value_name, value) in value_overrides {
        values.data.insert(value_name.clone(), value.clone());
    }

    // Either take passed template or all
    let templates: Vec<_> = project
        .templates
        .iter()
        .filter(|template| template_name.is_none_or(|name| name == template.name))
        .collect();

    if templates.is_empty() {
        return match template_name {
            Some(ref name) => Err(anyhow!(
                "No template named '{}' found in project '{}'",
                name,
                project_name
            )),
            None => Err(anyhow!("No templates found")),
        };
    }

    // Generate all templates
    for template in templates {
        generate_template(template, &values, values_name)?;

        println!("Generated template '{}'", &template.name);
    }

    Ok(())
}

fn generate_template(template: &Template, values: &Values, values_name: &str) -> Result<()> {
    // Expand 'repeat' statements
    let mut repeated_template = template.contents.clone();
    while REPEAT_REGEX.is_match(&repeated_template) {
        repeated_template = expand_repeat_statement(&repeated_template, values, &template.name)?;
    }

    // Fill template
    let filled = fill_template(&repeated_template, &values.data, values_name)?;

    // Write template
    for path in &template.out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create output directory for template '{}' at '{}'",
                    &template.name,
                    parent.display(),
                )
            })?;
        }

        let contents = match template.mode {
            TemplateMode::Replace => &filled,
            TemplateMode::Append => &format!(
                "{}{}",
                clean_template(path, &filled, &template.name)?,
                filled
            ),
            TemplateMode::Prepend => &format!(
                "{}{}",
                filled,
                clean_template(path, &filled, &template.name)?
            ),
        };

        write_template(&template.name, path, contents)?;
    }

    Ok(())
}

fn write_template(template_name: &str, path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| {
        format!(
            "Failed to write template '{}' to '{}'",
            template_name,
            path.display()
        )
    })
}

fn clean_template(path: &Path, template: &str, template_name: &str) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }

    let file_contents = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read existing template out file: {}",
            path.display()
        )
    })?;

    // Retrieve template start and end markers
    let mut template_lines = template.lines();
    let template_start_line = template_lines
        .next()
        .with_context(|| format!("Template can not be empty: {}", template_name))?;
    let template_end_line = template_lines
        .last()
        .with_context(|| format!("Template has only one line: {}", template_name))?;

    // Remove already generated template
    let mut result = String::new();
    let mut skipping = false;

    for line in file_contents.lines() {
        if !skipping && line == template_start_line {
            skipping = true;
            continue;
        }

        if skipping && line == template_end_line {
            skipping = false;
            continue;
        }

        if !skipping {
            result.push_str(line);
            result.push('\n');
        }
    }

    Ok(result)
}

fn expand_repeat_statement(template: &str, values: &Values, template_name: &str) -> Result<String> {
    let lines: Vec<&str> = template.lines().collect();

    for (start_index, line) in lines.iter().enumerate() {
        if let Some(captures) = REPEAT_REGEX.captures(line) {
            let values_pool = match &captures[1] {
                "values" => &values.data,
                "vars" => &values.vars,
                capture => {
                    return Err(anyhow!(
                        "Can only repeat over 'values' or 'vars': Got '{}'",
                        capture
                    ));
                }
            };

            // Find 'endrepeat' statement
            let end_index = lines[start_index + 1..]
                .iter()
                .position(|line2| ENDREPEAT_REGEX.is_match(line2))
                .ok_or_else(|| anyhow!(
                    "No endrepeat statement found after repeat statement in line '{}' in template '{}'",
                    start_index + 1,
                    template_name
                ))? + start_index + 1;

            // Check nested 'repeat' statement
            if lines[start_index + 1..end_index]
                .iter()
                .any(|line2| REPEAT_REGEX.is_match(line2))
            {
                let nested_index = lines[start_index + 1..end_index]
                    .iter()
                    .position(|line2| REPEAT_REGEX.is_match(line2))
                    .unwrap();

                return Err(anyhow!(
                    "Nested repeat statements not allowed. First in line '{}', second in line '{}'",
                    start_index + 1,
                    start_index + 1 + nested_index + 1
                ));
            }

            let repeat_content = lines[start_index + 1..end_index].join("\n");

            let mut insert_lines = String::new();
            for (value_key, value_value) in values_pool {
                let mut repeat_values = HashMap::with_capacity(2);
                repeat_values.insert("key".to_string(), value_key.clone());
                repeat_values.insert("value".to_string(), value_value.clone());

                let filled = fill_template(&repeat_content, &repeat_values, "key,value")?;
                insert_lines.push_str(&filled);
                insert_lines.push('\n');
            }

            // Construct final template
            let mut result = String::new();

            // Lines before 'repeat' statement
            result.push_str(&lines[..start_index].join("\n"));

            // Repeated content
            if !insert_lines.is_empty() {
                result.push('\n');
                result.push_str(insert_lines.trim_end());
            }

            // Lines after 'repeat' statement
            if end_index + 1 < lines.len() {
                result.push('\n');
                result.push_str(&lines[end_index + 1..].join("\n"));
            }

            return Ok(result);
        }
    }

    Ok(template.to_string())
}

fn fill_template(
    template: &str,
    value_pool: &HashMap<String, String>,
    values_name: &str,
) -> Result<String> {
    // Fill out template
    let mut missing_keys: Vec<String> = Vec::new();
    let result = TEMPLATE_REGEX
        .replace_all(template, |captures: &regex::Captures| {
            let key = &captures[1];

            if key.split_whitespace().count() != 1 {
                // '{{' in a format string equals '{' so this returns '{{THE CAPTURE}}' essentially doing nothing
                return format!("{{{{{}}}}}", &captures[1]);
            }

            let trimmed = key.trim_start_matches("-");
            let dash_count = key.len() - trimmed.len();

            match value_pool.get(trimmed) {
                Some(value) => remove_prefix(value, dash_count).to_string(),
                None => {
                    missing_keys.push(trimmed.to_string());

                    String::new()
                }
            }
        })
        .to_string();

    // Report missing keys
    if !missing_keys.is_empty() {
        return Err(anyhow!(
            "Could not find keys in values '{}': {}",
            values_name,
            missing_keys.join(", "),
        ));
    }

    Ok(result)
}

fn remove_prefix(string: &str, amount: usize) -> &str {
    let byte_index = string
        .char_indices()
        .nth(amount)
        .map(|(index, _)| index)
        .unwrap_or(string.len());

    &string[byte_index..]
}
