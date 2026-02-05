use crate::cli::Cli;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

// Parsed files
#[derive(Debug, Deserialize)]
struct TemplateConfig {
    #[serde(deserialize_with = "single_or_vec")]
    out: Vec<PathBuf>,
    file: PathBuf,
}
#[derive(Debug, Deserialize)]
struct ProjectConfig {
    values: Option<Vec<String>>,
    templates: HashMap<String, TemplateConfig>,
}
#[derive(Debug, Deserialize)]
struct ValuesFile {
    #[serde(flatten)]
    values: HashMap<String, String>,
    #[serde(default)]
    vars: HashMap<String, String>,
}

// Runtime representation
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub contents: String,
    pub out: Vec<PathBuf>,
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
}

impl Config {
    pub fn parse(cli: &Cli) -> Result<Self> {
        let config_directory = cli
            .config
            .clone()
            .or_else(|| dirs::config_dir().map(|dir| dir.join("metemplate")))
            .ok_or_else(|| anyhow!("Could not find config directory"))?;

        Ok(Self {
            projects: fs::read_dir(&config_directory)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .map(|entry| load_project(&entry.path()))
                .collect::<Result<_>>()?,
        })
    }
}

fn load_project(path: &Path) -> Result<(String, Project)> {
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
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let mut templates: Vec<Template> = config
        .templates
        .into_iter()
        .map(|(name, template_config)| {
            let template_path = templates_path.join(template_config.file);

            // Expand home directory
            let out: Vec<PathBuf> = template_config
                .out
                .into_iter()
                .map(|path| {
                    if path.starts_with("~") {
                        home_dir.join(path.strip_prefix("~").unwrap())
                    } else {
                        path
                    }
                })
                .collect();

            Ok(Template {
                out,
                name,
                contents: fs::read_to_string(&template_path).with_context(|| {
                    format!(
                        "Failed to read template file at path '{}'",
                        template_path.display()
                    )
                })?,
            })
        })
        .collect::<Result<_>>()?;
    templates.sort_by(|a, b| a.name.cmp(&b.name));

    // Values
    let values_path = path.join("values");
    let values: Values = fs::read_dir(&values_path)
        .with_context(|| {
            format!(
                "Failed to read values directory at path '{}'",
                values_path.display(),
            )
        })?
        .map(|entry| load_values(&entry?.path(), &config.values))
        .collect::<Result<_>>()?;

    Ok((project_name, Project { templates, values }))
}

fn load_values(path: &Path, values: &Option<Vec<String>>) -> Result<(String, ValuesData)> {
    let values_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let values_file: ValuesFile = read_toml(path)
        .with_context(|| format!("Failed to read values file at path '{}'", path.display()))?;

    // Resolve values from vars section
    let data: ValuesData = values_file
        .values
        .into_iter()
        .map(|(key, value)| {
            // If the value starts with a '$' it is a var
            let resolved_value = if let Some(var) = value.strip_prefix("$") {
                values_file
                    .vars
                    .get(var)
                    .ok_or_else(|| {
                        anyhow!(
                            "Data '{}' not defined in values file at path '{}'",
                            var,
                            path.display()
                        )
                    })?
                    .clone()
            // If the value starts with a '\$' it should be a literal dollar sign
            } else if let Some(rest) = value.strip_prefix("\\$") {
                format!("${}", rest)
            // Otherwise it is a literal value
            } else {
                value
            };

            Ok((key, resolved_value))
        })
        .collect::<Result<_>>()?;

    // Validate values
    if let Some(required_values) = values.as_ref() {
        // Find missing values
        let missing_values: Vec<_> = required_values
            .iter()
            .filter(|key| !data.contains_key(*key))
            .map(|key| key.as_str())
            .collect();

        // Find extra values
        let extra_values: Vec<_> = data
            .keys()
            .filter(|key| !required_values.contains(*key))
            .map(|key| key.as_str())
            .collect();

        if !missing_values.is_empty() || !extra_values.is_empty() {
            let mut msg = String::new();

            if !missing_values.is_empty() {
                msg.push_str(&format!(
                    "Missing keys in values file at path '{}': {}",
                    path.display(),
                    missing_values.join(", ")
                ));
            }
            if !extra_values.is_empty() {
                msg.push_str(&format!(
                    "Unspecified keys in values file at path '{}': {}",
                    path.display(),
                    extra_values.join(", ")
                ));
            }

            return Err(anyhow!(msg.trim().to_string()));
        }
    }

    Ok((values_name, data))
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file '{}'", path.display()))?;
    let parsed = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse TOML in '{}'", path.display()))?;

    Ok(parsed)
}

fn single_or_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        One(T),
        Many(Vec<T>),
    }

    match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(v) => Ok(vec![v]),
        OneOrMany::Many(v) => Ok(v),
    }
}
