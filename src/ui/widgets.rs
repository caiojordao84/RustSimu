use iced::widget::{text, tooltip};
use iced::Element;

use crate::theme::{self, UI};

pub fn app_tooltip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    tip: impl Into<String>,
    position: tooltip::Position,
) -> Element<'a, M> {
    tooltip(content, text(tip.into()).size(12).font(UI), position)
        .style(theme::tooltip)
        .padding(6.0)
        .gap(4.0)
        .into()
}
