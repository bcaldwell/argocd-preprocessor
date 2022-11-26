use anyhow::{anyhow, Result};
use clap::Parser;
use std::{
    error::Error,
    fs,
    path::{self, Path},
};
use tera::Tera;
use tracing::{error, info};
use tracing_subscriber;
mod app_project;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input_path: Option<String>,
    #[arg(short, long)]
    output_path: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Config {
    application_template: String,
    targets: Vec<ConfigTarget>,
    vars: Option<serde_json::Value>,
    default_application_options: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ConfigTarget {
    name: String,
}

#[derive(serde::Deserialize, Debug)]
struct Metadata {
    // namespace: Option<String>,
    script: Option<String>,
    application_options: Option<serde_json::Value>,
    targets: Vec<MetadataTarget>,
}

#[derive(serde::Deserialize, Debug)]
struct MetadataTarget {
    name: String,
    vars: Option<serde_json::Value>,
}

#[derive(serde::Serialize, Debug)]
struct TemplateContext {
    namespace: String,
    path: TemplateContextPath,
}

#[derive(serde::Serialize, Debug)]
struct TemplateContextPath {
    path: String,
    basename: String,
    parent: String,
}

fn main() {
    tracing_subscriber::fmt::init();
    match run() {
        Ok(_) => (),
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let input_path = match args.input_path {
        Some(v) => std::path::PathBuf::from(v),
        None => std::path::PathBuf::from("."),
    };

    let output_path = match args.output_path {
        Some(v) => std::path::PathBuf::from(v),
        None => tempdir::TempDir::new("argocd-preprocessor")?
            .path()
            .to_path_buf(),
    };

    fs::create_dir_all(&output_path)?;
    let input_path = input_path.canonicalize()?;
    let output_path = output_path.canonicalize()?;

    info!(input_path=?input_path, output_path=?output_path, "resolved input and output paths");

    let config = read_config(&input_path)?;
    for target in config.targets {
        let target_dir = output_path.join(target.name);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)?;
        }
        fs::create_dir_all(target_dir)?;
    }

    let tera = Tera::new(input_path.join("**/*.tera").to_str().unwrap())?;
    for k in tera.get_template_names() {
        info!(key=?k, "templates")
    }

    let template_path = input_path.join(config.application_template);
    let template_name = template_path.strip_prefix(&input_path);
    let template_name = template_name.unwrap().to_str().unwrap();

    for file in glob::glob(input_path.join("**/metadata.toml").to_str().unwrap())? {
        let file = file.unwrap();
        info!(file = ?file, "processing file");
        let metadata = read_metadata(&file)?;

        let base_template_context = TemplateContext {
            namespace: "namespace".to_string(),
            path: TemplateContextPath {
                parent: file.parent().unwrap().to_str().unwrap().to_string(),
                basename: file.file_name().unwrap().to_str().unwrap().to_string(),
                path: file.to_str().unwrap().to_string(),
            },
        };

        let mut template_context = config
            .default_application_options
            .clone()
            .unwrap_or_else(default_serde_object);
        merge(
            &mut template_context,
            metadata
                .application_options
                .unwrap_or_else(default_serde_object),
        );
        merge(
            &mut template_context,
            serde_json::to_value(base_template_context)?,
        );

        if tera.templates.contains_key(template_name) {
            let render_result = tera.render(
                &template_name,
                &tera::Context::from_value(template_context)?,
            );
            let application = match render_result {
                Err(e) => match e.source() {
                    Some(err_source) => Err(anyhow!("{:#}", err_source)),
                    None => Err(anyhow!("{}", e)),
                },
                Ok(r) => Ok(r),
            }?;
            println!("{}", application);
        }

        for target in metadata.targets {
            let in_folder_path = file.parent().unwrap();
            let rel_path = in_folder_path.strip_prefix(&input_path);
            let out_folder_path = output_path
                .join(target.name)
                .join(rel_path.clone().unwrap());
            info!(from_path=?rel_path, to_path=?out_folder_path, "copying");

            let mut target_vars = config.vars.clone().unwrap_or_else(default_serde_object);
            merge(
                &mut target_vars,
                target.vars.unwrap_or_else(default_serde_object),
            );
            let target_vars = tera::Context::from_value(target_vars)?;

            copy_and_template_folder(
                &tera,
                &target_vars,
                input_path.clone(),
                in_folder_path.to_path_buf().clone(),
                in_folder_path.to_path_buf(),
                out_folder_path.clone(),
            )?;

            match metadata.script.as_ref() {
                Some(script) => {
                    std::process::Command::new("bash")
                        .arg("-c")
                        .arg(script)
                        .env("in", in_folder_path)
                        .env("out", out_folder_path)
                        .output()?;
                }
                None => (),
            };
        }
    }

    Ok(())
}

fn default_serde_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn copy_and_template_folder(
    tera: &tera::Tera,
    tera_context: &tera::Context,
    tera_root: path::PathBuf,
    root_from_dir: path::PathBuf,
    from_dir: path::PathBuf,
    to_dir: path::PathBuf,
) -> Result<()> {
    for f in fs::read_dir(from_dir)? {
        let entry = f?;
        let path = entry.path();
        if path.is_dir() {
            copy_and_template_folder(
                tera,
                tera_context,
                tera_root.clone(),
                root_from_dir.clone(),
                path,
                to_dir.clone(),
            )?;
            continue;
        }
        let rel_to_path = path.strip_prefix(&root_from_dir)?;
        let mut to_path = to_dir.join(rel_to_path);
        fs::create_dir_all(to_path.parent().unwrap())?;
        if path.extension().unwrap() == "tera" {
            let tera_template_name = path.strip_prefix(&tera_root)?;
            let contents = tera.render(tera_template_name.to_str().unwrap(), tera_context);
            let contents = match contents {
                Err(e) => match e.source() {
                    Some(err_source) => Err(anyhow!("{:#}", err_source)),
                    None => Err(anyhow!("{}", e)),
                },
                Ok(r) => Ok(r),
            }?;
            to_path.set_extension("");
            fs::write(to_path, contents)?;
            continue;
        }
        fs::copy(path, to_path)?;
    }
    return Ok(());
}

fn read_metadata(metadata_file: &Path) -> Result<Metadata> {
    let config = fs::read(&metadata_file)
        .map_err(|e| anyhow!("failed to parse config file {:?}: {}", metadata_file, e))?;
    let config = toml::from_slice(&config)
        .map_err(|e| anyhow!("failed to parse config file {:?}: {}", metadata_file, e))?;

    info!(file=?metadata_file, config=?config, "loaded metadata file");
    return Ok(config);
}

fn read_config(input_path: &Path) -> Result<Config> {
    let config_file_path = input_path.join("bargo.toml");
    let config = fs::read(&config_file_path)
        .map_err(|e| anyhow!("failed to parse config file {:?}: {}", config_file_path, e))?;
    let config = toml::from_slice(&config)
        .map_err(|e| anyhow!("failed to parse config file {:?}: {}", config_file_path, e))?;

    info!(config_file_path=?config_file_path, config=?config, "loaded config");
    return Ok(config);
}

// from: https://stackoverflow.com/questions/47070876/how-can-i-merge-two-json-objects-with-rust
fn merge(a: &mut serde_json::Value, b: serde_json::Value) {
    if let serde_json::Value::Object(a) = a {
        if let serde_json::Value::Object(b) = b {
            for (k, v) in b {
                if v.is_null() {
                    a.remove(&k);
                } else {
                    merge(a.entry(k).or_insert(serde_json::Value::Null), v);
                }
            }

            return;
        }
    }

    *a = b;
}
