//! Root view: command grid, output panel, and theme toggle.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use airtable_sync_core::{CommandDispatch, DispatchResult};
use egui::{RichText, ScrollArea, Ui};
use nest_config::ConfigService;
use nest_core::AppContext;
use nest_error::NestResult;
use nest_gui::GuiView;
use nest_theme::{ThemeId, ThemeMode, ThemeService};

/// A command exposed in the button grid.
struct CommandButton {
    label: &'static str,
    args: &'static [&'static str],
    json: bool,
}

/// Command groups shown in the grid.
struct CommandGroup {
    title: &'static str,
    commands: &'static [CommandButton],
}

const COMMAND_GROUPS: &[CommandGroup] = &[
    CommandGroup {
        title: "Reports",
        commands: &[
            CommandButton {
                label: "report summary",
                args: &["report", "summary"],
                json: true,
            },
            CommandButton {
                label: "report validation",
                args: &["report", "validation"],
                json: true,
            },
            CommandButton {
                label: "report changes",
                args: &["report", "changes"],
                json: true,
            },
        ],
    },
    CommandGroup {
        title: "Sync",
        commands: &[
            CommandButton {
                label: "sync dry-run",
                args: &["sync", "dry-run"],
                json: true,
            },
            CommandButton {
                label: "sync review",
                args: &["sync", "review"],
                json: true,
            },
            CommandButton {
                label: "sync approve-all",
                args: &["sync", "approve-all"],
                json: true,
            },
            CommandButton {
                label: "sync apply",
                args: &["sync", "apply"],
                json: true,
            },
        ],
    },
    CommandGroup {
        title: "Setup",
        commands: &[
            CommandButton {
                label: "config validate",
                args: &["config", "validate"],
                json: true,
            },
            CommandButton {
                label: "db init",
                args: &["db", "init"],
                json: true,
            },
            CommandButton {
                label: "airtable pull-schema",
                args: &["airtable", "pull-schema"],
                json: true,
            },
            CommandButton {
                label: "csv import-headers",
                args: &["csv", "import-headers"],
                json: true,
            },
        ],
    },
    CommandGroup {
        title: "Other",
        commands: &[CommandButton {
            label: "version",
            args: &["version"],
            json: true,
        }],
    },
];

enum DispatchState {
    Idle,
    Running {
        command: String,
        receiver: mpsc::Receiver<DispatchResult>,
    },
    Done {
        command: String,
        result: DispatchResult,
    },
}

/// Minimal GUI shell: dispatches to CLI commands and shows output.
#[derive(Default)]
pub struct MainView {
    dispatch: Option<CommandDispatch>,
    state: Option<DispatchState>,
    last_command: String,
    output_text: String,
    theme_initialized: bool,
}

impl MainView {
    fn ensure_dispatch(&mut self, ctx: &AppContext) {
        if self.dispatch.is_some() {
            return;
        }
        let config_path = ctx
            .service::<ConfigService>()
            .ok()
            .and_then(|config| config.path().map(PathBuf::from));
        self.dispatch = Some(CommandDispatch::new(config_path));
    }

    fn ensure_dark_theme(&mut self, ctx: &AppContext) {
        if self.theme_initialized {
            return;
        }
        if let Ok(theme_service) = ctx.service::<ThemeService>() {
            let _ = theme_service.set_active_theme(&ThemeId::from("nest-dark"));
            self.theme_initialized = true;
        }
    }

    fn apply_theme(&self, ui: &mut Ui, ctx: &AppContext) {
        ui.ctx().style_mut(|style| {
            if let Ok(theme_service) = ctx.service::<ThemeService>() {
                if let Ok(theme) = theme_service.active_theme() {
                    match theme.mode {
                        ThemeMode::Dark => style.visuals = egui::Visuals::dark(),
                        ThemeMode::Light => style.visuals = egui::Visuals::light(),
                    }
                }
            }
        });
    }

    fn active_theme_label(&self, ctx: &AppContext) -> String {
        ctx.service::<ThemeService>()
            .ok()
            .and_then(|themes| themes.active_id().ok())
            .map(|id| id.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn is_running(&self) -> bool {
        matches!(self.state, Some(DispatchState::Running { .. }))
    }

    fn poll_dispatch(&mut self) {
        let done = match &self.state {
            Some(DispatchState::Running { receiver, command }) => receiver
                .try_recv()
                .ok()
                .map(|result| (command.clone(), result)),
            _ => None,
        };

        if let Some((command, result)) = done {
            self.output_text = format_dispatch_output(&result);
            self.last_command = command.clone();
            self.state = Some(DispatchState::Done { command, result });
        }
    }

    fn run_command(&mut self, label: &str, args: &[&str], json: bool) {
        let Some(dispatch) = self.dispatch.clone() else {
            return;
        };
        let args_owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let args_for_thread: Vec<&str> = args_owned.iter().map(String::as_str).collect();
            let result = dispatch.run(&args_for_thread, json);
            let _ = sender.send(result);
        });

        self.state = Some(DispatchState::Running {
            command: label.to_string(),
            receiver,
        });
        self.output_text = format!("Running `{label}`…\n");
    }

    fn status_label(&self) -> &'static str {
        match &self.state {
            None | Some(DispatchState::Idle) => "idle",
            Some(DispatchState::Running { .. }) => "running",
            Some(DispatchState::Done { result, .. }) if result.success => "ok",
            Some(DispatchState::Done { .. }) => "failed",
        }
    }

    fn render_top_bar(&mut self, ui: &mut Ui, ctx: &AppContext) {
        ui.horizontal(|ui| {
            ui.heading("Airtable Sync");
            ui.separator();
            ui.label(format!("theme: {}", self.active_theme_label(ctx)));
            ui.separator();

            let running = self.is_running();
            if ui
                .add_enabled(!running, egui::Button::new("Light"))
                .clicked()
            {
                if let Ok(theme_service) = ctx.service::<ThemeService>() {
                    let _ = theme_service.set_active_theme(&ThemeId::from("nest-light"));
                    ui.ctx().request_repaint();
                }
            }
            if ui
                .add_enabled(!running, egui::Button::new("Dark"))
                .clicked()
            {
                if let Ok(theme_service) = ctx.service::<ThemeService>() {
                    let _ = theme_service.set_active_theme(&ThemeId::from("nest-dark"));
                    ui.ctx().request_repaint();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status = self.status_label();
                let color = match status {
                    "ok" => egui::Color32::from_rgb(34, 197, 94),
                    "failed" => egui::Color32::from_rgb(239, 68, 68),
                    "running" => egui::Color32::from_rgb(59, 130, 246),
                    _ => egui::Color32::GRAY,
                };
                ui.colored_label(color, format!("status: {status}"));
            });
        });
    }

    fn render_command_grid(&mut self, ui: &mut Ui) {
        let running = self.is_running();
        ScrollArea::vertical()
            .max_height(280.0)
            .id_salt("command_grid")
            .show(ui, |ui| {
                for group in COMMAND_GROUPS {
                    ui.add_space(4.0);
                    ui.label(RichText::new(group.title).strong());
                    ui.horizontal_wrapped(|ui| {
                        for command in group.commands {
                            if ui
                                .add_enabled(!running, egui::Button::new(command.label))
                                .clicked()
                            {
                                self.run_command(command.label, command.args, command.json);
                            }
                        }
                    });
                }
            });
    }

    fn render_output_panel(&self, ui: &mut Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Output").strong());
            if !self.last_command.is_empty() {
                ui.label(format!("(last: `{}`)", self.last_command));
            }
        });

        ScrollArea::both()
            .id_salt("output_panel")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&self.output_text).monospace())
                        .selectable(true),
                );
            });
    }
}

impl GuiView for MainView {
    fn ui(&mut self, ui: &mut Ui, ctx: &AppContext) -> NestResult<()> {
        self.ensure_dispatch(ctx);
        self.ensure_dark_theme(ctx);
        self.apply_theme(ui, ctx);
        self.poll_dispatch();

        ui.vertical(|ui| {
            self.render_top_bar(ui, ctx);
            ui.separator();
            self.render_command_grid(ui);
            ui.add_space(8.0);
            self.render_output_panel(ui);
        });

        if self.is_running() {
            ui.ctx().request_repaint();
        }

        Ok(())
    }
}

fn format_dispatch_output(result: &DispatchResult) -> String {
    let mut text = String::new();
    if !result.stdout.is_empty() {
        text.push_str(&result.stdout);
        if !result.stdout.ends_with('\n') {
            text.push('\n');
        }
    }
    if let Some(error) = &result.error {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&error.to_string());
    }
    if text.is_empty() {
        if result.success {
            text.push_str("(command completed with no output)\n");
        } else {
            text.push_str("(command failed)\n");
        }
    }
    text
}
