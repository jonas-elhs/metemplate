use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    out: PathBuf,
    file: PathBuf,
}
#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    templates: HashMap<String, TemplateConfig>,
}
#[derive(Debug, Deserialize)]
pub struct ValuesFile {
    #[serde(flatten)]
    values: HashMap<String, String>,
    vars: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub contents: String,
    pub out: PathBuf,
}
pub type ValuesData = HashMap<String, String>;
pub type Values = BTreeMap<String, ValuesData>;
#[derive(Debug, Clone)]
pub struct Project {
    pub templates: Vec<Template>,
    pub values: Values,
}
pub type Projects = BTreeMap<String, Project>;
#[derive(Debug)]
pub struct Config {
    pub projects: Projects,
    pub path: PathBuf,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let projects: Projects = fs::read_dir(&path)?
            .map(|entry| load_project(entry?.path()))
            .collect::<Result<_>>()?;

        Ok(Self {
            path: path.as_ref().into(),
            projects,
        })
    }
}

fn load_project(path: PathBuf) -> Result<(String, Project)> {
    // Name
    let project_name = path.file_name().unwrap().to_string_lossy().to_string();

    // Config
    let config_path = path.join("config.toml");
    let config: ProjectConfig = read_toml(&config_path).with_context(|| {
        format!(
            "Failed to read project config file at path '{}'",
            path.display()
        )
    })?;

    // Templates
    let templates_path = path.join("templates");
    let mut templates: Vec<Template> = config
        .templates
        .iter()
        .map(|(name, template_config)| {
            let template_path = templates_path.join(&template_config.file);

            Ok(Template {
                name: name.to_string(),
                out: template_config.out.clone(),
                contents: fs::read_to_string(&template_path).with_context(|| {
                    format!(
                        "Failed to read template file at path '{}'",
                        template_path.display()
                    )
                })?,
            })
        })
        .collect::<Result<_>>()?;
    templates.sort_unstable_by(|a, b| a.name.cmp(&b.name));

    // Values
    let values_path = path.join("values");
    let values: Values = fs::read_dir(&values_path)
        .with_context(|| {
            format!(
                "Failed to read values directory at path '{}'",
                values_path.display(),
            )
        })?
        .map(|entry| load_values(entry?.path()))
        .collect::<Result<_>>()?;

    Ok((project_name, Project { templates, values }))
}

fn load_values(path: PathBuf) -> Result<(String, ValuesData)> {
    let values_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let values_file: ValuesFile = read_toml(&path)
        .with_context(|| format!("Failed to read values file at path '{}'", path.display()))?;

    let data: ValuesData = values_file
        .values
        .into_iter()
        .map(|(key, value)| {
            let resolved_value = values_file
                .vars
                .get(&value)
                .ok_or_else(|| {
                    anyhow!(
                        "Data '{}' not defined in values file at path '{}'",
                        value,
                        path.display()
                    )
                })?
                .clone();

            Ok((key, resolved_value))
        })
        .collect::<Result<ValuesData>>()?;

    Ok((values_name, data))
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let contents = fs::read_to_string(path)?;
    let parsed = toml::from_str(&contents)?;

    Ok(parsed)
}
