use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::app;
use crate::config;
use crate::registry;
use crate::scanner;
use crate::service::{
    normalize_memo, normalize_scheme, normalize_source, normalize_tags, normalize_title,
    normalize_url_path, LocalService, ServiceMetadata,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ServiceView {
    pub id: String,
    pub pid: u32,
    pub port: u16,
    pub address: String,
    pub process_name: String,
    pub command: String,
    pub kind: Option<&'static str>,
    pub title: Option<String>,
    pub memo: Option<String>,
    pub tags: Vec<String>,
    pub url_path: Option<String>,
    pub scheme: String,
    pub source: Option<String>,
    pub url: String,
}

#[derive(Debug, Default)]
pub(crate) struct MetadataPatch {
    pub port: u16,
    pub pid: Option<u32>,
    pub title: Option<String>,
    pub memo: Option<String>,
    pub tags: Option<Vec<String>>,
    pub url_path: Option<String>,
    pub scheme: Option<String>,
    pub source: Option<String>,
}

impl ServiceView {
    fn from_service(service: &LocalService) -> Self {
        Self {
            id: service.metadata_key(),
            pid: service.pid,
            port: service.port,
            address: service.address.clone(),
            process_name: service.process_name.clone(),
            command: service.command.clone(),
            kind: service.kind.label(),
            title: service.metadata.title.clone(),
            memo: service.metadata.memo.clone(),
            tags: service.metadata.tags.clone(),
            url_path: service.metadata.url_path.clone(),
            scheme: service
                .metadata
                .scheme
                .clone()
                .unwrap_or_else(|| "http".to_string()),
            source: service.metadata.source.clone(),
            url: service.open_url(),
        }
    }
}

pub(crate) fn is_cli_command(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("list" | "label" | "open" | "clear" | "mcp" | "help" | "--help" | "-h")
    )
}

pub(crate) async fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("list") => run_list(&args[1..]).await,
        Some("label") => run_label(&args[1..]).await,
        Some("open") => run_open(&args[1..]).await,
        Some("clear") => run_clear(&args[1..]).await,
        Some("mcp") => crate::mcp::run_stdio().await,
        Some("help" | "--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some(command) => Err(anyhow!("unknown command: {command}")),
        None => Ok(()),
    }
}

pub(crate) async fn list_service_views() -> Result<Vec<ServiceView>> {
    let mut data = config::load_data()?;
    let services = load_services(&mut data, false).await?;
    Ok(services.iter().map(ServiceView::from_service).collect())
}

pub(crate) async fn set_service_metadata(patch: MetadataPatch) -> Result<ServiceView> {
    let mut data = config::load_data()?;
    let mut services = load_services(&mut data, false).await?;
    let index = find_service_index(&services, patch.port, patch.pid)?;
    let mut service = services.remove(index);
    let key = service.metadata_key();
    let mut metadata = data.metadata.get(&key).cloned().unwrap_or_default();

    apply_patch_to_metadata(&mut metadata, patch)?;
    metadata.updated_at_unix = Some(registry::now_unix());

    if metadata.is_empty() {
        data.metadata.remove(&key);
    } else {
        data.metadata.insert(key, metadata.clone());
    }

    config::save_data(&data)?;
    service.metadata = metadata;
    Ok(ServiceView::from_service(&service))
}

pub(crate) async fn clear_service_metadata(port: u16, pid: Option<u32>) -> Result<ServiceView> {
    let mut data = config::load_data()?;
    let mut services = load_services(&mut data, false).await?;
    let index = find_service_index(&services, port, pid)?;
    let mut service = services.remove(index);

    registry::clear_metadata(&mut data, &service);
    config::save_data(&data)?;
    service.metadata = ServiceMetadata::default();
    Ok(ServiceView::from_service(&service))
}

pub(crate) async fn open_service(port: u16, pid: Option<u32>) -> Result<ServiceView> {
    let mut data = config::load_data()?;
    let services = load_services(&mut data, false).await?;
    let service = services
        .get(find_service_index(&services, port, pid)?)
        .ok_or_else(|| anyhow!("service disappeared"))?;
    let view = ServiceView::from_service(service);
    app::open_url(&view.url)?;
    Ok(view)
}

async fn run_list(args: &[String]) -> Result<()> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            _ => return Err(anyhow!("unknown list option: {arg}")),
        }
    }

    let services = list_service_views().await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&services)?);
    } else if services.is_empty() {
        println!("No localhost services on ports 1024-9999");
    } else {
        for service in services {
            let label = service
                .title
                .as_deref()
                .unwrap_or(service.process_name.as_str());
            let tags = if service.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", service.tags.join(","))
            };
            println!("{}  {}{}  {}", service.port, label, tags, service.url);
        }
    }
    Ok(())
}

async fn run_label(args: &[String]) -> Result<()> {
    let patch = parse_metadata_patch(args)?;
    let view = set_service_metadata(patch).await?;
    let label = view.title.as_deref().unwrap_or(view.process_name.as_str());
    println!("labeled {} as {}", view.port, label);
    Ok(())
}

async fn run_open(args: &[String]) -> Result<()> {
    let (port, pid) = parse_port_pid(args)?;
    let view = open_service(port, pid).await?;
    println!("opened {}", view.url);
    Ok(())
}

async fn run_clear(args: &[String]) -> Result<()> {
    let (port, pid) = parse_port_pid(args)?;
    let view = clear_service_metadata(port, pid).await?;
    println!("cleared metadata for {}", view.port);
    Ok(())
}

fn parse_metadata_patch(args: &[String]) -> Result<MetadataPatch> {
    let mut patch = MetadataPatch::default();
    let mut tag_values = Vec::new();
    let mut idx = 0;

    while idx < args.len() {
        match args[idx].as_str() {
            "--port" => {
                idx += 1;
                patch.port = parse_required(args, idx, "--port")?.parse()?;
            }
            "--pid" => {
                idx += 1;
                patch.pid = Some(parse_required(args, idx, "--pid")?.parse()?);
            }
            "--title" => {
                idx += 1;
                patch.title = Some(parse_required(args, idx, "--title")?.to_string());
            }
            "--memo" => {
                idx += 1;
                patch.memo = Some(parse_required(args, idx, "--memo")?.to_string());
            }
            "--tag" => {
                idx += 1;
                tag_values.push(parse_required(args, idx, "--tag")?.to_string());
            }
            "--tags" => {
                idx += 1;
                tag_values.push(parse_required(args, idx, "--tags")?.to_string());
            }
            "--url" | "--path" => {
                idx += 1;
                patch.url_path = Some(parse_required(args, idx, "--url")?.to_string());
            }
            "--scheme" => {
                idx += 1;
                patch.scheme = Some(parse_required(args, idx, "--scheme")?.to_string());
            }
            "--source" => {
                idx += 1;
                patch.source = Some(parse_required(args, idx, "--source")?.to_string());
            }
            value => return Err(anyhow!("unknown label option: {value}")),
        }
        idx += 1;
    }

    if patch.port == 0 {
        return Err(anyhow!("label requires --port"));
    }
    if !tag_values.is_empty() {
        patch.tags = Some(tag_values);
    }

    Ok(patch)
}

fn apply_patch_to_metadata(metadata: &mut ServiceMetadata, patch: MetadataPatch) -> Result<()> {
    let mut changed = false;

    if let Some(title) = patch.title {
        metadata.title = normalize_title(&title);
        changed = true;
    }
    if let Some(memo) = patch.memo {
        metadata.memo = normalize_memo(&memo);
        changed = true;
    }
    if let Some(tags) = patch.tags {
        metadata.tags = normalize_tags(&tags);
        changed = true;
    }
    if let Some(url_path) = patch.url_path {
        metadata.url_path = normalize_url_path(&url_path);
        changed = true;
    }
    if let Some(scheme) = patch.scheme {
        metadata.scheme =
            Some(normalize_scheme(&scheme).ok_or_else(|| anyhow!("scheme must be http or https"))?);
        changed = true;
    }
    if let Some(source) = patch.source {
        metadata.source = normalize_source(&source);
        changed = true;
    }

    if !changed {
        return Err(anyhow!("label requires at least one metadata field"));
    }

    Ok(())
}

fn parse_port_pid(args: &[String]) -> Result<(u16, Option<u32>)> {
    let mut port = None;
    let mut pid = None;
    let mut idx = 0;

    while idx < args.len() {
        match args[idx].as_str() {
            "--port" => {
                idx += 1;
                port = Some(parse_required(args, idx, "--port")?.parse()?);
            }
            "--pid" => {
                idx += 1;
                pid = Some(parse_required(args, idx, "--pid")?.parse()?);
            }
            value if !value.starts_with('-') && port.is_none() => {
                port = Some(value.parse()?);
            }
            value => return Err(anyhow!("unknown option: {value}")),
        }
        idx += 1;
    }

    Ok((port.ok_or_else(|| anyhow!("port is required"))?, pid))
}

fn parse_required<'a>(args: &'a [String], idx: usize, flag: &str) -> Result<&'a str> {
    args.get(idx)
        .map(String::as_str)
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}

fn find_service_index(services: &[LocalService], port: u16, pid: Option<u32>) -> Result<usize> {
    let matches = services
        .iter()
        .enumerate()
        .filter(|(_, service)| service.port == port && pid.is_none_or(|pid| service.pid == pid))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(anyhow!("no live localhost service on port {port}")),
        [index] => Ok(*index),
        _ => Err(anyhow!("multiple services match port {port}; add --pid")),
    }
}

async fn load_services(data: &mut config::AppData, prune_stale: bool) -> Result<Vec<LocalService>> {
    let mut services = scanner::scan_services().await?;
    if registry::apply_metadata(data, &mut services, prune_stale) && prune_stale {
        config::save_data(data)?;
    }
    Ok(services)
}

fn print_help() {
    println!(
        "hostel\n\
         \n\
         TUI:    hostel\n\
         List:   hostel list [--json]\n\
         Label:  hostel label --port 5173 --title \"Frontend\" --memo \"Vite app\" --tag vite\n\
         Open:   hostel open 5173\n\
         Clear:  hostel clear 5173\n\
         MCP:    hostel mcp"
    );
}
