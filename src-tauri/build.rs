/// Commands exposed by the inline `airtable_sync` Tauri plugin (see `commands.rs`).
///
/// Listing them here lets `tauri-build` autogenerate `allow-*`/`deny-*` ACL
/// permissions and an `airtable-sync:default` set. Without this, Tauri v2 denies
/// every `plugin:airtable-sync|*` invoke ("plugin not found"). Keep in sync with
/// `airtable_sync_plugin`'s `generate_handler!`.
///
/// The plugin identifier must be hyphenated (`airtable-sync`) — Tauri ACL
/// identifiers reject underscores. Command names may keep underscores; they are
/// normalized to `allow-airtable-sync-run` etc.
const AIRTABLE_SYNC_COMMANDS: &[&str] = &["airtable_sync_run", "airtable_sync_command_groups"];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().plugin(
            "airtable-sync",
            tauri_build::InlinedPlugin::new()
                .commands(AIRTABLE_SYNC_COMMANDS)
                .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
        ),
    )
    .expect("failed to run tauri-build");
}
