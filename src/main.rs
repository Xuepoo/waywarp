use clap::Parser;

mod agent;
mod config;
mod hint;
mod input;
mod pointer;
mod render;
mod scanner;

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

    /// Move cursor by relative coordinate offsets (dx dy)
    #[arg(long, num_args = 2)]
    move_by: Option<Vec<i32>>,

    /// Click type
    #[arg(long, value_enum, default_value = "left")]
    click: ClickType,

    /// Print coordinates after selection
    #[arg(long)]
    print_coords: bool,

    /// One-shot mode: exit after first selection
    #[arg(long)]
    oneshot: bool,

    /// Start in continuous Normal Mode (cursor drive)
    #[arg(long)]
    normal: bool,

    /// Trigger head-less visual scan and render overlay on top of GUI widgets
    #[arg(long)]
    scan: bool,
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
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    // Load configuration
    let _config = config::Config::load();

    let args = Args::parse();

    if args.list_hints {
        agent::AgentMode::list_hints(&_config)?;
        return Ok(());
    }

    if let Some(label) = &args.select {
        let coords = agent::AgentMode::select_hint(label, &_config)?;
        if args.print_coords {
            match args.format {
                OutputFormat::Text => {
                    println!("{} {}", coords.0, coords.1);
                }
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "x": coords.0,
                        "y": coords.1,
                        "screen": coords.2,
                    });
                    println!("{}", json);
                }
            }
        }
        println!("ok");
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

    if let Some(offsets) = &args.move_by {
        let mouse_btn = match args.click {
            ClickType::Left => pointer::MouseButton::Left,
            ClickType::Right => pointer::MouseButton::Right,
            ClickType::Middle => pointer::MouseButton::Middle,
        };
        agent::AgentMode::move_by(offsets[0], offsets[1], Some(mouse_btn), &_config)?;
        return Ok(());
    }

    if args.scan {
        let config = config::Config::load();
        let scanner_output = scanner::run_visual_scan()?;
        let chars = hint::HintGrid::get_unique_chars(&config.hint_chars);
        let labels = hint::HintGrid::generate_labels(scanner_output.elements.len(), &chars);

        let mut grid =
            hint::HintGrid::new(scanner_output.screen_width, scanner_output.screen_height, 0);
        grid.is_element_based = true;

        for (i, elem) in scanner_output.elements.iter().enumerate() {
            let label = labels[i].clone();
            grid.hints.push(hint::Hint {
                label,
                x: elem.center[0],
                y: elem.center[1],
                width: 40,
                height: 30,
                screen: elem.monitor_index,
            });
        }

        let mut renderer = render::Renderer::new()?;
        renderer.draw_overlay(&grid, &config)?;
        return Ok(());
    }

    // Default: interactive hint mode
    let config = config::Config::load();
    let grid = hint::HintGrid::new(0, 0, 0);
    let mut renderer = render::Renderer::new()?;
    if args.normal {
        renderer.state.borrow_mut().mode = render::InteractionMode::Normal;
    }
    renderer.draw_overlay(&grid, &config)?;

    Ok(())
}
