use std::process::ExitCode;

use clap::Parser;
use tracing::Level;

use tex_cli::cli::{Cli, Commands, ConfigCmd, TemplateCmd, TemplatesCmd};
use tex_cli::errors::TexError;

const BANNER: &str = include_str!("../assets/banner.txt");

fn main() -> ExitCode {
    let cli = Cli::parse();

    init_tracing(cli.verbose);
    eprint!("{BANNER}");

    let outcome = match cli.command {
        Commands::Init(args) => tex_cli::cli::handle_init(args),
        Commands::Config(ConfigCmd::Show { format }) => tex_cli::cli::handle_config_show(format),
        Commands::Config(ConfigCmd::Set { key, value }) => {
            tex_cli::cli::handle_config_set(key, value)
        }
        Commands::Templates(args) => match args.command {
            Some(TemplatesCmd::List { format }) => tex_cli::cli::handle_templates_list(format),
            Some(TemplatesCmd::Show { name }) => tex_cli::cli::handle_templates_show(name),
            Some(TemplatesCmd::Add(add_args)) => tex_cli::cli::handle_templates_add(add_args),
            Some(TemplatesCmd::Remove { name, force }) => {
                tex_cli::cli::handle_templates_remove(name, force)
            }
            None => tex_cli::cli::handle_templates_menu(),
        },
        Commands::Template(cmd) => match cmd {
            TemplateCmd::Install { source, force } => {
                tex_cli::cli::handle_template_install(source, force)
            }
            TemplateCmd::List {
                json,
                include_builtin,
            } => tex_cli::cli::handle_template_list(json, include_builtin),
            TemplateCmd::Remove { identifier, yes } => {
                tex_cli::cli::handle_template_remove(identifier, yes)
            }
            TemplateCmd::Trust {
                identifier,
                version,
                revoke,
            } => tex_cli::cli::handle_template_trust(identifier, version, revoke),
        },
        Commands::Render(args) => tex_cli::cli::handle_render(args),
        Commands::Compile(args) => tex_cli::cli::handle_compile(args),
        Commands::Build(args) => tex_cli::cli::handle_build(args),
    };

    match outcome {
        Ok(()) => ExitCode::from(0),
        Err(err) => {
            eprintln!("{err}");
            let code = err
                .downcast_ref::<TexError>()
                .map(|te| te.exit_code())
                .unwrap_or(1);
            ExitCode::from(code as u8)
        }
    }
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .init();
}
