mod app;
mod config;
mod discovery;
mod events;
mod log_stream;
mod search;
mod ssh;
mod ui;

use std::io;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use crossterm::{
    event::{poll, read},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use tokio::runtime::Runtime;

use crate::app::App;
use crate::config::Config;
use crate::ssh::{SshSession, SshTarget};

#[derive(Parser, Debug)]
#[command(name = "lview")]
#[command(about = "TUI for viewing remote logs over SSH")]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Connect via SSH and browse logs.
    ///
    /// All arguments after `ssh` are passed directly to the system ssh command,
    /// so your ~/.ssh/config, keys and agent settings are honored.
    Ssh {
        /// Arguments passed to ssh (e.g. -p22 root@host).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    let _ = execute!(io::stdout(), crossterm::event::DisableMouseCapture);
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let command = args.command.unwrap_or_else(|| {
        eprintln!("Usage: lview ssh [ssh-options] [user@]host");
        std::process::exit(1);
    });

    let Command::Ssh { args: ssh_args } = command;
    if ssh_args.is_empty() {
        eprintln!("Error: missing SSH target.");
        eprintln!("Example: lview ssh -p22 root@192.168.53.3");
        std::process::exit(1);
    }

    let config = Config::load();
    let target = SshTarget { args: ssh_args };
    let session = SshSession::new(config.ssh_command.clone(), target);

    let rt = Runtime::new()?;

    // Run discovery before entering TUI
    let mut app = App::new(session, config);
    rt.block_on(app.discover());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Ensure terminal is restored on panic or Ctrl-C
    std::panic::set_hook(Box::new(|info| {
        restore_terminal();
        eprintln!("Panic: {}", info);
    }));
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("failed to set Ctrl-C handler");

    let result = run_app(&mut terminal, app, &rt, running);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    rt: &Runtime,
    running: Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()>
where
    io::Error: From<<B as Backend>::Error>,
{
    let tick_rate = Duration::from_millis(100);

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        app.update_stream();

        if app.loading {
            rt.block_on(app.discover());
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        if poll(tick_rate)? {
            let event = read()?;
            if !events::handle_event(&mut app, event) {
                return Ok(());
            }
        }
    }
    Ok(())
}
