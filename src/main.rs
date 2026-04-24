use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstArray, CstInputValue, CstRootNode};

const DEFAULT_REGISTRY_GITMODULES_URL: &str =
    "https://raw.githubusercontent.com/LuaLS/LLS-Addons/main/.gitmodules";

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    fs::create_dir_all(&cli.target).map_err(|err| {
        format!(
            "failed to create target directory {}: {err}",
            cli.target.display()
        )
    })?;

    if cli.list {
        let registry = fetch_registry(&cli.gitmodules_url)?;
        for addon in registry.values() {
            if let Some(branch) = &addon.branch {
                println!("{}\t{}\t{}", addon.name, addon.url, branch);
            } else {
                println!("{}\t{}", addon.name, addon.url);
            }
        }
        return Ok(());
    }

    if cli.addons.is_empty() {
        return Err("at least one addon name is required unless --list is used".to_string());
    }

    ensure_git_available()?;

    let registry = fetch_registry(&cli.gitmodules_url)?;

    for addon_name in &cli.addons {
        let addon = registry.get(addon_name).ok_or_else(|| {
            let available = registry.keys().take(10).cloned().collect::<Vec<_>>().join(", ");
            format!(
                "unknown addon '{addon_name}'. Use --list to inspect available addons. Sample entries: {available}"
            )
        })?;

        let destination = cli.target.join(&addon.name);
        install_addon(addon, &destination, cli.force)?;
        println!(
            "installed '{}' from {} to {}",
            addon.name,
            addon.url,
            destination.display()
        );
    }

    if !cli.no_luarc {
        let luarc_path = cli.luarc_path();
        let target = cli.target.canonicalize().map_err(|err| {
            format!(
                "failed to resolve target directory {}: {err}",
                cli.target.display()
            )
        })?;
        update_luarc(&luarc_path, &target)?;
        println!(
            "updated {} with workspace.userThirdParty = {}",
            luarc_path.display(),
            target.display()
        );
    }

    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    name = "lls-addon-install",
    version,
    about = "Install LuaLS addons from the official LLS-Addons registry into a parent directory.",
    after_help = "Behavior:\n  - This tool reads the LLS-Addons registry and resolves addon names to their real Git repositories.\n  - Each installed addon becomes its own directory under --target.\n  - By default it updates .luarc.json and appends --target to workspace.userThirdParty.\n  - Manual install behavior follows https://luals.github.io/wiki/addons/."
)]
struct Cli {
    /// Destination parent directory that contains installed addon folders
    #[arg(short, long, value_name = "DIR")]
    target: PathBuf,

    /// Addon names from the LLS-Addons registry, such as openresty or love2d
    #[arg(value_name = "ADDON", conflicts_with = "list")]
    addons: Vec<String>,

    /// Replace an existing addon directory
    #[arg(short, long)]
    force: bool,

    /// Print available addons from the registry and exit
    #[arg(long)]
    list: bool,

    /// Override the raw .gitmodules URL for the addon registry
    #[arg(long, value_name = "URL", default_value = DEFAULT_REGISTRY_GITMODULES_URL)]
    gitmodules_url: String,

    /// Write LuaLS configuration to this .luarc.json file
    #[arg(long, value_name = "FILE", conflicts_with = "no_luarc")]
    luarc: Option<PathBuf>,

    /// Skip writing .luarc.json
    #[arg(long)]
    no_luarc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AddonRegistryEntry {
    name: String,
    url: String,
    branch: Option<String>,
}

impl Cli {
    fn luarc_path(&self) -> PathBuf {
        self.luarc
            .clone()
            .unwrap_or_else(|| PathBuf::from(".luarc.json"))
    }
}

fn ensure_git_available() -> Result<(), String> {
    let status = Command::new("git")
        .arg("--version")
        .status()
        .map_err(|err| format!("failed to execute git: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("git is required to install addons, but `git --version` failed".to_string())
    }
}

fn fetch_registry(url: &str) -> Result<BTreeMap<String, AddonRegistryEntry>, String> {
    let response = reqwest::blocking::Client::new()
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            concat!("lls-addon-install/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .map_err(|err| format!("failed to fetch registry metadata from {url}: {err}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "failed to fetch registry metadata from {url}: HTTP {}",
            status
        ));
    }

    let body = response
        .text()
        .map_err(|err| format!("failed to read registry metadata from {url}: {err}"))?;

    parse_gitmodules(&body)
}

fn parse_gitmodules(contents: &str) -> Result<BTreeMap<String, AddonRegistryEntry>, String> {
    let mut addons = BTreeMap::new();
    let mut current_name = None::<String>;
    let mut current_path = None::<String>;
    let mut current_url = None::<String>;
    let mut current_branch = None::<String>;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[submodule ") {
            push_current_entry(
                &mut addons,
                current_name.take(),
                current_path.take(),
                current_url.take(),
                current_branch.take(),
            )?;

            current_name = parse_submodule_name(line);
            current_path = None;
            current_url = None;
            current_branch = None;
            continue;
        }

        if let Some(value) = line.strip_prefix("path = ") {
            current_path = Some(value.trim().to_string());
            continue;
        }

        if let Some(value) = line.strip_prefix("url = ") {
            current_url = Some(value.trim().to_string());
            continue;
        }

        if let Some(value) = line.strip_prefix("branch = ") {
            current_branch = Some(value.trim().to_string());
        }
    }

    push_current_entry(
        &mut addons,
        current_name,
        current_path,
        current_url,
        current_branch,
    )?;

    if addons.is_empty() {
        return Err("registry metadata did not contain any addons".to_string());
    }

    Ok(addons)
}

fn parse_submodule_name(line: &str) -> Option<String> {
    let prefix = "[submodule \"";
    let suffix = "\"]";
    let name = line.strip_prefix(prefix)?.strip_suffix(suffix)?;
    let mut segments = name.split('/');
    match (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) {
        (Some("addons"), Some(addon), Some("module"), None) => Some(addon.to_string()),
        _ => None,
    }
}

fn push_current_entry(
    addons: &mut BTreeMap<String, AddonRegistryEntry>,
    name: Option<String>,
    path: Option<String>,
    url: Option<String>,
    branch: Option<String>,
) -> Result<(), String> {
    let Some(name) = name else {
        return Ok(());
    };

    let path = path.ok_or_else(|| format!("missing path for addon '{name}'"))?;
    let url = url.ok_or_else(|| format!("missing url for addon '{name}'"))?;

    let path_name = parse_addon_name_from_path(&path)
        .ok_or_else(|| format!("invalid addon path '{path}' for addon '{name}'"))?;

    if path_name != name {
        return Err(format!(
            "submodule name/path mismatch: header addon '{name}', path addon '{path_name}'"
        ));
    }

    addons.insert(name.clone(), AddonRegistryEntry { name, url, branch });

    Ok(())
}

fn parse_addon_name_from_path(path: &str) -> Option<String> {
    let mut segments = path.split('/');
    match (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) {
        (Some("addons"), Some(addon), Some("module"), None) => Some(addon.to_string()),
        _ => None,
    }
}

fn install_addon(
    addon: &AddonRegistryEntry,
    destination: &Path,
    force: bool,
) -> Result<(), String> {
    if destination.exists() {
        if !force {
            return Err(format!(
                "destination {} already exists, pass --force to replace it",
                destination.display()
            ));
        }
        fs::remove_dir_all(destination).map_err(|err| {
            format!(
                "failed to remove existing addon directory {}: {err}",
                destination.display()
            )
        })?;
    }

    let mut command = Command::new("git");
    command.arg("clone").arg("--depth").arg("1");

    if let Some(branch) = &addon.branch {
        command.arg("--branch").arg(branch);
    }

    let status = command
        .arg(&addon.url)
        .arg(destination)
        .status()
        .map_err(|err| format!("failed to execute git clone for '{}': {err}", addon.name))?;

    if !status.success() {
        return Err(format!(
            "git clone failed for '{}' from {}",
            addon.name, addon.url
        ));
    }

    let git_dir = destination.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir).map_err(|err| {
            format!(
                "addon '{}' was cloned but failed to remove {}: {err}",
                addon.name,
                git_dir.display()
            )
        })?;
    }

    Ok(())
}

fn update_luarc(luarc_path: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = luarc_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    let root = if luarc_path.exists() {
        let contents = fs::read_to_string(luarc_path)
            .map_err(|err| format!("failed to read {}: {err}", luarc_path.display()))?;
        parse_luarc_root(&contents, luarc_path)?
    } else {
        CstRootNode::parse("", &ParseOptions::default()).map_err(|err| {
            format!(
                "failed to create JSONC document for {}: {err}",
                luarc_path.display()
            )
        })?
    };

    let key = if root
        .object_value()
        .and_then(|object| object.get("workspace.userThirdParty"))
        .is_some()
    {
        "workspace.userThirdParty"
    } else if root
        .object_value()
        .and_then(|object| object.get("Lua.workspace.userThirdParty"))
        .is_some()
    {
        "Lua.workspace.userThirdParty"
    } else {
        "workspace.userThirdParty"
    };

    upsert_user_third_party(&root, key, target)?;

    fs::write(luarc_path, root.to_string())
        .map_err(|err| format!("failed to write {}: {err}", luarc_path.display()))?;

    Ok(())
}

fn parse_luarc_root(contents: &str, path: &Path) -> Result<CstRootNode, String> {
    let root = CstRootNode::parse(contents, &ParseOptions::default())
        .map_err(|err| format!("failed to parse {} as JSONC: {err}", path.display()))?;

    if root.value().is_some() && root.object_value().is_none() {
        return Err(format!(
            "{} must contain a JSON object at the top level",
            path.display()
        ));
    }

    Ok(root)
}

fn upsert_user_third_party(root: &CstRootNode, key: &str, target: &Path) -> Result<(), String> {
    let object = root.object_value_or_set();
    let array = object.array_value_or_create(key).ok_or_else(|| {
        format!("existing `{key}` value in .luarc.json must be an array of paths")
    })?;

    let target = target.to_string_lossy().into_owned();
    if !array_contains_string(&array, &target)? {
        array.append(CstInputValue::String(target));
    }

    Ok(())
}

fn array_contains_string(array: &CstArray, expected: &str) -> Result<bool, String> {
    for element in array.elements() {
        if let Some(string) = element.as_string_lit() {
            let decoded = string
                .decoded_value()
                .map_err(|err| format!("failed to decode string value in .luarc.json: {err}"))?;
            if decoded == expected {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{AddonRegistryEntry, parse_addon_name_from_path, parse_gitmodules, update_luarc};
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_addon_name_from_path() {
        assert_eq!(
            parse_addon_name_from_path("addons/openresty/module"),
            Some("openresty".to_string())
        );
        assert_eq!(parse_addon_name_from_path("addons/openresty"), None);
    }

    #[test]
    fn parses_gitmodules_entries() {
        let parsed = parse_gitmodules(
            r#"
[submodule "addons/openresty/module"]
    path = addons/openresty/module
    url = https://github.com/LuaCATS/openresty.git
[submodule "addons/luvit/module"]
    path = addons/luvit/module
    url = https://github.com/Bilal2453/luvit-meta.git
    branch = release
"#,
        )
        .expect("gitmodules should parse");

        let mut expected = BTreeMap::new();
        expected.insert(
            "luvit".to_string(),
            AddonRegistryEntry {
                name: "luvit".to_string(),
                url: "https://github.com/Bilal2453/luvit-meta.git".to_string(),
                branch: Some("release".to_string()),
            },
        );
        expected.insert(
            "openresty".to_string(),
            AddonRegistryEntry {
                name: "openresty".to_string(),
                url: "https://github.com/LuaCATS/openresty.git".to_string(),
                branch: None,
            },
        );

        assert_eq!(parsed, expected);
    }

    #[test]
    fn creates_luarc_with_workspace_user_third_party() {
        let temp_dir = unique_temp_dir();
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let luarc_path = temp_dir.join(".luarc.json");
        let addon_root = temp_dir.join("addons");
        fs::create_dir_all(&addon_root).expect("addon root should be created");

        update_luarc(&luarc_path, &addon_root).expect("luarc should be written");

        let written = fs::read_to_string(&luarc_path).expect("luarc should exist");
        let parsed: serde_json::Value =
            serde_json::from_str(&written).expect("luarc should be valid json");

        assert_eq!(
            parsed.get("workspace.userThirdParty"),
            Some(&serde_json::json!([addon_root
                .to_string_lossy()
                .to_string()]))
        );

        fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn preserves_existing_lua_prefixed_key_style() {
        let temp_dir = unique_temp_dir();
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let luarc_path = temp_dir.join(".luarc.json");
        let addon_root = temp_dir.join("addons");
        fs::create_dir_all(&addon_root).expect("addon root should be created");
        fs::write(
            &luarc_path,
            r#"{
  "Lua.workspace.userThirdParty": [
    "/tmp/existing"
  ]
}
"#,
        )
        .expect("seed luarc should be written");

        update_luarc(&luarc_path, &addon_root).expect("luarc should be updated");

        let written = fs::read_to_string(&luarc_path).expect("luarc should exist");
        let parsed: serde_json::Value =
            serde_json::from_str(&written).expect("luarc should be valid json");

        assert_eq!(
            parsed.get("Lua.workspace.userThirdParty"),
            Some(&serde_json::json!([
                "/tmp/existing",
                addon_root.to_string_lossy().to_string()
            ]))
        );

        fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn accepts_jsonc_luarc_with_comments_and_trailing_commas() {
        let temp_dir = unique_temp_dir();
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let luarc_path = temp_dir.join(".luarc.json");
        let addon_root = temp_dir.join("addons");
        fs::create_dir_all(&addon_root).expect("addon root should be created");
        fs::write(
            &luarc_path,
            format!(
                r#"{{
  // existing addon roots
  "workspace.userThirdParty": [
    "/tmp/existing",
  ],
  /* another setting */
  "runtime.version": "LuaJIT",
}}
"#
            ),
        )
        .expect("seed luarc should be written");

        update_luarc(&luarc_path, &addon_root).expect("jsonc luarc should be updated");

        let written = fs::read_to_string(&luarc_path).expect("luarc should exist");
        let parsed = jsonc_parser::parse_to_serde_value(&written, &Default::default())
            .expect("written luarc should be valid jsonc")
            .expect("written luarc should contain a root value");

        assert!(written.contains("// existing addon roots"));
        assert!(written.contains("/* another setting */"));
        assert_eq!(
            parsed.get("workspace.userThirdParty"),
            Some(&serde_json::json!([
                "/tmp/existing",
                addon_root.to_string_lossy().to_string()
            ]))
        );
        assert_eq!(
            parsed.get("runtime.version"),
            Some(&serde_json::Value::String("LuaJIT".to_string()))
        );

        fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    fn unique_temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("lls-addon-install-test-{nanos}"))
    }
}
