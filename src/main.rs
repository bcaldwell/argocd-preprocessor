mod app_project;
mod processor;
use anyhow::Result;
use clap::Parser;
use processor::ProjectProcessor;
use tracing::error;
use tracing_subscriber;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long)]
    input_path: Option<String>,
    #[arg(short, long)]
    output_path: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Config {
    application_template: String,
    argocd_namespace: String,
    argocd_source_repo: String,
    targets: Vec<ConfigTarget>,
    vars: Option<serde_json::Value>,
    default_application_options: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ConfigTarget {
    name: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct Metadata {
    namespace: Option<String>,
    script: Option<String>,
    application_options: Option<serde_json::Value>,
    targets: Vec<MetadataTarget>,
    project_options: Option<MetadataProjectOptions>,
}

#[derive(serde::Deserialize, Debug)]
struct MetadataTarget {
    name: String,
    vars: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct MetadataProjectOptions {
    additional_namespaces: Option<Vec<String>>,
    cluster_resource_whitelist: Option<Vec<crate::app_project::AppProjectClusterResourceWhitelist>>,
}

#[derive(serde::Serialize, Debug)]
struct TemplateContext {
    namespace: String,
    project: String,
    app_name: String,
    normalized_project: String,
    normalized_app_name: String,
    path: String,
    target_name: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let run_result = run();

    return match run_result {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    };
}

fn run() -> Result<()> {
    let args = Args::parse();
    let mut project_processor = ProjectProcessor::new(args)?;
    return project_processor.process();
}

