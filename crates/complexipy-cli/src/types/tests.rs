use std::fs;
use std::path::PathBuf;

use complexipy_core::config::LspSection;
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

use super::Config;

const SCHEMA_ID: &str = "https://www.schemastore.org/complexipy.json";

fn workspace_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name)
}

fn config_schema() -> Value {
    let mut generator = SchemaSettings::draft07().into_generator();
    let lsp = generator.subschema_for::<LspSection>();
    let mut schema = generator.into_root_schema_for::<Config>().to_value();

    schema["$id"] = json!(SCHEMA_ID);
    schema["properties"]["lsp"] = lsp.to_value();
    schema
}

fn rendered_schema() -> String {
    let mut text = serde_json::to_string_pretty(&config_schema()).unwrap();
    text.push('\n');
    text
}

fn toml_to_json(text: &str) -> Value {
    serde_json::to_value(text.parse::<toml::Table>().unwrap()).unwrap()
}

#[test]
fn committed_schema_matches_the_config_types() {
    let path = workspace_file("complexipy.schema.json");
    let expected = rendered_schema();

    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        fs::write(&path, &expected).unwrap();
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_default();

    assert!(
        committed.replace("\r\n", "\n") == expected,
        "complexipy.schema.json is stale: run \
         `UPDATE_SCHEMA=1 cargo test -p complexipy-cli committed_schema`"
    );
}

#[test]
fn repository_config_files_match_the_schema() {
    let validator = jsonschema::validator_for(&config_schema()).unwrap();
    let complexipy_toml = fs::read_to_string(workspace_file("complexipy.toml")).unwrap();
    let pyproject = fs::read_to_string(workspace_file("pyproject.toml")).unwrap();
    let configs = [
        toml_to_json(&complexipy_toml),
        toml_to_json(&pyproject)["tool"]["complexipy"].clone(),
    ];

    for config in configs {
        let errors: Vec<String> = validator
            .iter_errors(&config)
            .map(|error| error.to_string())
            .collect();

        assert!(errors.is_empty(), "{errors:?}");
    }
}

#[test]
fn unknown_keys_fail_validation() {
    let validator = jsonschema::validator_for(&config_schema()).unwrap();

    for text in [
        "max-complexity-alowed = 10",
        "[lsp]\ninlay-hint = \"always\"",
        "[diff]\nbase = \"main\"",
    ] {
        assert!(!validator.is_valid(&toml_to_json(text)), "{text}");
    }
}
