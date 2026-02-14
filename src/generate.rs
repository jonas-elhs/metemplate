use crate::config::{Config, Template, TemplateMode, ValuesData};
use anyhow::{Context, Result, anyhow};
use rand::seq::IteratorRandom;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

pub fn generate(
    project_name: String,
    values_name: Option<String>,
    value_overrides: Vec<(String, String)>,
    random_values: bool,
    template_name: Option<String>,
    config: &Config,
) -> Result<()> {
    // Retrieve project
    let project = match config.projects.get(&project_name) {
        Some(project) => project,
        None => {
            return Err(anyhow!("No project named '{}' found", project_name));
        }
    };

    // If 'values_name' is not passed, choose a random one
    if project.values.is_empty() {
        return Err(anyhow!("Project '{}' has no values", project_name));
    }
    let values_name = values_name.unwrap_or_else(|| {
        if random_values {
            project
                .values
                .keys()
                .choose(&mut rand::rng())
                .cloned()
                .unwrap()
        } else {
            String::new()
        }
    });

    // Retrieve values
    let mut values = if values_name.is_empty() {
        if value_overrides.is_empty() {
            return Err(anyhow!(
                "Either a values name (--values) or the random flag (--random) has to be passed"
            ));
        }

        HashMap::new()
    } else {
        project
            .values
            .get(&values_name)
            .ok_or_else(|| {
                anyhow!(
                    "No values named '{}' found in project '{}'",
                    values_name,
                    project_name,
                )
            })?
            .clone()
    };

    // Override values
    for (value_name, value) in value_overrides {
        values.insert(value_name, value);
    }

    // Either take passed template or all
    let templates: Vec<_> = project
        .templates
        .iter()
        .filter(|template| {
            template_name
                .as_ref()
                .is_none_or(|name| name == &template.name)
        })
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
        generate_template(template, &values, &values_name)?;

        println!("Generated template '{}'", &template.name);
    }

    Ok(())
}

static TEMPLATE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*([^\s]+)\s*\}\}").unwrap());
static REPEAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^<\{\s*repeat\s+([^\s]+)\s*\}>$").unwrap());
static ENDREPEAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^<\{\s*endrepeat\s*\}>$").unwrap());

fn generate_template(template: &Template, values: &ValuesData, values_name: &str) -> Result<()> {
    // repeat
    let mut repeated_template = template.contents.clone();
    while REPEAT_REGEX.is_match(&repeated_template) {
        repeated_template = fill_repeat_template(repeated_template, values, &template.name)?;
    }

    // Fill out template
    let filled = fill_template(&repeated_template, values, values_name)?;

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

fn fill_repeat_template(
    template: String,
    values: &ValuesData,
    template_name: &str,
) -> Result<String> {
    for (start_index, line) in template.clone().lines().enumerate() {
        if let Some(captures) = REPEAT_REGEX.captures(line) {
            let mut end_index: usize = 0;
            for (index2, line2) in template.lines().skip(start_index + 1).enumerate() {
                if REPEAT_REGEX.is_match(line2) {
                    return Err(anyhow!(
                        "Repeat statement inside repeat statement not allowed. First in line '{}', second in line '{}'",
                        start_index + 1,
                        start_index + index2 + 2
                    ));
                }

                if ENDREPEAT_REGEX.is_match(line2) {
                    end_index = start_index + index2 + 1;

                    break;
                }
            }

            if end_index == 0 {
                return Err(anyhow!(
                    "No endrepeat statement found after repeat statement in line '{}' in template '{}'",
                    start_index + 1,
                    template_name
                ));
            }

            println!("{} -> {}", start_index, end_index);

            let repeat_lines = template
                .lines()
                .skip(start_index + 1)
                .take(end_index - start_index - 1)
                .collect::<Vec<&str>>()
                .join("\n");

            let insert_lines: Vec<String> = values
                .iter()
                .map(|(value_key, value_value)| {
                    let repeat_values: HashMap<String, String> = [
                        ("key".to_string(), value_key.clone()),
                        ("value".to_string(), value_value.clone()),
                    ]
                    .into();

                    fill_template(&repeat_lines, &repeat_values, "key,value")
                })
                .collect::<Result<Vec<_>>>()?;

            return Ok(template
                .lines()
                .take(start_index)
                .map(str::to_owned)
                .chain(insert_lines)
                .chain(template.lines().skip(end_index + 1).map(str::to_owned))
                .collect::<Vec<_>>()
                .join("\n"));
        }
    }

    Ok(template)
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
                    missing_keys.push(trimmed.into());

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
