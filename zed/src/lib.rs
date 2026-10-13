use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

const SERVER_NAME: &str = "complexipy";

struct ComplexipyExtension;

impl zed::Extension for ComplexipyExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary = LspSettings::for_worktree(SERVER_NAME, worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let configured_path = binary.as_ref().and_then(|binary| binary.path.clone());
        let args = binary
            .and_then(|binary| binary.arguments)
            .unwrap_or_else(|| vec!["lsp".to_string()]);

        let command = configured_path
            .or_else(|| worktree.which(SERVER_NAME))
            .ok_or_else(|| {
                "complexipy was not found on the PATH. Install it with \
                 `uv tool install complexipy` or `pipx install complexipy`, \
                 or set `lsp.complexipy.binary.path` in the Zed settings."
                    .to_string()
            })?;

        Ok(zed::Command {
            command,
            args,
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(ComplexipyExtension);
