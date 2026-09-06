mod actions;
mod theme;
mod view_detail;
mod view_list;

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Direction, Layout};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::session::{Session, Store};

struct SessionRow {
    id: Uuid,
    name: String,
    updated_at: String,
    cookie_count: usize,
    header_count: usize,
}

enum Mode {
    Browsing,
    Filtering,
    ConfirmDelete,
    Help,
}

struct App {
    store: Store,
    rows: Vec<SessionRow>,
    filtered: Vec<usize>,
    selected: usize,
    filter: String,
    mode: Mode,
    reveal_secrets: bool,
    detail: Option<Session>,
    status: Option<String>,
    should_quit: bool,
}

impl App {
    fn new(config: &Config) -> Result<Self> {
        let mut app = Self {
            store: Store::new(config.sessions_dir.clone()),
            rows: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            filter: String::new(),
            mode: Mode::Browsing,
            reveal_secrets: false,
            detail: None,
            status: None,
            should_quit: false,
        };
        app.reload()?;
        Ok(app)
    }

    fn reload(&mut self) -> Result<()> {
        let mut list = self.store.list()?;
        list.sort_by(|a, b| a.1.cmp(&b.1));

        let mut rows = Vec::with_capacity(list.len());
        for (id, name) in list {
            let session = self.store.load(id)?;
            rows.push(SessionRow {
                id,
                name,
                updated_at: format_timestamp(session.meta.updated_at),
                cookie_count: session.cookies.len(),
                header_count: session.headers.len(),
            });
        }
        self.rows = rows;
        self.apply_filter();
        self.load_selected_detail()
    }

    fn apply_filter(&mut self) {
        let needle = self.filter.to_ascii_lowercase();
        self.filtered = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| needle.is_empty() || row.name.to_ascii_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn load_selected_detail(&mut self) -> Result<()> {
        self.detail = match self.filtered.get(self.selected) {
            Some(&idx) => Some(self.store.load(self.rows[idx].id)?),
            None => None,
        };
        Ok(())
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal
                .draw(|frame| self.draw(frame))
                .map_err(|source| Error::Io {
                    path: "<terminal>".into(),
                    source,
                })?;

            if event::poll(Duration::from_millis(250)).map_err(|source| Error::Io {
                path: "<terminal>".into(),
                source,
            })? && let Event::Key(key) = event::read().map_err(|source| Error::Io {
                path: "<terminal>".into(),
                source,
            })? && key.kind == KeyEventKind::Press
            {
                self.handle_key(key.code, terminal)?;
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(frame.area());

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(rows[0]);

        view_list::render(frame, columns[0], self);
        view_detail::render(frame, columns[1], self);
        self.render_status_bar(frame, rows[1]);

        match self.mode {
            Mode::ConfirmDelete => actions::render_confirm_delete(frame, self),
            Mode::Help => actions::render_help(frame),
            Mode::Browsing | Mode::Filtering => {}
        }
    }

    fn render_status_bar(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::text::Line;
        use ratatui::widgets::Paragraph;

        let text = match (&self.mode, &self.status) {
            (Mode::Filtering, _) => format!("filter: {}\u{2588}", self.filter),
            (_, Some(status)) => status.clone(),
            _ => "j/k move  /filter  s reveal  e edit  d delete  ? help  q quit".to_string(),
        };
        let style = if self.status.is_some() {
            theme::success()
        } else {
            theme::dim()
        };
        frame.render_widget(Paragraph::new(Line::styled(text, style)), area);
    }

    fn handle_key(&mut self, code: KeyCode, terminal: &mut DefaultTerminal) -> Result<()> {
        self.status = None;
        match self.mode {
            Mode::ConfirmDelete => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete()?,
                _ => self.mode = Mode::Browsing,
            },
            Mode::Help => self.mode = Mode::Browsing,
            Mode::Filtering => match code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.apply_filter();
                    self.mode = Mode::Browsing;
                    self.load_selected_detail()?;
                }
                KeyCode::Enter => self.mode = Mode::Browsing,
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.apply_filter();
                    self.load_selected_detail()?;
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.apply_filter();
                    self.load_selected_detail()?;
                }
                _ => {}
            },
            Mode::Browsing => match code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Down | KeyCode::Char('j') => self.move_selection(1)?,
                KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1)?,
                KeyCode::Char('/') => self.mode = Mode::Filtering,
                KeyCode::Char('s') => self.reveal_secrets = !self.reveal_secrets,
                KeyCode::Char('d') if !self.filtered.is_empty() => self.mode = Mode::ConfirmDelete,
                KeyCode::Char('e') if !self.filtered.is_empty() => self.edit_selected(terminal)?,
                KeyCode::Char('?') => self.mode = Mode::Help,
                _ => {}
            },
        }
        Ok(())
    }

    fn move_selection(&mut self, delta: i32) -> Result<()> {
        if self.filtered.is_empty() {
            return Ok(());
        }
        let len = self.filtered.len() as i32;
        let next = (self.selected as i32 + delta).clamp(0, len - 1);
        self.selected = next as usize;
        self.load_selected_detail()
    }

    fn confirm_delete(&mut self) -> Result<()> {
        if let Some(&idx) = self.filtered.get(self.selected) {
            let name = self.rows[idx].name.clone();
            self.store.delete(&name)?;
            self.status = Some(format!("Deleted \"{name}\"."));
            self.reload()?;
        }
        self.mode = Mode::Browsing;
        Ok(())
    }

    /// Suspends the TUI's terminal takeover, hands the terminal to
    /// `$EDITOR`, then restores the TUI afterward and re-reads the session
    /// from disk. `ratatui::init()`'s panic hook keeps covering the
    /// terminal across this shell-out — if the editor process itself
    /// panics something upstream, the hook still restores raw mode.
    fn edit_selected(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(&idx) = self.filtered.get(self.selected) else {
            return Ok(());
        };
        let id = self.rows[idx].id;
        let path = self.store.path_for(id);

        ratatui::restore();
        let edit_result = crate::editor::open(&path);
        *terminal = ratatui::init();
        terminal.clear().map_err(|source| Error::Io {
            path: "<terminal>".into(),
            source,
        })?;

        self.status = Some(match edit_result {
            Ok(()) => match self.store.load(id) {
                Ok(_) => {
                    self.store.touch_updated_at(id)?;
                    "Session saved.".to_string()
                }
                Err(err) => format!("Not saved — invalid TOML: {err}"),
            },
            Err(err) => format!("Editor failed: {err}"),
        });

        self.reload()
    }
}

pub fn run(config: &Config) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new(config).and_then(|app| app.run(&mut terminal));
    ratatui::restore();
    result
}

fn format_timestamp(dt: time::OffsetDateTime) -> String {
    let format = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]");
    dt.format(&format).unwrap_or_else(|_| dt.to_string())
}
