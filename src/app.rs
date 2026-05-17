use crossterm::event::KeyCode;

/// Which pane currently receives keyboard input
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    #[default]
    OutputPreview,
    QueryInput,
}

/// Core application state
pub struct App {
    /// The raw JSON input loaded from file or stdin
    pub input_json: String,
    /// The current jq query text being edited
    pub query_buf: String,
    /// The pretty-printed output (result of running the query)
    pub output: String,
    /// Error message to display (None if no error)
    pub error: Option<String>,
    /// Scroll offset for the output preview pane
    pub output_scroll: u16,
    /// Which pane has focus
    pub focus: Focus,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Minimum number of lines for the query input area
    pub min_query_lines: u16,
}

impl App {
    /// Create a new App with the given JSON input.
    pub fn new(input_json: String, min_query_lines: u16) -> Self {
        Self {
            input_json,
            query_buf: String::new(),
            output: String::new(),
            error: None,
            output_scroll: 0,
            focus: Focus::default(),
            should_quit: false,
            min_query_lines,
        }
    }

    /// Compute query area height: grows with content, capped at max
    pub fn query_area_height(&self, screen_height: u16) -> u16 {
        let line_count = self.query_buf.lines().count().max(1) as u16;
        let max_h = (screen_height / 4).min(10).max(self.min_query_lines);
        line_count.clamp(self.min_query_lines, max_h) + 2
    }

    /// Toggle focus between OutputPreview and QueryInput
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::OutputPreview => Focus::QueryInput,
            Focus::QueryInput => Focus::OutputPreview,
        };
    }

    /// Handle a character being typed in the query input
    pub fn input_char(&mut self, c: char) {
        self.query_buf.push(c);
    }

    /// Handle backspace in the query input
    pub fn input_backspace(&mut self) {
        self.query_buf.pop();
    }

    /// Handle enter/newline in the query input
    pub fn input_enter(&mut self) {
        self.query_buf.push('\n');
    }

    /// Handle arrow key navigation: Up/Down/PageUp/PageDown
    /// Returns true if the event was handled
    pub fn handle_scroll(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up => {
                self.output_scroll = self.output_scroll.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                self.output_scroll = self.output_scroll.saturating_add(1);
                true
            }
            KeyCode::PageUp => {
                self.output_scroll = self.output_scroll.saturating_sub(10);
                true
            }
            KeyCode::PageDown => {
                self.output_scroll = self.output_scroll.saturating_add(10);
                true
            }
            _ => false,
        }
    }
}
