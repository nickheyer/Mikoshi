use std::collections::VecDeque;
use sdl2::pixels::Color;
use std::cmp::min;

const MAX_HISTORY_LINES: usize = 1000;
const MAX_COMMAND_HISTORY: usize = 100;

#[derive(Clone, Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    fn new(line: usize, column: usize) -> Self {
        let pos = Position { line, column };
        Self {
            start: pos.clone(),
            end: pos,
        }
    }

    pub fn normalize(&self) -> (Position, Position) {
        if self.start.line < self.end.line || 
           (self.start.line == self.end.line && self.start.column <= self.end.column) {
            (Position { line: self.start.line, column: self.start.column }, 
             Position { line: self.end.line, column: self.end.column })
        } else {
            (Position { line: self.end.line, column: self.end.column },
             Position { line: self.start.line, column: self.start.column })
        }
    }
}

pub struct TerminalState {
    history: VecDeque<String>,
    current_input: String,
    cursor_position: usize,
    settings: TerminalSettings,
    viewport: TerminalViewport,
    selection: Option<Selection>,
    command_history: VecDeque<String>,  // Changed from Vec to VecDeque
    command_index: Option<usize>,
    visible_lines: usize,
}

pub struct TerminalViewport {
    pub offset: usize,
    pub visible_lines: usize,
    pub line_height: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub struct TerminalSettings {
    pub font_size: u16,
    pub colors: TerminalColors,
    pub prompt: String,
}

#[derive(Clone)]
pub struct TerminalColors {
    pub text: Color,
    pub background: Color,
    pub selection: Color,
    pub cursor: Color,
    pub input: Color,
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self {
            text: Color::RGB(0, 255, 170),
            background: Color::RGB(10, 10, 30),
            selection: Color::RGB(70, 70, 150),
            cursor: Color::RGB(255, 255, 255),
            input: Color::RGB(200, 200, 255),
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            font_size: 16,
            colors: TerminalColors::default(),
            prompt: "$ ".to_string(),
        }
    }
}

impl TerminalState {
    pub fn new(width: u32, height: u32, line_height: u32) -> Self {
        let visible_lines = (height / line_height) as usize;
        Self {
            history: VecDeque::with_capacity(MAX_HISTORY_LINES),
            current_input: String::new(),
            cursor_position: 0,
            settings: TerminalSettings::default(),
            viewport: TerminalViewport {
                offset: 0,
                visible_lines,
                line_height,
                width,
                height,
            },
            selection: None,
            command_history: VecDeque::with_capacity(MAX_COMMAND_HISTORY),
            command_index: None,
            visible_lines,
        }
    }

    // Selection handling
    pub fn start_selection(&mut self, line: usize, column: usize) {
        let content = self.get_visible_content();
        if line >= content.len() {
            return;
        }

        let line_content = &content[line].0;
        let bounded_column = min(column, line_content.len());
        self.selection = Some(Selection::new(line, bounded_column));
    }

    pub fn update_selection(&mut self, line: usize, column: usize) {
        let content = self.get_visible_content();
        if let Some(selection) = &mut self.selection {
            let bounded_line = min(line, content.len().saturating_sub(1));
            let line_content = &content[bounded_line].0;
            let bounded_column = min(column, line_content.len());
            println!("UPDATED SELECTION: {:#?}", self.current_input);
            selection.end = Position {
                line: bounded_line,
                column: bounded_column,
            };
        }
    }
    

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn get_selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    pub fn get_selected_text(&self) -> String {
        if let Some(selection) = &self.selection {
            self.get_text_from_selection(selection)
        } else {
            String::new()
        }
    }

    fn get_text_from_selection(&self, selection: &Selection) -> String {
        let visible_content = self.get_visible_content();
        let (start_pos, end_pos) = selection.normalize();

        let mut result = String::new();
        for (i, (line, _)) in visible_content.iter().enumerate() {
            if i < start_pos.line || i > end_pos.line {
                continue;
            }

            let line_start = if i == start_pos.line { start_pos.column } else { 0 };
            let line_end = if i == end_pos.line {
                min(end_pos.column, line.len())
            } else {
                line.len()
            };

            if !result.is_empty() {
                result.push('\n');
            }

            if line_start < line.len() {
                result.push_str(&line[line_start..line_end]);
            }
        }
        result
    }

    // Command history handling
    pub fn handle_key_up(&mut self) {
        println!("KEY UP: {:#?}", self.current_input);
        if self.command_history.is_empty() {
            return;
        }
        let current_index = self.command_index.get_or_insert(self.command_history.len());
        if *current_index > 0 {
            *current_index -= 1;
            self.current_input = self.command_history[*current_index].clone();
            self.cursor_position = self.current_input.len();
        }
    }
    
    pub fn handle_key_down(&mut self) {
        println!("KEY DOWN: {:#?}", self.current_input);
        if let Some(ref mut index) = self.command_index {
            if *index < self.command_history.len().saturating_sub(1) {
                *index += 1;
                self.current_input = self.command_history[*index].clone();
            } else {
                self.current_input.clear();
                self.command_index = None;
            }
            self.cursor_position = self.current_input.len();
        }
    }

    // Input handling
    pub fn add_input(&mut self, input: &str) {
        // Reset command_index when typing after history navigation
        self.command_index = None;
        self.current_input.push_str(input);
        self.cursor_position += input.len();
        println!("CURRENT INPUT: {:#?}", self.current_input);
    }

    pub fn handle_backspace(&mut self) {
        
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.current_input.remove(self.cursor_position);
            // Reset command_index to break out of history mode
            self.command_index = None;
            println!("AFTER: {:#?}", self.current_input);
        }
    }

    pub fn commit_input(&mut self) {
        let input = std::mem::take(&mut self.current_input);
        if !input.is_empty() {
            self.command_history.push_back(input.clone());
            if self.command_history.len() > MAX_COMMAND_HISTORY {
                self.command_history.pop_front();
            }
        }
        
        self.add_output(&format!("{}{}\n", self.settings.prompt, input));
        println!("COMMITTED INPUT: {:#?}", self.current_input);
        self.cursor_position = 0;
        self.command_index = None; // Reset history navigation
        self.clear_selection();
    }

    // Output and viewport handling
    pub fn clear(&mut self) {
        self.history.clear();
        self.viewport.offset = 0;
        self.clear_selection();
    }

    pub fn add_output(&mut self, output: &str) {
        if output.contains("\x1b[H\x1b[2J") || output.contains("\x0C") {
            self.clear();
            return;
        }

        for line in output.lines() {
            if self.history.len() >= MAX_HISTORY_LINES {
                self.history.pop_front();
            }
            self.history.push_back(line.to_string());
        }

        if self.viewport.offset == 0 {
            self.scroll_to_bottom();
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = self.history.len()
            .saturating_sub(self.viewport.visible_lines.saturating_sub(1));
        self.viewport.offset = min(self.viewport.offset + lines, max_scroll);
        self.clear_selection();
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.viewport.offset = self.viewport.offset.saturating_sub(lines);
        self.clear_selection();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.viewport.offset = 0;
        self.clear_selection();
    }

    // Getters and utility functions
    fn get_visible_range(&self) -> (usize, usize) {
        let total_lines = self.history.len();
        let visible_lines = self.viewport.visible_lines.saturating_sub(1);
        let start = total_lines.saturating_sub(visible_lines + self.viewport.offset);
        let end = total_lines.saturating_sub(self.viewport.offset);
        (start, end)
    }

    pub fn get_visible_content(&self) -> Vec<(String, Color)> {
        let mut result = Vec::new();
        let (start, end) = self.get_visible_range();

        for line in self.history.range(start..end) {
            result.push((line.clone(), self.settings.colors.text));
        }

        if self.viewport.offset == 0 {
            result.push((
                format!("{}{}", self.settings.prompt, self.current_input),
                self.settings.colors.input,
            ));
        }

        result
    }

    pub fn get_viewport(&self) -> &TerminalViewport {
        &self.viewport
    }

    pub fn get_settings(&self) -> &TerminalSettings {
        &self.settings
    }
}

