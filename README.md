# Base TUI template

Almost completely copied from the [Ratatui Book](https://ratatui.rs/how-to/develop-apps/abstract-terminal-and-event-handler.html).

Any author credentials belong to the [Ratatui team](https://github.com/ratatui-org). This repository does not claim any ownership over the source code.

# Getting started

Add the dependency to the `Cargo.toml`:

```toml
ratatui-template = "0.1.0"
```

And then simply include it into your Rust TUI app:

```rust
use ratatui_template::Tui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let mut interface = Tui::new()?;
    
    interface.enter()?;
    loop {
        // implement application loop
    }
    interface.exit()?;
}

```

> Note: async features are powered by `tokio`.

# Building

Built as a Rust lib, you know the drill:

```shell
cargo build
```

The project requires Rust `1.73.0` or later.
