use crate::config::{Config, Template, ValuesData};
use anyhow::{Context, Result, anyhow};
use rand::seq::IteratorRandom;
use regex::Regex;
use std::fs;

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
    let template_regex = Regex::new(r"\{\{([^\{\\\s]+)\}\}").unwrap();
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
            let key = &captures[1];

            match values.get(key) {
                Some(value) => value,
                None => {
                    missing_keys.push(key.into());
                    ""
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
        fs::write(path, &result).with_context(|| {
            format!(
                "Failed to write template '{}' to '{}'",
                template.name,
                path.display()
            )
        })?;
    }

    Ok(())
}
