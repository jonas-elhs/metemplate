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
            return Err(anyhow!("No project named '{}' found!", project_name));
        }
    };

    // If 'values_name' is not passed, choose a random one
    let values_name = values_name.unwrap_or_else(|| {
        project
            .values
            .keys()
            .choose(&mut rand::rng())
            .unwrap()
            .clone()
    });
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

    // Generate all templates
    for template in templates {
        generate_template(template, values, &values_name)?;
        println!("Generated template '{}'", &template.name);
    }

    Ok(())
}

fn generate_template(template: &Template, values: &ValuesData, values_name: &String) -> Result<()> {
    let regex = Regex::new(r"\{\{([^\{\\\s]+)\}\}").unwrap();

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
    if !missing_keys.is_empty() {
        return Err(anyhow!(
            "Could not find {} '{}' in values '{}'!",
            if missing_keys.len() == 1 {
                "key"
            } else {
                "keys"
            },
            missing_keys.join("', '"),
            values_name
        ));
    }

    // Write template
    fs::write(&template.out, result).with_context(|| {
        format!(
            "Could not write template '{}' to '{}'!",
            template.name,
            template.out.display()
        )
    })?;

    Ok(())
}
