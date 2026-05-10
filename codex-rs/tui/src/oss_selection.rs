use std::io;

use crate::key_hint;
use crate::key_hint::KeyBinding;
use crate::key_hint::KeyBindingListExt;
use codex_http_client::HttpClient;
use codex_http_client::HttpClientBuilder;
use codex_model_provider_info::DEFAULT_LMSTUDIO_PORT;
use codex_model_provider_info::DEFAULT_OLLAMA_PORT;
use codex_model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
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
use ratatui::layout::Direction;
use ratatui::layout::HorizontalAlignment;
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

// This startup wizard runs before the main TUI runtime keymap is available, so
// it mirrors the built-in horizontal list defaults instead of reading config.
// The shared matcher still covers raw C0 Ctrl-H/Ctrl-L terminal reports.
const MOVE_LEFT_KEYS: [KeyBinding; 2] = [
    key_hint::plain(KeyCode::Left),
    key_hint::ctrl(KeyCode::Char('h')),
];
const MOVE_RIGHT_KEYS: [KeyBinding; 2] = [
    key_hint::plain(KeyCode::Right),
    key_hint::ctrl(KeyCode::Char('l')),
];

pub struct OssSelectionWidget<'a> {
    select_options: &'a Vec<SelectOption>,
    confirmation_prompt: Paragraph<'a>,

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
        match key_event {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_decision("__CANCELLED__".to_string());
            }
            _ if MOVE_LEFT_KEYS.is_pressed(key_event) => {
                self.selected_option = (self.selected_option + self.select_options.len() - 1)
                    % self.select_options.len();
            }
            _ if MOVE_RIGHT_KEYS.is_pressed(key_event) => {
                self.selected_option = (self.selected_option + 1) % self.select_options.len();
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                let opt = &self.select_options[self.selected_option];
                self.send_decision(opt.provider_id.to_string());
            }
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                self.send_decision(LMSTUDIO_OSS_PROVIDER_ID.to_string());
            }
            KeyEvent { code, .. } => {
                let other = code;
                let normalized = Self::normalize_keycode(other);
                if let Some(opt) = self
                    .select_options
                    .iter()
                    .find(|opt| Self::normalize_keycode(opt.key) == normalized)
                {
                    self.send_decision(opt.provider_id.to_string());
                }
                self.adjust_scroll();
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.filtered_indices.len() {
                    self.selected_index += 1;
                }
                self.adjust_scroll();
            }
            KeyCode::PageUp => {
                self.selected_index = self.selected_index.saturating_sub(10);
                self.adjust_scroll();
            }
            KeyCode::PageDown => {
                self.selected_index =
                    (self.selected_index + 10).min(self.filtered_indices.len().saturating_sub(1));
                self.adjust_scroll();
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.adjust_scroll();
            }
            KeyCode::End => {
                self.selected_index = self.filtered_indices.len().saturating_sub(1);
                self.adjust_scroll();
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

    fn adjust_scroll(&mut self) {
        let visible_count = 20; // Approximate visible items
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_count {
            self.scroll_offset = self.selected_index.saturating_sub(visible_count - 1);
        }
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
        let prompt_height = self.get_confirmation_prompt_height(area.width);
        let [prompt_chunk, response_chunk] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(prompt_height), Constraint::Min(0)])
            .areas(area);

        let lines: Vec<Line> = self
            .select_options
            .iter()
            .enumerate()
            .map(|(idx, opt)| {
                let style = if idx == self.selected_option {
                    Style::new().bg(Color::Cyan).fg(Color::Black)
                } else {
                    Style::new().bg(Color::DarkGray)
                };
                opt.label
                    .clone()
                    .alignment(HorizontalAlignment::Center)
                    .style(style)
            })
            .collect();

        let [title_area, button_area, description_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
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

        // Use the pre-computed scroll_offset (updated by adjust_scroll in handle_select_key)
        let adjusted_scroll = self.scroll_offset;

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

pub(crate) struct OssProviderSelection {
    pub(crate) provider: String,
    pub(crate) manually_selected: bool,
}

pub async fn select_oss_provider() -> io::Result<OssProviderSelection> {
    // These probes intentionally bypass proxy discovery because both targets are
    // hardcoded plaintext loopback endpoints. Preserve the legacy custom-CA fallback so an
    // invalid inherited certificate bundle cannot prevent best-effort provider detection.
    #[allow(deprecated)]
    let client = HttpClientBuilder::new().build_direct_with_custom_ca_fallback();

    // Check provider statuses first
    let lmstudio_status = check_lmstudio_status(&client).await;
    let ollama_status = check_ollama_status(&client).await;

    // Autoselect if only one is running
    match (&lmstudio_status, &ollama_status) {
        (ProviderStatus::Running, ProviderStatus::NotRunning) => {
            let provider = LMSTUDIO_OSS_PROVIDER_ID.to_string();
            return Ok(OssProviderSelection {
                provider,
                manually_selected: false,
            });
        }
        (ProviderStatus::NotRunning, ProviderStatus::Running) => {
            let provider = OLLAMA_OSS_PROVIDER_ID.to_string();
            return Ok(OssProviderSelection {
                provider,
                manually_selected: false,
            });
        }
        _ => {
            // Both running or both not running - show UI
        }
    }

    let mut widget = OssSelectionWidget::new(lmstudio_status, ollama_status)?;

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
            break Ok(OssProviderSelection {
                provider: selection,
                manually_selected: true,
            });
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

async fn check_lmstudio_status(client: &HttpClient) -> ProviderStatus {
    match check_port_status(client, DEFAULT_LMSTUDIO_PORT).await {
        Ok(true) => ProviderStatus::Running,
        Ok(false) => ProviderStatus::NotRunning,
        Err(_) => ProviderStatus::Unknown,
    }
}

async fn check_ollama_status(client: &HttpClient) -> ProviderStatus {
    match check_port_status(client, DEFAULT_OLLAMA_PORT).await {
        Ok(true) => ProviderStatus::Running,
        Ok(false) => ProviderStatus::NotRunning,
        Err(_) => ProviderStatus::Unknown,
    }
}

async fn check_port_status(client: &HttpClient, port: u16) -> io::Result<bool> {
    let url = format!("http://localhost:{port}");

    match client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false), // Connection failed = not running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_h_l_move_provider_selection() {
        let mut widget = OssSelectionWidget::new(ProviderStatus::Unknown, ProviderStatus::Unknown)
            .expect("widget should initialize");

        assert_eq!(widget.selected_option, 0);
        widget.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL));
        assert_eq!(widget.selected_option, 1);
        widget.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(widget.selected_option, 0);
    }

    #[tokio::test]
    async fn localhost_probe_succeeds_with_invalid_inherited_ca_bundle() {
        const CHILD_ENV: &str = "CODEX_OSS_SELECTION_INVALID_CA_TEST_CHILD";

        if std::env::var_os(CHILD_ENV).is_none() {
            let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
            let invalid_ca_path = temp_dir.path().join("invalid-ca.pem");
            std::fs::write(&invalid_ca_path, "not a PEM certificate")
                .expect("invalid CA fixture should be written");

            for ca_env in ["CODEX_CA_CERTIFICATE", "SSL_CERT_FILE"] {
                let output = std::process::Command::new(
                    std::env::current_exe().expect("test executable should be available"),
                )
                .arg("--exact")
                .arg("oss_selection::tests::localhost_probe_succeeds_with_invalid_inherited_ca_bundle")
                .arg("--nocapture")
                .env_remove("CODEX_CA_CERTIFICATE")
                .env_remove("SSL_CERT_FILE")
                .env(ca_env, &invalid_ca_path)
                .env(CHILD_ENV, "1")
                .output()
                .expect("isolated CA subprocess should run");

                assert!(
                    output.status.success(),
                    "localhost probe failed with invalid {ca_env}\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                );
            }
            return;
        }

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        #[allow(deprecated)]
        let client = HttpClientBuilder::new().build_direct_with_custom_ca_fallback();
        assert!(
            check_port_status(&client, server.address().port())
                .await
                .expect("localhost provider probe should complete")
        );
    }
}
