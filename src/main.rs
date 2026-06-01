use clap::Parser;

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

    let args = Args::parse();

    if args.list_hints {
        // TODO: Implement hint listing
        println!("{{\"hints\": [], \"message\": \"not yet implemented\"}}");
        return Ok(());
    }

    if let Some(label) = &args.select {
        // TODO: Implement hint selection
        println!("Selected hint: {label}");
        return Ok(());
    }

    if let Some(coords) = &args.move_to {
        // TODO: Implement cursor movement
        println!("Moving to ({}, {})", coords[0], coords[1]);
        return Ok(());
    }

    // Default: interactive hint mode
    // TODO: Implement interactive hint overlay
    eprintln!("Interactive hint mode not yet implemented");
    eprintln!("Use --help to see available options");

    Ok(())
}
