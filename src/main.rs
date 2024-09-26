mod create_popup;
mod info_popup;

use crate::info_popup::{Checkbox, InfoPopup};
use chrono::Local;
use color_eyre::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::text::Span;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Margin, Rect},
    style::{self, Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader};
use style::palette::tailwind;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: &str = "(I) Info | (Esc) quit";
const ITEM_HEIGHT: usize = 4;
const JSON_FILE_PATH: &str = "data.json";

fn main() -> Result<()> {
    // Enable raw mode to capture all key-presses
    enable_raw_mode()?;
    color_eyre::install()?;

    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);

    // Disable raw mode when the program exits
    disable_raw_mode()?;
    ratatui::restore();
    app_result
}

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    name: String,
    description: String,
    progress: Progress,
    created: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
enum Progress {
    InProgress,
    Waiting,
    Done,
}

impl Progress {
    fn display(&self) -> (Color, String) {
        match self {
            Self::Waiting => (Color::Red, "Waiting".to_string()),
            Self::InProgress => (Color::Yellow, "In Progress".to_string()),
            Self::Done => (Color::Green, "Done".to_string()),
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::Waiting
    }
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            progress: self.progress.clone(),
            created: self.created.clone(),
        }
    }
}

struct App<'a> {
    state: TableState,
    items: Vec<Data>, // Original items loaded from JSON
    filtered_items: Vec<Data>,
    longest_item_lens: (u16, u16, u16, u16), // (name, information, progress, created)
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    show_create: bool,
    show_info: bool,
    input_name: String,
    input_description: String,
    input_focus: InputFocus,
    editing_index: Option<usize>,
    info_popup: InfoPopup<'a>,
    hide_completed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InputFocus {
    Name,
    Description,
}

impl App<'_> {
    fn new() -> Self {
        let data_vec = read_json().unwrap_or_default();
        Self {
            state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&data_vec),
            scroll_state: ScrollbarState::new(data_vec.len().saturating_sub(1) * ITEM_HEIGHT),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            items: data_vec.clone(),  // Original items
            filtered_items: data_vec, // Initially display all items
            show_create: false,
            show_info: false,
            input_name: String::new(),
            input_description: String::new(),
            input_focus: InputFocus::Name,
            editing_index: None,
            info_popup: InfoPopup {
                title: Line::from("Rust-TUI"),
                information: Line::from(vec![
                    Span::styled(
                        "By: Jacob Jørgensen | Github: Zelvios",
                        Style::default().add_modifier(Modifier::ITALIC),
                    ),
                    Span::from(""),
                    Span::styled(
                        "Commands:",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::from("(I) info | (Esc) quit"),
                    Span::from("(A) create new todo | (X) delete todo | (R) edit todo"),
                    Span::from("(↑) move up | (↓) move down | (→) next color | (←) previous color"),
                ]),
                checkboxes: vec![
                    Checkbox {
                        label: "Hide Completed".to_string(),
                        checked: false,
                    },
                    Checkbox {
                        label: "Lock Color".to_string(),
                        checked: false,
                    },
                    Checkbox {
                        label: "....".to_string(),
                        checked: false,
                    },
                ],
                style: Style::default().fg(Color::White),
                selected_checkbox: 0,
            },
            hide_completed: false,
        }
    }

    fn get_filtered_items(&self) -> Vec<&Data> {
        if self.hide_completed {
            self.items
                .iter()
                .filter(|item| item.progress != Progress::Done)
                .collect() // Filter out completed items
        } else {
            self.items.iter().collect() // Keep all items if not hiding
        }
    }
    fn item_matches(item: &Data, selected_item: &Data) -> bool {
        item.name == selected_item.name
            && item.description == selected_item.description
            && item.progress == selected_item.progress
            && item.created == selected_item.created
    }
    fn toggle_info(&mut self) {
        self.show_info = !self.show_info;
    }

    fn handle_info_input(&mut self, key: KeyCode) {
        let checkboxes = &mut self.info_popup.checkboxes; // Access checkboxes from the info_popup

        match key {
            KeyCode::Down => {
                if !checkboxes.is_empty() {
                    self.info_popup.selected_checkbox =
                        (self.info_popup.selected_checkbox + 1) % checkboxes.len();
                }
            }
            KeyCode::Up => {
                if !checkboxes.is_empty() {
                    self.info_popup.selected_checkbox =
                        (self.info_popup.selected_checkbox + checkboxes.len() - 1)
                            % checkboxes.len();
                }
            }
            KeyCode::Left => {
                // Logic for left arrow key
                if self.info_popup.selected_checkbox > 0 {
                    self.info_popup.selected_checkbox -= 1;
                } else {
                    self.info_popup.selected_checkbox = checkboxes.len() - 1; // Wrap around to the last checkbox
                }
            }
            KeyCode::Right => {
                // Logic for right arrow key
                if self.info_popup.selected_checkbox < checkboxes.len() - 1 {
                    self.info_popup.selected_checkbox += 1;
                } else {
                    self.info_popup.selected_checkbox = 0; // Wrap around to the first checkbox
                }
            }
            KeyCode::Enter => {
                if let Some(checkbox) = checkboxes.get_mut(self.info_popup.selected_checkbox) {
                    checkbox.checked = !checkbox.checked;

                    if checkbox.label == "Hide Completed" {
                        self.hide_completed = checkbox.checked;

                        // Update filtered items
                        if self.hide_completed {
                            self.filtered_items = self
                                .items
                                .iter()
                                .filter(|item| item.progress != Progress::Done)
                                .cloned()
                                .collect();
                        } else {
                            self.filtered_items = self.items.clone();
                        }

                        // Recalculate longest item lengths
                        self.longest_item_lens = constraint_len_calculator(&self.filtered_items);

                        // Ensure the selection is valid
                        self.update_selected_index();
                    }
                }
            }
            _ => {}
        }
    }

    fn create_item(&self) -> Data {
        Data {
            name: self.input_name.clone(),
            description: self.input_description.clone(),
            progress: Progress::InProgress,
            created: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    fn toggle_create(&mut self) {
        self.show_create = !self.show_create;

        if self.show_create {
            // Clear fields every time the popup is opened
            self.input_name.clear();
            self.input_description.clear();

            if let Some(index) = self.editing_index {
                // Load the existing item's data if editing
                self.input_name = self.items[index].name.clone();
                self.input_description = self.items[index].description.clone();
            }

            // Set focus to the name field by default
            self.input_focus = InputFocus::Name;
        } else {
            // When closing the popup, reset the editing index and clear the input fields
            self.editing_index = None; // Reset the editing index when closing
            self.input_name.clear();
            self.input_description.clear();
        }
    }

    fn add_item(&mut self) {
        if self.input_name.trim().is_empty() {
            // If name is empty don't add
            return;
        }

        let new_item = self.create_item();
        self.items.push(new_item);

        if let Err(e) = save_json(&self.items) {
            panic!("Error saving JSON: {e}")
        }

        self.toggle_create();
    }

    fn handle_popup_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c)
                if self.input_focus == InputFocus::Name && self.input_name.len() < 50 =>
            {
                self.input_name.push(c);
            }
            KeyCode::Char(c)
                if self.input_focus == InputFocus::Description
                    && self.input_description.len() < 255 =>
            {
                self.input_description.push(c);
            }
            KeyCode::Backspace => match self.input_focus {
                InputFocus::Name => {
                    if !self.input_name.is_empty() {
                        self.input_name.pop();
                    }
                }
                InputFocus::Description => {
                    if !self.input_description.is_empty() {
                        self.input_description.pop();
                    }
                }
            },
            KeyCode::Enter => {
                if self.input_focus == InputFocus::Description {
                    self.add_item(); // Save and close the popup
                } else {
                    self.input_focus = InputFocus::Description;
                }
            }
            KeyCode::Tab if !self.show_info => {
                self.input_focus = match self.input_focus {
                    InputFocus::Name => InputFocus::Description,
                    InputFocus::Description => InputFocus::Name,
                };
            }
            _ => {}
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len().saturating_sub(1)
                } else {
                    i.saturating_sub(1)
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i.saturating_mul(ITEM_HEIGHT));
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    fn delete(&mut self) {
        if let Some(selected) = self.state.selected() {
            if !self.items.is_empty() {
                let filtered_items = self.get_filtered_items();

                // Ensure the selected index is valid for the filtered items
                if selected < filtered_items.len() {
                    let selected_item = filtered_items[selected];

                    if let Some(index) = self.items.iter().position(|item| {
                        item.name == selected_item.name
                            && item.description == selected_item.description
                            && item.progress == selected_item.progress
                            && item.created == selected_item.created
                    }) {
                        self.items.remove(index);

                        let new_index = if index >= self.items.len() {
                            self.items.len().saturating_sub(1)
                        } else {
                            index
                        };
                        self.state.select(Some(new_index));
                        self.scroll_state = self.scroll_state.position(new_index * ITEM_HEIGHT);

                        if let Err(e) = save_json(&self.items) {
                            eprintln!("Error saving JSON: {e}");
                        }
                    }
                }
            }
        }
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if self.show_create {
                        match key.code {
                            KeyCode::Esc => self.show_create = false,
                            KeyCode::Enter => {
                                if self.input_focus == InputFocus::Description {
                                    self.save_item(); // Save and close the popup
                                } else {
                                    self.input_focus = InputFocus::Description;
                                }
                            }
                            _ => {
                                self.handle_popup_input(key.code);
                            }
                        }
                    } else if self.show_info {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('i') => self.show_info = false,
                            _ => self.handle_info_input(key.code), // Handle input for the info popup
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('j') | KeyCode::Down => self.next(),
                            KeyCode::Char('k') | KeyCode::Up => self.previous(),
                            KeyCode::Char('l') | KeyCode::Right => {
                                let lock_color_checked = Option::unwrap_or(
                                    self.info_popup
                                        .checkboxes
                                        .iter()
                                        .find(|checkbox| checkbox.label == "Lock Color")
                                        .map(|checkbox| checkbox.checked),
                                    false,
                                );
                                if !lock_color_checked {
                                    self.next_color();
                                }
                            }
                            KeyCode::Char('h') | KeyCode::Left => {
                                let lock_color_checked = Option::unwrap_or(
                                    self.info_popup
                                        .checkboxes
                                        .iter()
                                        .find(|checkbox| checkbox.label == "Lock Color")
                                        .map(|checkbox| checkbox.checked),
                                    false,
                                );
                                if !lock_color_checked {
                                    self.previous_color();
                                }
                            }
                            KeyCode::Char('x') | KeyCode::Delete => self.delete(),
                            KeyCode::Char('i') => self.toggle_info(),
                            KeyCode::Char('r') => {
                                self.edit_item(); // Call edit item logic
                            }
                            KeyCode::Char('a') => {
                                self.editing_index = None;
                                self.toggle_create(); // Toggle create popup
                            }
                            KeyCode::Char('n') => self.next_progress(),
                            KeyCode::Char('t') => {
                                self.hide_completed = !self.hide_completed; // Toggle hiding
                                self.update_selected_index(); // Ensure the selection is valid
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn update_selected_index(&mut self) {
        let filtered_items = self.get_filtered_items();

        // Check if the currently selected index is still valid
        if let Some(selected) = self.state.selected() {
            if selected >= filtered_items.len() {
                // Reset selection to the first visible item if out of bounds
                self.state.select(Some(0));
            }
        } else {
            // If nothing is selected, default to the first item
            self.state.select(Some(0));
        }
    }
    fn edit_item(&mut self) {
        if let Some(selected) = self.state.selected() {
            let filtered_items = self.get_filtered_items();

            // Ensure the selected index is valid for the filtered items
            if selected < filtered_items.len() {
                let selected_item = filtered_items[selected];

                if let Some(index) = self
                    .items
                    .iter()
                    .position(|item| App::<'_>::item_matches(item, selected_item))
                {
                    self.editing_index = Some(index);
                    self.toggle_create(); // Open the popup
                }
            }
        }
    }

    fn save_item(&mut self) {
        if self.input_name.trim().is_empty() {
            return; // Don't save if the name is empty
        }

        let item = self.create_item();

        if let Some(index) = self.editing_index {
            // If editing an existing item, update it
            self.items[index] = item;
        } else {
            // Otherwise, add a new item
            self.items.push(item);
        }

        if let Err(e) = save_json(&self.items) {
            eprintln!("Error saving JSON: {e}");
        }

        // Reset the editing index and close the popup
        self.editing_index = None;
        self.toggle_create();
    }

    fn next_progress(&mut self) {
        if let Some(selected) = self.state.selected() {
            // Create a filtered list based on hide_completed flag
            let filtered_items: Vec<&Data> = self.get_filtered_items();

            // Ensure the selected index is valid for the filtered items
            if selected < filtered_items.len() {
                // Get the selected item from the filtered list
                let selected_item = filtered_items[selected];

                // Find the index in the original items to compare
                if let Some(original_index) = self
                    .items
                    .iter()
                    .position(|item| App::<'_>::item_matches(item, selected_item))
                {
                    // Update the progress of the original item
                    let item = &mut self.items[original_index];
                    item.progress = match item.progress {
                        Progress::InProgress => Progress::Waiting,
                        Progress::Waiting => Progress::Done,
                        Progress::Done => Progress::InProgress,
                    };

                    if let Err(e) = save_json(&self.items) {
                        eprintln!("Error saving JSON: {e}");
                    }
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let vertical = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]);
        let rects = vertical.split(frame.size());

        self.set_colors();
        self.render_table(frame, rects[0]);
        self.render_scrollbar(frame, rects[0]);
        self.render_footer(frame, rects[1]);

        // Rendering the creation popup
        if self.show_create {
            let create = create_popup::CreatePopup {
                name: self.input_name.clone(),
                description: self.input_description.clone(),
                style: Style::default().fg(Color::White),
            };
            create.render(
                popup_area(area, area.width / 2, area.height),
                frame.buffer_mut(),
                self.input_focus,
                self.colors.selected_style_fg,
            );
        }

        // Rendering the info popup
        if self.show_info {
            self.info_popup.render(
                popup_area(area, area.width / 2, area.height),
                frame.buffer_mut(),
                self.colors.selected_style_fg,
            );
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_style_fg);

        let header = ["Name", "Description", "Progress", "Created"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        // Filter items based on hide_completed flag
        let filtered_items: Vec<&Data> = if self.hide_completed {
            self.items
                .iter()
                .filter(|item| item.progress != Progress::Done)
                .collect()
        } else {
            self.items.iter().collect()
        };

        if let Some(selected_index) = self.state.selected() {
            if selected_index >= filtered_items.len() {
                self.state.select(Some(0));
            }
        }

        let rows = filtered_items.iter().enumerate().map(|(i, data)| {
            let color = if i % 2 == 0 {
                self.colors.normal_row_color
            } else {
                self.colors.alt_row_color
            };
            let progress_display = data.progress.display(); // Get the display value for progress
            let progress_text = progress_display.1; // Extract the text
            let progress_color = progress_display.0; // Extract the color

            // Wrap both name and information if they exceed the specified lengths
            let wrapped_name = wrap_text(&data.name, 22);
            let wrapped_description = wrap_text(&data.description, 42);

            Row::new(vec![
                Cell::from(Text::from(wrapped_name)),
                Cell::from(Text::from(wrapped_description)),
                Cell::from(Text::from(progress_text).style(Style::new().fg(progress_color))),
                Cell::from(Text::from(data.created.clone())),
            ])
            .style(Style::new().fg(self.colors.row_fg).bg(color))
            .height(u16::try_from(ITEM_HEIGHT).expect("REASON"))
        });

        let t = Table::new(
            rows,
            [
                Constraint::Length(22),
                Constraint::Length(42),
                Constraint::Min(self.longest_item_lens.2),
                Constraint::Min(self.longest_item_lens.3),
            ],
        )
        .header(header)
        .highlight_style(selected_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            " █ ".into(),
            " █ ".into(),
            "".into(),
        ]))
        .bg(self.colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(t, area, &mut self.state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Line::from(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );
        frame.render_widget(info_footer, area);
    }
}

fn wrap_text(text: &str, max_len: usize) -> String {
    text.chars()
        .collect::<Vec<_>>() // Collect into a vector of chars for easier manipulation
        .chunks(max_len) // Split into chunks of max_len
        .map(|chunk| chunk.iter().collect::<String>()) // Collect each chunk back into a string
        .collect::<Vec<_>>() // Collect into a vector of lines
        .join("\n")
}

fn constraint_len_calculator(items: &[Data]) -> (u16, u16, u16, u16) {
    let name_len = items
        .iter()
        .map(|data| u16::try_from(data.name.len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    let description_len = items
        .iter()
        .flat_map(|data| data.description.lines())
        .map(|line| u16::try_from(line.len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    let progress_len = items
        .iter()
        .map(|data| u16::try_from(format!("{:?}", data.progress).len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    let created_len = items
        .iter()
        .map(|data| u16::try_from(data.created.len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    (name_len, description_len, progress_len, created_len)
}

fn read_json() -> io::Result<Vec<Data>> {
    let file = File::open(JSON_FILE_PATH).map_err(|e| {
        eprintln!("Error opening file: {e}");
        e
    })?;
    let reader = BufReader::new(file);
    let data: Vec<Data> = serde_json::from_reader(reader).map_err(|e| {
        eprintln!("Error parsing JSON: {e}");
        e
    })?;
    Ok(data)
}

fn save_json(data: &[Data]) -> io::Result<()> {
    let file = File::create(JSON_FILE_PATH)?;
    serde_json::to_writer_pretty(file, data)?;
    Ok(())
}

fn popup_area(area: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
