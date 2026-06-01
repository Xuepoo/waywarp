use clap::Parser;

mod agent;
mod config;
mod hint;
mod input;
mod pointer;
mod render;

/// A high-performance, keyboard-driven mouse control tool for Wayland compositors
#[derive(Parser, Debug)]
#[command(name = "waywarp", version, about)]
struct Args {
    /// List all hints with coordinates as JSON
    #[arg(long)]
    list_hints: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Programmatically select a hint by label
    #[arg(long)]
    select: Option<String>,

    /// Move cursor to specific coordinates (x y)
    #[arg(long, num_args = 2)]
    move_to: Option<Vec<i32>>,

    /// Click type
    #[arg(long, value_enum, default_value = "left")]
    click: ClickType,

    /// Print coordinates after selection
    #[arg(long)]
    print_coords: bool,

    /// One-shot mode: exit after first selection
    #[arg(long)]
    oneshot: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ClickType {
    Left,
    Right,
    Middle,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Load configuration
    let _config = config::Config::load();

    let args = Args::parse();

    if args.list_hints {
        agent::AgentMode::list_hints(&_config)?;
        return Ok(());
    }

    if let Some(label) = &args.select {
        agent::AgentMode::select_hint(label, &_config)?;
        return Ok(());
    }

    if let Some(coords) = &args.move_to {
        let mouse_btn = match args.click {
            ClickType::Left => pointer::MouseButton::Left,
            ClickType::Right => pointer::MouseButton::Right,
            ClickType::Middle => pointer::MouseButton::Middle,
        };
        agent::AgentMode::move_to(coords[0], coords[1], Some(mouse_btn), &_config)?;
        return Ok(());
    }

    // Default: interactive hint mode
    let config = config::Config::load();
    let grid = hint::HintGrid::new(0, 0, 0);
    let mut renderer = render::Renderer::new()?;
    renderer.draw_overlay(&grid, &config)?;

    Ok(())
}
