use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, text, Space,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, Message};
use crate::canvas_view::EditorTool;
use crate::theme::{self, tokens, MENU_H, MONO, UI};

pub use crate::app::ActiveMenu;

pub fn view_menubar<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let brand = row![
        crate::icons::view_icon(crate::icons::Icon::Bolt, 16, t.primary),
        text("RustSimu").size(13).font(Font {
            weight: iced::font::Weight::Bold,
            ..UI
        }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let menu_btn = |menu: ActiveMenu, label: &'static str| {
        let is_open = app.open_menu == Some(menu);
        let s = if is_open {
            theme::tool_active
        } else {
            theme::menu_item
        };
        button(text(label).size(12).font(UI))
            .style(s)
            .padding(Padding::from([4.0, 8.0]))
            .on_press(Message::ToggleMenu(Some(menu)))
    };

    let menus = row![
        menu_btn(ActiveMenu::Arquivo, "Arquivo"),
        menu_btn(ActiveMenu::Editar, "Editar"),
        menu_btn(ActiveMenu::Projeto, "Projeto"),
        menu_btn(ActiveMenu::Exibicao, "Exibição"),
        menu_btn(ActiveMenu::Configuracoes, "Configurações"),
        menu_btn(ActiveMenu::Ajuda, "Ajuda"),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    let file_name = app
        .current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Novo Projeto");

    let file_badge = text(file_name).size(11).font(MONO).color(t.text2);

    let mode_dot = container(Space::new(6, 6)).style(theme::status_dot(if app.sim_running {
        t.amber
    } else {
        t.primary
    }));
    let mode_label = text(if app.sim_running {
        "Simulação"
    } else {
        "Edição"
    })
    .size(10.5)
    .font(MONO);

    let mode_pill = container(
        row![mode_dot, mode_label]
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .style(theme::mode_badge(app.sim_running))
    .padding(Padding::from([2.0, 6.0]));

    let menubar_row = row![brand, menus, horizontal_space(), file_badge, mode_pill,]
        .spacing(10)
        .align_y(Alignment::Center);

    let top_bar = container(menubar_row)
        .style(theme::bar)
        .height(MENU_H)
        .padding(Padding::from([0.0, 12.0]))
        .width(Length::Fill);

    top_bar.into()
}

pub fn view_dropdown_menu<'a>(app: &'a CadeSimuApp, menu: ActiveMenu) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let menu_item_btn = |icon_opt: Option<crate::icons::Icon>,
                         label: &'static str,
                         shortcut: &'static str,
                         msg: Message| {
        let left_part = row![
            if let Some(ic) = icon_opt {
                container(crate::icons::view_icon(ic, 14, t.text2))
                    .width(16)
                    .height(16)
                    .center_x(16)
                    .center_y(16)
            } else {
                container(Space::with_width(16).height(16))
            },
            text(label).size(12).font(UI),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        button(
            row![
                left_part,
                horizontal_space(),
                text(shortcut).size(11).font(MONO).color(t.text2),
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center),
        )
        .style(theme::menu_item)
        .padding(Padding::from([5.0, 12.0]))
        .width(Length::Fill)
        .on_press(msg)
    };

    let items: Element<'_, Message> = match menu {
        ActiveMenu::Arquivo => column![
            menu_item_btn(
                Some(crate::icons::Icon::New),
                "Novo Esquema",
                "Ctrl+N",
                Message::NewDocument
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Open),
                "Abrir Esquema...",
                "Ctrl+O",
                Message::OpenFileDialog
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Save),
                "Salvar",
                "Ctrl+S",
                Message::SaveDocument
            ),
            menu_item_btn(
                Some(crate::icons::Icon::SaveAs),
                "Salvar Como...",
                "Ctrl+Shift+S",
                Message::SaveFileDialog
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Duplicate),
                "Salvar Cópia...",
                "Ctrl+Alt+S",
                Message::SaveAll
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Close),
                "Fechar Esquema",
                "Ctrl+W",
                Message::CloseDocument
            ),
            menu_item_btn(
                Some(crate::icons::Icon::ExportPdf),
                "Exportar (PDF, SVG, CAD)...",
                "Ctrl+Shift+E",
                Message::OpenExportModal
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Print),
                "Imprimir...",
                "Ctrl+P",
                Message::PrintDocument
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Close),
                "Fechar Menu",
                "ESC",
                Message::CloseMenu
            ),
        ]
        .spacing(2)
        .width(280)
        .into(),
        ActiveMenu::Editar => column![
            menu_item_btn(
                Some(crate::icons::Icon::Undo),
                "Desfazer",
                "Ctrl+Z",
                Message::Undo
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Redo),
                "Refazer",
                "Ctrl+Shift+Z",
                Message::Redo
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Cut),
                "Cortar",
                "Ctrl+X",
                Message::Cut
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Copy),
                "Copiar",
                "Ctrl+C",
                Message::Copy
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Paste),
                "Colar",
                "Ctrl+V",
                Message::Paste
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Paste),
                "Colar na Posição Original",
                "Ctrl+Shift+V",
                Message::PasteAtOriginalPos
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Duplicate),
                "Duplicar",
                "Ctrl+D",
                Message::Duplicate
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Bolt),
                "Inserir o Último Elemento",
                "A",
                Message::InsertLastElement
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Palette),
                "Colorir Elemento...",
                "",
                Message::ColorElement
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Select),
                "Selecionar Tudo",
                "Ctrl+A",
                Message::SelectAll
            ),
            menu_item_btn(
                None,
                "Anular a Seleção",
                "Ctrl+Shift+A",
                Message::DeselectAll
            ),
            menu_item_btn(
                Some(crate::icons::Icon::MirrorH),
                "Inverter Seleção",
                "Ctrl+I",
                Message::InvertSelection
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Group),
                "Agrupar",
                "Ctrl+G",
                Message::GroupSelected
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Ungroup),
                "Desagrupar",
                "Ctrl+U",
                Message::UngroupSelected
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Copy),
                "Copiar Info do Elemento",
                "Ctrl+Alt+C",
                Message::CopyElementInfo
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Paste),
                "Colar Info do Elemento",
                "Ctrl+Alt+V",
                Message::PasteElementInfo
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Delete),
                "Deletar",
                "Del",
                Message::DeleteSelected
            ),
            menu_item_btn(
                Some(crate::icons::Icon::RotateCw),
                "Rotacionar 90°",
                "R",
                Message::RotateClockwise
            ),
            menu_item_btn(
                Some(crate::icons::Icon::RotateCcw),
                "Rotacionar como Grupo",
                "Shift+R",
                Message::RotateCounterClockwise
            ),
            menu_item_btn(
                Some(crate::icons::Icon::MirrorH),
                "Espelhar Horizontal",
                "M",
                Message::MirrorHorizontal
            ),
            menu_item_btn(
                Some(crate::icons::Icon::MirrorV),
                "Espelhar Vertical",
                "F",
                Message::MirrorVertical
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::New),
                "Propriedades da Página",
                "Ctrl+Shift+Enter",
                Message::SelectPropTab(0)
            ),
        ]
        .spacing(2)
        .width(290)
        .into(),
        ActiveMenu::Projeto => {
            let autoconn_str = if app.auto_connect {
                "[X] Criação Automática de Condutores"
            } else {
                "[  ] Criação Automática de Condutores"
            };
            let autodisc_str = if app.auto_disconnect {
                "[X] Desconexão Automática de Condutores"
            } else {
                "[  ] Desconexão Automática de Condutores"
            };
            column![
                menu_item_btn(
                    Some(crate::icons::Icon::Copy),
                    "Propriedades do Projeto",
                    "",
                    Message::SelectPropTab(0)
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::WireL),
                    autoconn_str,
                    "",
                    Message::ToggleAutoConnect
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::DeleteWire),
                    autodisc_str,
                    "",
                    Message::ToggleAutoDisconnect
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::AddAfter),
                    "Adicionar Página",
                    "Ctrl+T",
                    Message::AddPageAfter(app.active_page_index)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Delete),
                    "Deletar esta Página",
                    "Ctrl+Shift+Bksp",
                    Message::DeletePage(app.active_page_index)
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::ChevronRight),
                    "Próxima Folha",
                    "Ctrl+PgDn",
                    Message::SelectPage(app.active_page_index + 1)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::ChevronLeft),
                    "Folha Anterior",
                    "Ctrl+PgUp",
                    Message::SelectPage(app.active_page_index.saturating_sub(1))
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Pencil),
                    "Renomear Folha Atual",
                    "",
                    Message::StartEditingPageName(app.active_page_index)
                ),
            ]
            .spacing(2)
            .width(310)
            .into()
        }
        ActiveMenu::Exibicao => {
            let select_str = if app.tool == EditorTool::Select {
                "[X] Selecionar (Ponteiro)"
            } else {
                "[  ] Selecionar (Ponteiro)"
            };
            let pan_str = if app.tool == EditorTool::Pan {
                "[X] Mover (Pan)"
            } else {
                "[  ] Mover (Pan)"
            };
            let snap_str = if app.snap_to_grid {
                "[X] Ajustar à Grelha (Snap)"
            } else {
                "[  ] Ajustar à Grelha (Snap)"
            };
            let text_str = if app.show_texts {
                "[X] Textos: Ligado"
            } else {
                "[  ] Textos: Desligado"
            };
            column![
                menu_item_btn(
                    Some(crate::icons::Icon::Folder),
                    "Mostrar Projetos",
                    "",
                    Message::SelectLeftDockTab(1)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Select),
                    select_str,
                    "",
                    Message::SelectTool(EditorTool::Select)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Pan),
                    pan_str,
                    "",
                    Message::SelectTool(EditorTool::Pan)
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Grid),
                    "Mostrar a Grelha",
                    "",
                    Message::ToggleGrid(!app.show_grid)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Magnet),
                    snap_str,
                    "Ctrl+Shift+G",
                    Message::ToggleSnap
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Text),
                    text_str,
                    "",
                    Message::ToggleShowTexts
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Theme),
                    "Alternar Tema",
                    "Ctrl+Shift+T",
                    Message::CycleTheme
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::ZoomIn),
                    "Ampliar (Zoom In)",
                    "Ctrl++",
                    Message::ZoomIn
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::ZoomOut),
                    "Reduzir (Zoom Out)",
                    "Ctrl+-",
                    Message::ZoomOut
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::ZoomReset),
                    "Zoom do Conteúdo",
                    "Ctrl+8",
                    Message::ZoomContent
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::FitToPage),
                    "Ajustar à Vista",
                    "Ctrl+9",
                    Message::FitToPage
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::ZoomReset),
                    "Repor Zoom (100%)",
                    "Ctrl+0",
                    Message::ZoomReset
                ),
            ]
            .spacing(2)
            .width(290)
            .into()
        }
        ActiveMenu::Configuracoes => column![
            menu_item_btn(
                Some(crate::icons::Icon::Grid),
                "Grade Fina (10 pts/cm)",
                "",
                Message::SetGridDensity(crate::model::GridDensity::Fine)
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Grid),
                "Grade Média (5 pts/cm)",
                "",
                Message::SetGridDensity(crate::model::GridDensity::Medium)
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Grid),
                "Grade Larga (2.5 pts/cm)",
                "",
                Message::SetGridDensity(crate::model::GridDensity::Coarse)
            ),
        ]
        .spacing(2)
        .width(270)
        .into(),
        ActiveMenu::Ajuda => column![
            menu_item_btn(
                Some(crate::icons::Icon::Bolt),
                "Sobre o RustSimu",
                "",
                Message::ShowAbout
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Open),
                "Carregar Exemplo: Partida Direta",
                "",
                Message::LoadPresetDol
            ),
            horizontal_rule(1),
            menu_item_btn(
                Some(crate::icons::Icon::Close),
                "Fechar Menu",
                "ESC",
                Message::CloseMenu
            ),
        ]
        .spacing(2)
        .width(270)
        .into(),
    };

    let left_padding = match menu {
        ActiveMenu::Arquivo => 85.0,
        ActiveMenu::Editar => 145.0,
        ActiveMenu::Projeto => 195.0,
        ActiveMenu::Exibicao => 250.0,
        ActiveMenu::Configuracoes => 315.0,
        ActiveMenu::Ajuda => 415.0,
    };

    row![
        Space::with_width(left_padding),
        container(items)
            .style(theme::dropdown_menu)
            .padding(Padding::from([6.0, 8.0])),
        horizontal_space(),
    ]
    .width(Length::Fill)
    .into()
}
