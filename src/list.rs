use crate::config::{Config, Projects};

pub fn list(project_name: Option<String>, config: &Config) {
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
        if let Some(project_name) = project_name {
            println!("No project named '{}' found", project_name);
        } else {
            println!("No projects found");
        }

        return;
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
}
