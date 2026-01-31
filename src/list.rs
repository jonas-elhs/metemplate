use crate::config::{Config, Projects};
use anyhow::{Result, anyhow};

pub fn list(project_name: Option<String>, config: &Config) -> Result<()> {
    let projects: Projects = config
        .projects
        .iter()
        .filter(|(name, _)| {
            project_name
                .as_ref()
                .is_none_or(|project_name| project_name == *name)
        })
        .map(|(name, project)| (name.clone(), project.clone()))
        .collect();

    if projects.is_empty() {
        return Err(anyhow!(if let Some(project_name) = project_name {
            format!("No project named '{}' found!", project_name)
        } else {
            "No projects found".into()
        }));
    }

    for (index, (project_name, project)) in projects.iter().enumerate() {
        if index > 0 {
            println!();
        }

        // Print project name
        println!("{}", project_name);

        // Print values
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

    Ok(())
}
