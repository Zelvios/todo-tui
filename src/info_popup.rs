use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Color, Line, Style, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};
use tui_big_text::{BigText, PixelSize};
#[derive(Debug, Default)]
pub struct Checkbox {
    pub checked: bool,
    pub label: String,
}
#[derive(Default)]
pub struct InfoPopup<'a> {
    pub title: Line<'a>,
    pub information: Line<'a>,
    pub checkboxes: Vec<Checkbox>,
    pub selected_checkbox: usize,
    pub style: Style,
}
impl InfoPopup<'_> {
    pub fn render(&self, area: Rect, buf: &mut Buffer, selected_style_fg: Color) {
        Clear.render(area, buf);

        let border_color = selected_style_fg;
        let top_offset = 2;
        let name_area = Rect::new(area.x, area.y + top_offset, area.width, 4);

        // Create title
        let big_text = BigText::builder()
            .pixel_size(PixelSize::HalfHeight)
            .style(Style::new().fg(selected_style_fg))
            .lines(vec![self.title.clone()])
            .centered()
            .build();

        // Render the title
        big_text.render(name_area, buf);

        let columns: usize = 3;
        let rows = (self.checkboxes.len() + columns - 1) / columns; // Calculate rows dynamically
        #[allow(clippy::cast_possible_truncation)]
        let checkbox_width = area.width / columns as u16;
        let checkbox_height: u16 = 1;

        // Render the checkboxes
        for (i, checkbox) in self.checkboxes.iter().enumerate() {
            let row: usize = i / columns;
            let col: usize = i % columns;

            #[allow(clippy::cast_possible_truncation)]
            let checkbox_area = Rect::new(
                area.x + (col as u16) * checkbox_width,
                area.y + top_offset + 5 + row as u16 * (checkbox_height + 1),
                checkbox_width,
                checkbox_height,
            );

            let checkbox_label = if checkbox.checked {
                format!("[✔] {}", checkbox.label)
            } else {
                format!("[ ] {}", checkbox.label)
            };

            let style = if i == self.selected_checkbox {
                Style::new().fg(selected_style_fg)
            } else {
                self.style
            };

            // Render the checkbox label
            buf.set_string(
                checkbox_area.x + 1,
                checkbox_area.y + 1,
                checkbox_label,
                style,
            );
        }

        #[allow(clippy::cast_possible_truncation)]
        let description_area = Rect::new(
            area.x,
            area.y + top_offset + 5 + (rows as u16 * (checkbox_height + 1)) + 1,
            area.width,
            area.height - (top_offset + 5 + (rows as u16 * (checkbox_height + 1)) + 1),
        );

        // Split the information into spans, each span in information will be on a new line
        let mut text = Text::default();
        for span in &self.information.spans {
            text.lines.push(Line::from(vec![span.clone()]));
        }

        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(
                Block::new()
                    .title("Information")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .render(description_area, buf);
    }
}
