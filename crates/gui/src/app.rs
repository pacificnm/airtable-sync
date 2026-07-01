//! GUI host wiring for Airtable Sync.

use nest_gui::{GuiApp, LoggingConfig};
use nest_theme::ThemeModule;

use crate::view::MainView;

/// Builds the Airtable Sync GUI host.
pub fn gui_app() -> GuiApp {
    GuiApp::new("airtable-sync")
        .with_logging(LoggingConfig::for_gui("airtable-sync"))
        .module(ThemeModule::default())
        .view(MainView::default())
}
