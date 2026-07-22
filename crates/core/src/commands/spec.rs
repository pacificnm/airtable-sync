//! CLI command tree matching the product README.

/// A nested subcommand specification.
pub struct SubcommandSpec {
    /// Subcommand name (kebab-case).
    pub name: &'static str,
    /// Short help text.
    pub about: &'static str,
}

/// A top-level command group and its nested subcommands.
pub struct CommandGroupSpec {
    /// Top-level command name.
    pub name: &'static str,
    /// Short help text for the group.
    pub about: &'static str,
    /// Nested subcommands (empty for leaf commands such as `version`).
    pub subcommands: &'static [SubcommandSpec],
}

/// Full CLI command tree for `--help` and future implementation.
pub const COMMAND_GROUPS: &[CommandGroupSpec] = &[
    CommandGroupSpec {
        name: "setup",
        about: "First-run setup and onboarding",
        subcommands: &[SubcommandSpec {
            name: "init",
            about: "Validate config, init DB, pull schema, import CSV headers, auto-map",
        }],
    },
    CommandGroupSpec {
        name: "config",
        about: "Configuration management",
        subcommands: &[
            SubcommandSpec {
                name: "validate",
                about: "Validate config.toml",
            },
            SubcommandSpec {
                name: "show",
                about: "Display the loaded configuration",
            },
            SubcommandSpec {
                name: "init",
                about: "Create a default config.toml",
            },
        ],
    },
    CommandGroupSpec {
        name: "db",
        about: "SQLite database management",
        subcommands: &[
            SubcommandSpec {
                name: "init",
                about: "Create the SQLite database",
            },
            SubcommandSpec {
                name: "reset",
                about: "Recreate the database (destructive)",
            },
            SubcommandSpec {
                name: "schema",
                about: "Display database schema information",
            },
            SubcommandSpec {
                name: "migrate",
                about: "Apply database migrations",
            },
        ],
    },
    CommandGroupSpec {
        name: "airtable",
        about: "Airtable schema operations",
        subcommands: &[
            SubcommandSpec {
                name: "test",
                about: "Test Airtable connectivity",
            },
            SubcommandSpec {
                name: "pull-schema",
                about: "Download tables and fields into SQLite",
            },
            SubcommandSpec {
                name: "list-tables",
                about: "List configured Airtable tables",
            },
            SubcommandSpec {
                name: "list-fields",
                about: "List fields for a table",
            },
        ],
    },
    CommandGroupSpec {
        name: "csv",
        about: "CSV import operations",
        subcommands: &[
            SubcommandSpec {
                name: "import-headers",
                about: "Import CSV headers into SQLite",
            },
            SubcommandSpec {
                name: "preview",
                about: "Preview CSV records",
            },
            SubcommandSpec {
                name: "validate",
                about: "Validate CSV structure",
            },
        ],
    },
    CommandGroupSpec {
        name: "mapping",
        about: "Field mapping management",
        subcommands: &[
            SubcommandSpec {
                name: "auto",
                about: "Auto-map CSV fields to Airtable fields",
            },
            SubcommandSpec {
                name: "list",
                about: "Display current mappings",
            },
            SubcommandSpec {
                name: "set",
                about: "Create or update a field mapping",
            },
            SubcommandSpec {
                name: "remove",
                about: "Remove a mapping",
            },
            SubcommandSpec {
                name: "enable",
                about: "Enable field synchronization",
            },
            SubcommandSpec {
                name: "disable",
                about: "Disable field synchronization",
            },
            SubcommandSpec {
                name: "report",
                about: "Generate mapping report",
            },
        ],
    },
    CommandGroupSpec {
        name: "compare",
        about: "Compare CSV to Airtable",
        subcommands: &[
            SubcommandSpec {
                name: "table",
                about: "Compare one table",
            },
            SubcommandSpec {
                name: "all",
                about: "Compare every configured table",
            },
        ],
    },
    CommandGroupSpec {
        name: "sync",
        about: "Synchronize Airtable",
        subcommands: &[
            SubcommandSpec {
                name: "dry-run",
                about: "Generate update plan only (no writes)",
            },
            SubcommandSpec {
                name: "review",
                about: "Review the active change plan",
            },
            SubcommandSpec {
                name: "approve",
                about: "Approve one pending change",
            },
            SubcommandSpec {
                name: "deny",
                about: "Deny one pending change",
            },
            SubcommandSpec {
                name: "approve-all",
                about: "Approve all pending changes in the active plan",
            },
            SubcommandSpec {
                name: "deny-all",
                about: "Deny all pending changes in the active plan",
            },
            SubcommandSpec {
                name: "apply",
                about: "Apply approved updates",
            },
            SubcommandSpec {
                name: "table",
                about: "Synchronize a single table",
            },
            SubcommandSpec {
                name: "all",
                about: "Synchronize all enabled tables",
            },
        ],
    },
    CommandGroupSpec {
        name: "report",
        about: "Generate reports",
        subcommands: &[
            SubcommandSpec {
                name: "changes",
                about: "Generate change report",
            },
            SubcommandSpec {
                name: "validation",
                about: "Validation report",
            },
            SubcommandSpec {
                name: "summary",
                about: "Overall sync summary",
            },
        ],
    },
    CommandGroupSpec {
        name: "cache",
        about: "Cache management",
        subcommands: &[SubcommandSpec {
            name: "clear",
            about: "Clear cached schema",
        }],
    },
    CommandGroupSpec {
        name: "logs",
        about: "Log file utilities",
        subcommands: &[SubcommandSpec {
            name: "show",
            about: "View recent logs",
        }],
    },
    CommandGroupSpec {
        name: "version",
        about: "Display version information",
        subcommands: &[],
    },
];
