use crate::config::Config;
use anyhow::{Result, anyhow};

pub fn list(project_name: Option<&str>, no_values: bool, config: &Config) -> Result<()> {
    let projects = config.projects.iter().filter(|(name, _)| {
        project_name
            .as_ref()
            .is_none_or(|project_name| project_name == *name)
    });

    let mut found = false;
    for (index, (project_name, project)) in projects.enumerate() {
        found = true;

        if !no_values && index > 0 {
            println!();
        }

        // Print project name
        println!("{}", project_name);

        if !no_values {
            // Print values in a tree shape
            let mut values_names_iter = project.values.keys().peekable();

            while let Some(name) = values_names_iter.next() {
                let prefix = if values_names_iter.peek().is_some() {
                    "├"
                } else {
                    "└"
                };

                println!("  {}─ {}", prefix, name);
            }
        }
    }

    if !found {
        return match project_name {
            Some(ref name) => Err(anyhow!("No project named '{}' found", name)),
            None => Err(anyhow!("No projects found")),
        };
    }

    Ok(())
}
