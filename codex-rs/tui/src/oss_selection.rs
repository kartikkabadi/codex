use std::io;

use codex_model_provider_info::built_in_model_providers;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::{self};
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
/// A single provider row shown in the scrollable list.
struct ProviderItem {
    id: String,
    name: String,
    protocol: String,
    auth_type: String,
}

pub struct OssSelectionWidget {
    providers: Vec<ProviderItem>,
    filtered_indices: Vec<usize>,

    /// Currently selected index in *filtered* list.
    selected_index: usize,

    /// Scroll offset for the provider list.
    scroll_offset: usize,

    /// Search/filter text.
    filter_text: String,

    /// Set to `true` once a decision has been sent.
    done: bool,

    selection: Option<String>,
}

impl OssSelectionWidget {
    fn new() -> io::Result<Self> {
        let built_in = built_in_model_providers(None);
        let mut providers: Vec<ProviderItem> = built_in
            .into_iter()
            .map(|(id, info)| ProviderItem {
                id: id.clone(),
                name: info.name.clone(),
                protocol: info.protocol_id.clone(),
                auth_type: format_auth(&info),
            })
            .collect();

        // Sort by name for consistent display.
        providers.sort_by(|a, b| a.name.cmp(&b.name));

        let filtered_indices: Vec<usize> = (0..providers.len()).collect();

        Ok(Self {
            providers,
            filtered_indices,
            selected_index: 0,
            scroll_offset: 0,
            filter_text: String::new(),
            done: false,
            selection: None,
        })
    }

    fn apply_filter(&mut self) {
        let query = self.filter_text.to_lowercase();
        self.filtered_indices = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.name.to_lowercase().contains(&query)
                    || p.id.to_lowercase().contains(&query)
                    || p.protocol.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn selected_provider_id(&self) -> Option<String> {
        self.filtered_indices
            .get(self.selected_index)
            .map(|&idx| self.providers[idx].id.clone())
    }

    /// Process a `KeyEvent` coming from crossterm.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<String> {
        if key.kind == KeyEventKind::Press {
            self.handle_select_key(key);
        }
        if self.done {
            self.selection.clone()
        } else {
            None
        }
    }

    fn handle_select_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('c')
                if key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.send_decision("__CANCELLED__".to_string());
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.filtered_indices.len() {
                    self.selected_index += 1;
                }
            }
            KeyCode::PageUp => {
                self.selected_index = self.selected_index.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.selected_index =
                    (self.selected_index + 10).min(self.filtered_indices.len().saturating_sub(1));
            }
            KeyCode::Home => {
                self.selected_index = 0;
            }
            KeyCode::End => {
                self.selected_index = self.filtered_indices.len().saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(id) = self.selected_provider_id() {
                    self.send_decision(id);
                }
            }
            KeyCode::Esc => {
                self.send_decision("__CANCELLED__".to_string());
            }
            KeyCode::Char(c)
                if !key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                    && !key_event
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::ALT) =>
            {
                self.filter_text.push(c);
                self.apply_filter();
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn send_decision(&mut self, selection: String) {
        self.selection = Some(selection);
        self.done = true;
    }

    /// Returns `true` once the user has made a decision and the widget no
    /// longer needs to be displayed.
    #[allow(dead_code)]
    pub fn is_complete(&self) -> bool {
        self.done
    }
}

impl WidgetRef for OssSelectionWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let [header_area, list_area, footer_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .areas(area);

        // Header
        let header_lines = vec![
            Line::from(vec!["? ".fg(Color::Blue), "Select a model provider".bold()]),
            Line::from(vec![
                "  ".into(),
                format!("{} providers available", self.providers.len()).dim(),
            ]),
            Line::from(vec![
                "  Filter: ".dim(),
                if self.filter_text.is_empty() {
                    "(type to search)".dim()
                } else {
                    self.filter_text.clone().into()
                },
            ]),
        ];
        Paragraph::new(header_lines)
            .wrap(Wrap { trim: false })
            .render(header_area, buf);

        // List
        let visible_count = list_area.height as usize;
        let total = self.filtered_indices.len();

        // Adjust scroll offset to keep selection visible.
        let adjusted_scroll = if self.selected_index < self.scroll_offset {
            self.selected_index
        } else if self.selected_index >= self.scroll_offset + visible_count {
            self.selected_index.saturating_sub(visible_count - 1)
        } else {
            self.scroll_offset
        };

        let mut list_lines: Vec<Line> = Vec::new();
        for i in 0..visible_count {
            let idx = adjusted_scroll + i;
            if idx >= total {
                break;
            }
            let provider_idx = self.filtered_indices[idx];
            let provider = &self.providers[provider_idx];
            let is_selected = idx == self.selected_index;

            let caret = if is_selected { ">" } else { " " };
            let name_style = if is_selected {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            };
            let meta_style = Style::new().fg(Color::DarkGray);

            list_lines.push(Line::from(vec![
                Span::styled(format!("{caret} "), name_style),
                Span::styled(provider.name.clone(), name_style),
                Span::styled(
                    format!(
                        "  [{} | {} | {}]",
                        provider.id, provider.protocol, provider.auth_type
                    ),
                    meta_style,
                ),
            ]));
        }

        // Scrollbar indicator
        if total > visible_count {
            let scroll_info = format!(
                "  Showing {}-{} of {}",
                adjusted_scroll + 1,
                (adjusted_scroll + visible_count).min(total),
                total
            );
            list_lines.push(Line::from(""));
            list_lines.push(Line::from(scroll_info).dim());
        }

        Paragraph::new(list_lines)
            .wrap(Wrap { trim: false })
            .render(list_area, buf);

        // Footer
        let footer_lines = vec![
            Line::from(vec![
                "  ↑/↓ move • Enter select • Esc cancel • Type to filter".dim(),
            ]),
            Line::from(vec!["  PgUp/PgDn page • Home/End jump".dim()]),
        ];
        Paragraph::new(footer_lines)
            .wrap(Wrap { trim: false })
            .render(footer_area, buf);
    }
}

fn format_auth(info: &codex_model_provider_info::ModelProviderInfo) -> String {
    if info.requires_openai_auth {
        "OpenAI auth".to_string()
    } else if info.aws.is_some() {
        "AWS".to_string()
    } else if info.env_key.is_some() {
        format!("Env: {}", info.env_key.as_ref().unwrap())
    } else if info.auth.is_some() {
        "API key".to_string()
    } else {
        "None".to_string()
    }
}

pub async fn select_oss_provider(_codex_home: &std::path::Path) -> io::Result<String> {
    let mut widget = OssSelectionWidget::new()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = loop {
        terminal.draw(|f| {
            (&widget).render_ref(f.area(), f.buffer_mut());
        })?;

        if let Event::Key(key_event) = event::read()?
            && let Some(selection) = widget.handle_key_event(key_event)
        {
            break Ok(selection);
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}
