use iced::widget::{container, row, text, Space};
use iced::{Alignment, Element, Length, Padding};

use crate::app::{CadeSimuApp, Message};
use crate::theme::{self, tokens, MONO, STATUS_H};

pub fn view_statusbar<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let sep = || text("|").size(11).color(t.border);

    let snap_label = if app.show_grid {
        "snap ativo"
    } else {
        "snap desligado"
    };
    let zoom_pct = (app.zoom * 31.25).round() as i32;

    container(
        row![
            row![
                text("Coordenadas:").size(11).color(t.text2),
                text(format!(
                    "X: {:3}, Y: {:3}",
                    app.cursor_world.0, app.cursor_world.1
                ))
                .size(11)
                .font(MONO)
                .color(t.text),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            sep(),
            row![
                text("Grelha:").size(11).color(t.text2),
                text(format!("5×5 ({snap_label})"))
                    .size(11)
                    .font(MONO)
                    .color(t.text),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            sep(),
            text("Folha: DIN A4 (1040 × 720 px)")
                .size(11)
                .color(t.text2),
            sep(),
            {
                let status_color = if app.sim_running { t.primary } else { t.text2 };
                let status_dot = container(Space::new(6, 6)).style(theme::status_dot(status_color));
                row![
                    status_dot,
                    text(&app.status_msg).size(11).color(status_color)
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill)
            },
            sep(),
            row![
                text("Zoom:").size(11).color(t.text2),
                text(format!("{zoom_pct}%"))
                    .size(11)
                    .font(MONO)
                    .color(t.text),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            sep(),
            text(app.save_format.extension().to_uppercase())
                .size(11)
                .font(MONO)
                .color(t.text2),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(theme::bar)
    .height(STATUS_H)
    .padding(Padding::from([0.0, 10.0]))
    .width(Length::Fill)
    .into()
}
