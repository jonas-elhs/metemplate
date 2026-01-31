use crate::config::Config;
use crate::config::Project;

pub fn list(project_name: Option<String>, config: &Config) {
    let projects: Vec<&Project> = if let Some(name) = project_name.as_ref() {
        config
            .projects
            .iter()
            .filter(|project| project.name == *name)
            .collect()
    } else {
        config.projects.iter().collect()
    };

    if projects.is_empty() {
        if let Some(project_name) = project_name {
            println!("No project named '{}' found", project_name);
        } else {
            println!("No projects found");
        }

        return;
    }

    for (index, project) in projects.iter().enumerate() {
        if index > 0 {
            println!();
        }

        println!("{}", project.name);

        if let Some((last, rest)) = project.values.split_last() {
            for value in rest {
                println!("  ├─ {}", value.name);
            }

            println!("  └─ {}", last.name);
        }
    }
}
