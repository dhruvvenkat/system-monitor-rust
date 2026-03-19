use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    AppResult,
    cli::Cli,
    model::SortField,
    processes::ProcessCollector,
    query::{self, Query},
    ui,
};

pub fn run(cli: Cli) -> AppResult<()> {
    let mut app = App::from_cli(cli);

    if app.cli.json {
        let snapshot = app.collector.snapshot()?;
        let rendered = ui::render_json(&snapshot)?;
        println!("{rendered}");
        return Ok(());
    }

    if app.cli.once {
        app.refresh()?;
        let snapshot = app
            .snapshot
            .as_ref()
            .expect("snapshot exists after refresh");
        let rows = app.filtered_rows();
        let rendered = ui::render_once(snapshot, &rows, &app.query);
        println!("{rendered}");
        return Ok(());
    }

    let mut terminal = TerminalSession::enter()?;
    app.refresh()?;

    loop {
        terminal.terminal.draw(|frame| {
            app.draw(frame);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        if app.last_refresh.elapsed() >= app.tick_rate {
            app.refresh()?;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

struct App {
    cli: Cli,
    query: Query,
    collector: ProcessCollector,
    snapshot: Option<crate::model::SystemSnapshot>,
    should_quit: bool,
    tick_rate: Duration,
    last_refresh: Instant,
}

impl App {
    fn from_cli(cli: Cli) -> Self {
        let tick_rate = Duration::from_millis(cli.interval_ms);
        let query = Query::from_cli(cli.sort, cli.ascending, cli.filter.clone(), cli.limit);

        Self {
            cli,
            query,
            collector: ProcessCollector::new(),
            snapshot: None,
            should_quit: false,
            tick_rate,
            last_refresh: Instant::now(),
        }
    }

    fn refresh(&mut self) -> AppResult<()> {
        self.snapshot = Some(self.collector.snapshot()?);
        self.last_refresh = Instant::now();
        Ok(())
    }

    fn draw(&self, frame: &mut ratatui::Frame<'_>) {
        if let Some(snapshot) = self.snapshot.as_ref() {
            let rows = self.filtered_rows();
            ui::render(frame, snapshot, &rows, &self.query, self.tick_rate);
        } else {
            ui::render_loading(frame, &self.query);
        }
    }

    fn filtered_rows(&self) -> Vec<&crate::model::ProcessEntry> {
        match self.snapshot.as_ref() {
            Some(snapshot) => query::apply(snapshot, &self.query),
            None => Vec::new(),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Char('r') => {
                if self.refresh().is_err() {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('s') => {
                self.query.sort_by = next_sort_field(self.query.sort_by);
            }
            KeyCode::Char('a') | KeyCode::Char('d') => {
                self.query.descending = !self.query.descending;
            }
            _ => {}
        }
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    fn enter() -> AppResult<Self> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(|error| {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
            error
        })?;

        Ok(Self { terminal })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }
}

fn next_sort_field(sort: SortField) -> SortField {
    match sort {
        SortField::Cpu => SortField::Memory,
        SortField::Memory => SortField::Pid,
        SortField::Pid => SortField::Name,
        SortField::Name => SortField::Cpu,
    }
}
