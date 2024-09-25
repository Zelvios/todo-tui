use crate::InputFocus;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Color, Style, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

#[derive(Default)]
pub struct CreatePopup {
    pub name: String,
    pub description: String,
    pub style: Style,
}

impl CreatePopup {
    pub fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        input_focus: InputFocus,
        selected_style_fg: Color,
    ) {
        Clear.render(area, buf);

        let name_border_color = if input_focus == InputFocus::Name {
            selected_style_fg
        } else {
            Color::White
        };
        let description_border_color = if input_focus == InputFocus::Description {
            selected_style_fg
        } else {
            Color::White
        };

        let name_area = Rect::new(area.x, area.y, area.width, 3);
        Paragraph::new(Text::from(self.name))
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(
                Block::new()
                    .title("Name")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(name_border_color)),
            )
            .render(name_area, buf);

        let description_area = Rect::new(area.x, area.y + 4, area.width, area.height - 4);
        Paragraph::new(Text::from(self.description))
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(
                Block::new()
                    .title("Description")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(description_border_color)),
            )
            .render(description_area, buf);
    }
}
