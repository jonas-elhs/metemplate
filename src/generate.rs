use crate::config::{Config, Template, TemplateMode, ValuesData};
use anyhow::{Context, Result, anyhow};
use rand::seq::IteratorRandom;
use regex::Regex;
use std::fs;
use std::path::Path;

pub fn generate(
    project_name: String,
    values_name: Option<String>,
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
    let values_name = match values_name {
        Some(name) => name,
        None => project
            .values
            .keys()
            .choose(&mut rand::rng())
            .unwrap()
            .clone(),
    };
    // Retrieve values
    let values = match project.values.get(&values_name) {
        Some(values) => values,
        None => {
            return Err(anyhow!(
                "No values named '{}' found in project '{}'",
                values_name,
                project_name,
            ));
        }
    };

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
    let template_regex = Regex::new(r"\{\{([^\\\n]+)\}\}").unwrap();
    for template in templates {
        generate_template(&template_regex, template, values, &values_name)?;

        println!("Generated template '{}'", &template.name);
    }

    Ok(())
}

fn generate_template(
    regex: &Regex,
    template: &Template,
    values: &ValuesData,
    values_name: &str,
) -> Result<()> {
    // Fill out template
    let mut missing_keys: Vec<String> = Vec::new();
    let result = regex
        .replace_all(&template.contents, |captures: &regex::Captures| {
            let key = captures[1].trim();

            if key.split_whitespace().count() != 1 {
                return format!("{{{{{}}}}}", &captures[1]);
            }

            let trimmed = key.trim_start_matches("-");
            let dash_count = key.len() - trimmed.len();

            match values.get(trimmed) {
                Some(value) => remove_prefix(value, dash_count).to_string(),
                None => {
                    missing_keys.push(key.into());

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
            TemplateMode::Replace => &result,
            TemplateMode::Append => &format!(
                "{}{}",
                clean_template(path, &result, &template.name)?,
                result
            ),
            TemplateMode::Prepend => &format!(
                "{}{}",
                result,
                clean_template(path, &result, &template.name)?
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

fn remove_prefix(string: &str, amount: usize) -> &str {
    let byte_index = string
        .char_indices()
        .nth(amount)
        .map(|(index, _)| index)
        .unwrap_or(string.len());

    &string[byte_index..]
}
