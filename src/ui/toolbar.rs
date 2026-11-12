use iced::widget::{button, container, pick_list, row, scrollable, tooltip};
use iced::{Alignment, Element, Length, Padding};

use crate::app::{CadeSimuApp, Message};
use crate::canvas_view::EditorTool;
use crate::theme::{self, tokens, TOOL_H};
use crate::ui::widgets::app_tooltip;

pub fn view_toolbar<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let icon_btn = |icon: crate::icons::Icon, tip: &'static str, msg: Message| {
        app_tooltip(
            button(
                container(crate::icons::view_icon(icon, 16, t.text))
                    .width(20)
                    .height(20)
                    .center_x(20)
                    .center_y(20),
            )
            .style(theme::tool)
            .padding(Padding::from([3.0, 5.0]))
            .on_press(msg),
            tip,
            tooltip::Position::Bottom,
        )
    };

    let icon_toggle = |icon: crate::icons::Icon, tip: &'static str, active: bool, msg: Message| {
        let s = if active {
            theme::tool_active
        } else {
            theme::tool
        };
        let col = if active { t.primary } else { t.text };
        app_tooltip(
            button(
                container(crate::icons::view_icon(icon, 16, col))
                    .width(20)
                    .height(20)
                    .center_x(20)
                    .center_y(20),
            )
            .style(s)
            .padding(Padding::from([3.0, 5.0]))
            .on_press(msg),
            tip,
            tooltip::Position::Bottom,
        )
    };

    let file_group = container(
        row![
            icon_btn(
                crate::icons::Icon::New,
                "Novo (Ctrl+N)",
                Message::NewDocument
            ),
            icon_btn(
                crate::icons::Icon::Open,
                "Abrir (Ctrl+O)",
                Message::OpenFileDialog
            ),
            icon_btn(
                crate::icons::Icon::Save,
                "Salvar (Ctrl+S)",
                Message::SaveDocument
            ),
            icon_btn(
                crate::icons::Icon::SaveAs,
                "Salvar Como (Ctrl+Shift+S)",
                Message::SaveFileDialog
            ),
            icon_btn(
                crate::icons::Icon::Close,
                "Fechar (Ctrl+W)",
                Message::CloseDocument
            ),
            icon_btn(
                crate::icons::Icon::Print,
                "Imprimir (Ctrl+P)",
                Message::PrintDocument
            ),
            icon_btn(
                crate::icons::Icon::ExportPdf,
                "Exportar Esquema (Ctrl+Shift+E)",
                Message::OpenExportModal
            ),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let edit_group = container(
        row![
            icon_btn(crate::icons::Icon::Undo, "Desfazer (Ctrl+Z)", Message::Undo),
            icon_btn(
                crate::icons::Icon::Redo,
                "Refazer (Ctrl+Shift+Z)",
                Message::Redo
            ),
            icon_btn(crate::icons::Icon::Cut, "Cortar (Ctrl+X)", Message::Cut),
            icon_btn(crate::icons::Icon::Copy, "Copiar (Ctrl+C)", Message::Copy),
            icon_btn(crate::icons::Icon::Paste, "Colar (Ctrl+V)", Message::Paste),
            icon_btn(
                crate::icons::Icon::Duplicate,
                "Duplicar (Ctrl+D)",
                Message::Duplicate
            ),
            icon_btn(
                crate::icons::Icon::Delete,
                "Deletar (Del)",
                Message::DeleteSelected
            ),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let transform_group = container(
        row![
            icon_btn(
                crate::icons::Icon::RotateCw,
                "Rotacionar (R)",
                Message::RotateClockwise
            ),
            icon_btn(
                crate::icons::Icon::RotateCcw,
                "Rotacionar como Grupo (Shift+R)",
                Message::RotateCounterClockwise
            ),
            icon_btn(
                crate::icons::Icon::MirrorH,
                "Espelhar Horizontalmente (M)",
                Message::MirrorHorizontal
            ),
            icon_btn(
                crate::icons::Icon::MirrorV,
                "Espelhar Verticalmente (F)",
                Message::MirrorVertical
            ),
            icon_btn(
                crate::icons::Icon::Group,
                "Agrupar (Ctrl+G)",
                Message::GroupSelected
            ),
            icon_btn(
                crate::icons::Icon::Ungroup,
                "Desagrupar (Ctrl+U)",
                Message::UngroupSelected
            ),
            icon_btn(
                crate::icons::Icon::Palette,
                "Colorir Elemento",
                Message::ColorElement
            ),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let tool_btn = |tool: EditorTool, icon: crate::icons::Icon, tip: &'static str| {
        let active = app.tool == tool;
        let s = if active {
            theme::tool_active
        } else {
            theme::tool
        };
        let col = if active { t.primary } else { t.text };
        app_tooltip(
            button(
                container(crate::icons::view_icon(icon, 16, col))
                    .width(20)
                    .height(20)
                    .center_x(20)
                    .center_y(20),
            )
            .style(s)
            .padding(Padding::from([3.0, 5.0]))
            .on_press(Message::SelectTool(tool)),
            tip,
            tooltip::Position::Bottom,
        )
    };

    let nav_group = container(
        row![
            tool_btn(
                EditorTool::Select,
                crate::icons::Icon::Select,
                "Selecionar (Ponteiro)"
            ),
            tool_btn(EditorTool::Pan, crate::icons::Icon::Pan, "Mover (Pan)"),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let draw_group = container(
        row![
            tool_btn(
                EditorTool::DrawWire(4000),
                crate::icons::Icon::WireL,
                "Cabo Fase L"
            ),
            tool_btn(
                EditorTool::DrawWire(4009),
                crate::icons::Icon::WireN,
                "Cabo Neutro N"
            ),
            tool_btn(
                EditorTool::DrawWire(4010),
                crate::icons::Icon::WirePe,
                "Cabo Terra PE"
            ),
            tool_btn(
                EditorTool::DrawWire(4003),
                crate::icons::Icon::WireDc,
                "Cabo Contínuo CC+"
            ),
            tool_btn(
                EditorTool::PlaceJunction,
                crate::icons::Icon::Junction,
                "Ponto de Junção"
            ),
            tool_btn(
                EditorTool::PlaceText,
                crate::icons::Icon::Text,
                "Texto em Linha (T)"
            ),
            tool_btn(
                EditorTool::PlaceTextField,
                crate::icons::Icon::TextBox,
                "Campo de Texto (Caixa)"
            ),
            tool_btn(
                EditorTool::DrawGraphicLine,
                crate::icons::Icon::DrawLine,
                "Linha Auxiliar (L)"
            ),
            tool_btn(
                EditorTool::DrawGraphicRect,
                crate::icons::Icon::DrawRect,
                "Retângulo / Moldura (B)"
            ),
            tool_btn(
                EditorTool::DrawGraphicCircle,
                crate::icons::Icon::DrawCircle,
                "Círculo / Elipse (O)"
            ),
            tool_btn(
                EditorTool::DrawGraphicArc,
                crate::icons::Icon::DrawArc,
                "Arco Vetorial"
            ),
            tool_btn(
                EditorTool::Delete,
                crate::icons::Icon::DeleteWire,
                "Apagar Conexão"
            ),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let view_group = container(
        row![
            icon_btn(
                crate::icons::Icon::ZoomIn,
                "Aproximar Zoom (Ctrl++)",
                Message::ZoomIn
            ),
            icon_btn(
                crate::icons::Icon::ZoomOut,
                "Afastar Zoom (Ctrl+-)",
                Message::ZoomOut
            ),
            icon_btn(
                crate::icons::Icon::ZoomReset,
                "Repor Zoom (Ctrl+0)",
                Message::ZoomReset
            ),
            icon_btn(
                crate::icons::Icon::FitToPage,
                "Ajustar à Vista (Ctrl+9)",
                Message::FitToPage
            ),
            icon_toggle(
                crate::icons::Icon::Grid,
                "Mostrar a Grelha",
                app.show_grid,
                Message::ToggleGrid(!app.show_grid)
            ),
            icon_toggle(
                crate::icons::Icon::Magnet,
                "Snap à Grelha",
                app.snap_to_grid,
                Message::ToggleSnap
            ),
            pick_list(
                &crate::model::GridDensity::ALL[..],
                Some(app.grid_density),
                Message::SetGridDensity,
            )
            .text_size(11)
            .padding(Padding::from([3.0, 6.0])),
            pick_list(
                &crate::app::TextVisibility::ALL[..],
                Some(if app.show_texts {
                    crate::app::TextVisibility::On
                } else {
                    crate::app::TextVisibility::Off
                }),
                Message::SetTextVisibility,
            )
            .text_size(11)
            .padding(Padding::from([3.0, 6.0])),
        ]
        .spacing(3)
        .align_y(Alignment::Center),
    )
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let theme_group = container(app_tooltip(
        button(
            container(crate::icons::view_icon(
                crate::icons::Icon::Theme,
                16,
                t.text,
            ))
            .width(20)
            .height(20)
            .center_x(20)
            .center_y(20),
        )
        .style(theme::tool)
        .padding(Padding::from([3.0, 5.0]))
        .on_press(Message::CycleTheme),
        "Alternar Tema (Sistema / Claro / Escuro)",
        tooltip::Position::Bottom,
    ))
    .style(theme::tool_group)
    .padding(Padding::from([2.0, 4.0]));

    let transport_group = container(
        row![
            app_tooltip(
                button(
                    container(crate::icons::view_icon(
                        crate::icons::Icon::Play,
                        14,
                        t.on_primary
                    ))
                    .width(20)
                    .height(20)
                    .center_x(20)
                    .center_y(20),
                )
                .style(theme::pill_play(app.sim_running))
                .padding(Padding::from([3.0, 5.0]))
                .on_press(Message::ToggleSimulation),
                if app.sim_running {
                    "Pausar/Parar Simulação"
                } else {
                    "Iniciar Simulação Elétrica"
                },
                tooltip::Position::Bottom,
            ),
            app_tooltip(
                button(
                    container(crate::icons::view_icon(
                        crate::icons::Icon::Stop,
                        13,
                        t.on_danger
                    ))
                    .width(20)
                    .height(20)
                    .center_x(20)
                    .center_y(20),
                )
                .style(theme::pill_stop(app.sim_running))
                .padding(Padding::from([3.0, 5.0]))
                .on_press(Message::StopSimulation),
                "Parar Simulação e Resetar",
                tooltip::Position::Bottom,
            ),
            app_tooltip(
                button(
                    container(crate::icons::view_icon(
                        crate::icons::Icon::Step,
                        15,
                        t.text
                    ))
                    .width(20)
                    .height(20)
                    .center_x(20)
                    .center_y(20),
                )
                .style(theme::pill_ghost)
                .padding(Padding::from([3.0, 5.0]))
                .on_press(Message::SimTick),
                "Avançar Passo de Simulação",
                tooltip::Position::Bottom,
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .style(theme::transport)
    .padding(Padding::from([2.0, 6.0]));

    let left_tools = scrollable(
        row![
            file_group,
            edit_group,
            transform_group,
            nav_group,
            draw_group,
            view_group,
            theme_group,
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::new()
            .width(3.0)
            .scroller_width(3.0)
            .margin(0.0),
    ))
    .width(Length::Fill);

    container(
        row![left_tools, transport_group,]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .style(theme::bar)
    .height(TOOL_H)
    .padding(Padding::from([0.0, 10.0]))
    .width(Length::Fill)
    .into()
}
