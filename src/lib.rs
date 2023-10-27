use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use color_eyre::eyre::Result;
use crossterm::{
    cursor,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event as CrosstermEvent,
        KeyEvent, KeyEventKind, MouseEvent,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use ratatui::backend::CrosstermBackend as Backend;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

/// A collection of statuses handled by the internal
/// event loop.
#[derive(Clone, Debug)]
pub enum Event {

    /// Informational: notifies that the event loop is ready to receive events.
    Init,

    /// Notifies of an event failure.
    Error,

    /// Control-flow: emits a time tick instruction.
    Tick,

    /// Control-flow: emits a frame refresh instruction.
    Render,

    /// Input event: a key is pressed.
    Key(KeyEvent),

    /// Input event: window resize.
    Resize(u16, u16),

    /// Input event: mouse focus landed on a widget.
    /// Ignored, unless [Tui] is enabled with mouse support.
    FocusGained,

    /// Input event: mouse focus left a widget.
    /// Ignored, unless [Tui] is enabled with mouse support.
    FocusLost,

    /// Input event: a mouse click or scroll
    /// Ignored, unless [Tui] is enabled with mouse support.
    Mouse(MouseEvent),

    /// Input event: content pasted into the terminal.
    /// Ignored, unless [Tui] is enabled with copy-paste support.
    Paste(String),
}

/// A simple handler of terminal user interface with
/// optional mouse and copy-paste support.
pub struct Tui {
    terminal: ratatui::Terminal<Backend<std::io::Stderr>>,
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
    recv: UnboundedReceiver<Event>,
    send: UnboundedSender<Event>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub mouse_supported: bool,
    pub paste_supported: bool,
}

impl Tui {

    /// Creates a new instances with default parameters:
    /// - tick rate (TPS) = 4.0 ticks/sec
    /// - frame rate (FPS) = 60.0 frames/sec
    /// - no mouse or copy-paste support
    /// - uses [ratatui::backend::CrosstermBackend]
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    /// let mut interface = Tui::new().expect("failed to initialize TUI");
    /// ```
    pub fn new() -> Result<Self> {
        let tick_rate = 4.0;
        let frame_rate = 60.0;
        let terminal = ratatui::Terminal::new(Backend::new(std::io::stderr()))?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {});
        let mouse = false;
        let paste = false;
        Ok(Self { terminal, task, cancellation_token, recv: event_rx, send: event_tx, frame_rate, tick_rate, mouse_supported: mouse, paste_supported: paste })
    }

    /// Sets tick rate (TPS).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let tps = 5f64; // 5 ticks per second
    /// let mut interface = Tui::new()?
    ///     .tick_rate(tps);
    /// ```
    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    /// Sets frame rate (FPS).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let fps = 75f64; // 75 frames per second
    /// let mut interface = Tui::new()?
    ///     .frame_rate(fps);
    /// ```
    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    /// Enables/disables mouse support.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let mut with_mouse_support = Tui::new()?
    ///     .mouse(true);
    /// ```
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let mut no_mouse_support = Tui::new()?
    ///     .mouse(false);
    ///
    /// let mut also_no_mouse_support = Tui::new()?;
    /// ```
    pub fn mouse(mut self, mouse: bool) -> Self {
        self.mouse_supported = mouse;
        self
    }

    /// Enables/disables paste buffer support.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let mut with_paste_support = Tui::new()?
    ///     .paste(true);
    /// ```
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let mut no_paste_support = Tui::new()?
    ///     .paste(false);
    ///
    /// let mut also_no_paste_support = Tui::new()?;
    /// ```
    pub fn paste(mut self, paste: bool) -> Self {
        self.paste_supported = paste;
        self
    }

    /// Starts the terminal backend with configured settings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_template::Tui;
    ///
    /// let mut interface = Tui::new()?;
    ///
    /// interface.enter()?;
    /// // application logic goes here
    ///
    /// // it is HIGHLY recommended to gracefully exit
    /// // the TUI mode using this
    /// interface.exit()?;
    /// ```
    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), EnterAlternateScreen, cursor::Hide)?;
        if self.mouse_supported {
            crossterm::execute!(std::io::stderr(), EnableMouseCapture)?;
        }
        if self.paste_supported {
            crossterm::execute!(std::io::stderr(), EnableBracketedPaste)?;
        }
        self.start();
        Ok(())
    }

    /// A counterpart to [self::Tui::enter], will reset the
    /// terminal gracefully. To be invoked right before the
    /// application terminates.
    pub fn exit(&mut self) -> Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            if self.paste_supported {
                crossterm::execute!(std::io::stderr(), DisableBracketedPaste)?;
            }
            if self.mouse_supported {
                crossterm::execute!(std::io::stderr(), DisableMouseCapture)?;
            }
            crossterm::execute!(std::io::stderr(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    /// Requests next available application
    /// event and waits until it becomes available.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_template::{Event, Tui};
    ///
    /// let mut interface = Tui::new()?;
    /// interface.enter()?;
    ///
    /// loop {
    ///     if let Some(event) = interface.next() {
    ///         // do some event handling
    ///     }
    /// }
    ///
    /// interface.exit()?;
    /// ```
    pub async fn next(&mut self) -> Option<Event> {
        self.recv.recv().await
    }


    fn start(&mut self) {
        let tick_ttl = Duration::from_secs_f64(1.0 / self.tick_rate);
        let frame_ttl = Duration::from_secs_f64(1.0 / self.frame_rate);

        self.cancel();
        self.cancellation_token = CancellationToken::new();

        let _cancellation_token = self.cancellation_token.clone();
        let _event_send = self.send.clone();

        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_ttl);
            let mut render_interval = tokio::time::interval(frame_ttl);

            _event_send.send(Event::Init).unwrap();

            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = _cancellation_token.cancelled() => {
                        break;
                    }
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                match evt {
                                    CrosstermEvent::Key(key) => {
                                        if key.kind == KeyEventKind::Press {
                                            _event_send.send(Event::Key(key)).unwrap();
                                        }
                                    },
                                    CrosstermEvent::Mouse(mouse) => {
                                        _event_send.send(Event::Mouse(mouse)).unwrap();
                                    },
                                    CrosstermEvent::Resize(x, y) => {
                                        _event_send.send(Event::Resize(x, y)).unwrap();
                                    },
                                    CrosstermEvent::FocusLost => {
                                        _event_send.send(Event::FocusLost).unwrap();
                                    },
                                    CrosstermEvent::FocusGained => {
                                        _event_send.send(Event::FocusGained).unwrap();
                                    },
                                    CrosstermEvent::Paste(s) => {
                                        _event_send.send(Event::Paste(s)).unwrap();
                                    }
                                }
                            }
                            Some(Err(_)) => {
                                _event_send.send(Event::Error).unwrap();
                            }
                            None => {},
                        }
                    },
                    _ = tick_delay => {
                        _event_send.send(Event::Tick).unwrap();
                    },
                    _ = render_delay => {
                        _event_send.send(Event::Render).unwrap();
                    },
                }
            }
        });
    }

    fn stop(&self) -> Result<()> {
        self.cancel();
        let mut counter = 0;
        while !self.task.is_finished() {
            std::thread::sleep(Duration::from_millis(1));
            counter += 1;
            if counter > 50 {
                self.task.abort();
            }
            if counter > 100 {
                panic!("Failed to abort task in 100 milliseconds for unknown reason");
            }
        }
        Ok(())
    }

    fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /*fn suspend(&mut self) -> Result<()> {
        self.exit()?;
        #[cfg(not(windows))]
        signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP)?;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        self.enter()?;
        Ok(())
    }*/

}

impl Deref for Tui {
    type Target = ratatui::Terminal<Backend<std::io::Stderr>>;

    /// Dereferences the struct into the ratatui backend.
    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {

    /// Dereferences the struct into the ratatui backend.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {

    /// Delegates to [Tui::exit].
    fn drop(&mut self) {
        self.exit().unwrap();
    }
}