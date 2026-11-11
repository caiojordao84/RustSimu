use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, text, Space,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, Message};
use crate::canvas_view::EditorTool;
use crate::model::GridDensity;
use crate::theme::{self, tokens, ThemeMode, MENU_H, MONO, UI};

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
        menu_btn(ActiveMenu::Janela, "Janela"),
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

    let toggle_item_btn = |checked: bool,
                           icon_opt: Option<crate::icons::Icon>,
                           label: &'static str,
                           shortcut: &'static str,
                           msg: Message| {
        let mark_el: Element<'_, Message> = if checked {
            container(crate::icons::view_icon(
                crate::icons::Icon::Check,
                13,
                t.primary,
            ))
            .width(16)
            .height(16)
            .center_x(16)
            .center_y(16)
            .into()
        } else if let Some(ic) = icon_opt {
            container(crate::icons::view_icon(ic, 13, t.text2))
                .width(16)
                .height(16)
                .center_x(16)
                .center_y(16)
                .into()
        } else {
            container(Space::with_width(16).height(16)).into()
        };

        button(
            row![
                mark_el,
                text(label).size(12).font(UI),
                horizontal_space(),
                text(shortcut).size(11).font(MONO).color(t.text2),
            ]
            .spacing(8)
            .width(Length::Fill)
            .align_y(Alignment::Center),
        )
        .style(theme::menu_item)
        .padding(Padding::from([5.0, 12.0]))
        .width(Length::Fill)
        .on_press(msg)
    };

    let submenu_trigger_btn =
        |icon_opt: Option<crate::icons::Icon>, label: &'static str, key: &'static str| {
            let is_open = app.open_submenu == Some(key);
            let arrow = if is_open { "v" } else { ">" };
            menu_item_btn(icon_opt, label, arrow, Message::ToggleSubmenu(key))
        };

    let items: Element<'_, Message> = match menu {
        ActiveMenu::Arquivo => {
            let mut col = column![
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
                submenu_trigger_btn(
                    Some(crate::icons::Icon::Folder),
                    "Últimos Arquivos",
                    "arquivo_recentes"
                ),
            ];

            if app.open_submenu == Some("arquivo_recentes") {
                let mut sub = column![].spacing(2);
                if app.recent_files.is_empty() {
                    sub = sub.push(
                        container(text("(Nenhum arquivo recente)").size(11).color(t.text2))
                            .padding(Padding::from([4.0, 10.0])),
                    );
                } else {
                    for path_str in app.recent_files.iter().take(8) {
                        let file_name = std::path::Path::new(path_str)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path_str.clone());
                        sub = sub.push(
                            button(
                                row![
                                    crate::icons::view_icon(crate::icons::Icon::Open, 12, t.text2),
                                    text(file_name).size(11.5).font(UI).width(Length::Fill),
                                ]
                                .spacing(6)
                                .align_y(Alignment::Center),
                            )
                            .style(theme::menu_item)
                            .padding(Padding::from([4.0, 10.0]))
                            .width(Length::Fill)
                            .on_press(Message::RecentProjectSelected(path_str.clone())),
                        );
                    }
                    sub = sub.push(horizontal_rule(1));
                    sub = sub.push(menu_item_btn(
                        Some(crate::icons::Icon::Delete),
                        "Limpar Lista de Recentes",
                        "",
                        Message::ClearRecentFiles,
                    ));
                }
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col
                .push(horizontal_rule(1))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::Save),
                    "Salvar",
                    "Ctrl+S",
                    Message::SaveDocument,
                ))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::SaveAs),
                    "Salvar Como...",
                    "Ctrl+Shift+S",
                    Message::SaveFileDialog,
                ))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::Duplicate),
                    "Salvar Cópia...",
                    "Ctrl+Alt+S",
                    Message::SaveCopyDialog,
                ))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::Save),
                    "Salvar Tudo",
                    "",
                    Message::SaveAll,
                ))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::Close),
                    "Fechar Projeto",
                    "Ctrl+W",
                    Message::CloseDocument,
                ))
                .push(horizontal_rule(1))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::ExportPdf),
                    "Exportar (PNG, JPG, BMP, SVG, DXF, CAD, PDF)...",
                    "Ctrl+E",
                    Message::OpenExportModal,
                ))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::Print),
                    "Imprimir...",
                    "Ctrl+P",
                    Message::OpenPrintModal,
                ))
                .push(horizontal_rule(1))
                .push(menu_item_btn(
                    Some(crate::icons::Icon::Close),
                    "Sair do RustSimu",
                    "Alt+F4",
                    Message::ExitApplication,
                ));

            col.spacing(2).width(340).into()
        }
        ActiveMenu::Editar => {
            let mut col = column![
                menu_item_btn(
                    Some(crate::icons::Icon::Undo),
                    "Desfazer",
                    "Ctrl+Z",
                    Message::Undo
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Redo),
                    "Refazer",
                    "Ctrl+Y",
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
                    Some(crate::icons::Icon::Duplicate),
                    "Duplicar",
                    "Ctrl+D",
                    Message::Duplicate
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Delete),
                    "Excluir",
                    "Del",
                    Message::DeleteSelected
                ),
                horizontal_rule(1),
                submenu_trigger_btn(
                    Some(crate::icons::Icon::Select),
                    "Seleção, Grupos e Clipe",
                    "editar_selecao"
                ),
            ];

            if app.open_submenu == Some("editar_selecao") {
                let sub = column![
                    menu_item_btn(
                        Some(crate::icons::Icon::Paste),
                        "Colar na Posição Original",
                        "Ctrl+Shift+V",
                        Message::PasteAtOriginalPos
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Bolt),
                        "Inserir Último Elemento",
                        "A",
                        Message::InsertLastElement
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Copy),
                        "Copiar Info Elétrica",
                        "Ctrl+Alt+C",
                        Message::CopyElementInfo
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Paste),
                        "Colar Info Elétrica",
                        "Ctrl+Alt+V",
                        Message::PasteElementInfo
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::Select),
                        "Selecionar Tudo",
                        "Ctrl+A",
                        Message::SelectAll
                    ),
                    menu_item_btn(None, "Anular Seleção", "Ctrl+Shift+A", Message::DeselectAll),
                    menu_item_btn(
                        Some(crate::icons::Icon::MirrorH),
                        "Inverter Seleção",
                        "Ctrl+I",
                        Message::InvertSelection
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::Group),
                        "Agrupar Selecionados",
                        "Ctrl+G",
                        Message::GroupSelected
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Ungroup),
                        "Desagrupar",
                        "Ctrl+U",
                        Message::UngroupSelected
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col.push(submenu_trigger_btn(
                Some(crate::icons::Icon::Text),
                "Adicionar (Texto, Imagem, Formas)",
                "editar_adicionar",
            ));
            if app.open_submenu == Some("editar_adicionar") {
                let sub = column![
                    menu_item_btn(
                        Some(crate::icons::Icon::Text),
                        "Texto em Linha",
                        "",
                        Message::SelectTool(EditorTool::PlaceText)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Text),
                        "Campo de Texto",
                        "",
                        Message::SelectTool(EditorTool::PlaceTextField)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Image),
                        "Imagem...",
                        "",
                        Message::InsertImageDialog
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::DrawLine),
                        "Linha",
                        "",
                        Message::SelectTool(EditorTool::DrawGraphicLine)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::DrawRect),
                        "Retângulo",
                        "",
                        Message::SelectTool(EditorTool::DrawGraphicRect)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::DrawCircle),
                        "Círculo / Elipse",
                        "",
                        Message::SelectTool(EditorTool::DrawGraphicCircle)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::DrawArc),
                        "Arco",
                        "",
                        Message::SelectTool(EditorTool::DrawGraphicArc)
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col.push(submenu_trigger_btn(
                Some(crate::icons::Icon::Grid),
                "Linhas e Colunas",
                "editar_grade",
            ));
            if app.open_submenu == Some("editar_grade") {
                let sub = column![
                    menu_item_btn(
                        Some(crate::icons::Icon::AddAfter),
                        "Adicionar Coluna",
                        "",
                        Message::AddGridCol
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Delete),
                        "Remover Coluna",
                        "",
                        Message::RemoveGridCol
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::AddAfter),
                        "Adicionar Linha",
                        "",
                        Message::AddGridRow
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Delete),
                        "Remover Linha",
                        "",
                        Message::RemoveGridRow
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col.push(submenu_trigger_btn(
                Some(crate::icons::Icon::RotateCw),
                "Transformar, Alinhar e Cor",
                "editar_transform",
            ));
            if app.open_submenu == Some("editar_transform") {
                let sub = column![
                    menu_item_btn(
                        Some(crate::icons::Icon::RotateCw),
                        "Rotacionar 90° Horário",
                        "R",
                        Message::RotateClockwise
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::RotateCcw),
                        "Rotacionar Anti-horário",
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
                    menu_item_btn(
                        Some(crate::icons::Icon::Magnet),
                        "Alinhar à Grelha",
                        "",
                        Message::AlignToGrid
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Palette),
                        "Colorir Elemento...",
                        "",
                        Message::ColorElement
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col.push(horizontal_rule(1)).push(menu_item_btn(
                Some(crate::icons::Icon::Settings),
                "Propriedades da Página...",
                "Ctrl+Shift+Enter",
                Message::OpenPagePropertiesModal,
            ));

            col.spacing(2).width(325).into()
        }
        ActiveMenu::Projeto => column![
            menu_item_btn(
                Some(crate::icons::Icon::Folder),
                "Propriedades do Projeto...",
                "",
                Message::OpenProjectPropertiesModal
            ),
            horizontal_rule(1),
            toggle_item_btn(
                app.auto_connect,
                Some(crate::icons::Icon::WireL),
                "Autoconectar Condutores",
                "",
                Message::ToggleAutoConnect
            ),
            toggle_item_btn(
                app.auto_disconnect,
                Some(crate::icons::Icon::DeleteWire),
                "Desconectar Automaticamente ao Mover",
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
                Some(crate::icons::Icon::ChevronLeft),
                "Folha Anterior",
                "Ctrl+PgUp",
                Message::PrevFolio
            ),
            menu_item_btn(
                Some(crate::icons::Icon::ChevronRight),
                "Próxima Folha",
                "Ctrl+PgDn",
                Message::NextFolio
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Pencil),
                "Renomear Folha Atual",
                "",
                Message::StartEditingPageName(app.active_page_index)
            ),
        ]
        .spacing(2)
        .width(330)
        .into(),
        ActiveMenu::Exibicao => {
            let mut col = column![
                menu_item_btn(
                    Some(crate::icons::Icon::Folder),
                    "Mostrar Projetos",
                    "",
                    Message::ShowProjectsDock
                ),
                toggle_item_btn(
                    app.tool == EditorTool::Select,
                    Some(crate::icons::Icon::Select),
                    "Selecionar (Ponteiro)",
                    "",
                    Message::SelectTool(EditorTool::Select)
                ),
                toggle_item_btn(
                    app.tool == EditorTool::Pan,
                    Some(crate::icons::Icon::Pan),
                    "Mover (Pan)",
                    "",
                    Message::SelectTool(EditorTool::Pan)
                ),
                horizontal_rule(1),
                toggle_item_btn(
                    app.show_grid,
                    Some(crate::icons::Icon::Grid),
                    "Mostrar a Grelha",
                    "",
                    Message::ToggleGrid(!app.show_grid)
                ),
                toggle_item_btn(
                    app.snap_to_grid,
                    Some(crate::icons::Icon::Magnet),
                    "Ajustar à Grelha (Snap)",
                    "Ctrl+Shift+G",
                    Message::ToggleSnap
                ),
                submenu_trigger_btn(
                    Some(crate::icons::Icon::Grid),
                    "Passo da Grelha",
                    "exibicao_passo"
                ),
            ];

            if app.open_submenu == Some("exibicao_passo") {
                let sub = column![
                    toggle_item_btn(
                        app.grid_density == GridDensity::Fine,
                        None,
                        "Fina (10 pts/cm - 2 mm)",
                        "",
                        Message::SetGridDensity(GridDensity::Fine)
                    ),
                    toggle_item_btn(
                        app.grid_density == GridDensity::Medium,
                        None,
                        "Média (5 pts/cm - 4 mm)",
                        "",
                        Message::SetGridDensity(GridDensity::Medium)
                    ),
                    toggle_item_btn(
                        app.grid_density == GridDensity::Coarse,
                        None,
                        "Larga (2.5 pts/cm - 8 mm)",
                        "",
                        Message::SetGridDensity(GridDensity::Coarse)
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col
                .push(toggle_item_btn(
                    app.show_full_crosshair,
                    None,
                    "Mira em Todo o Fólio",
                    "",
                    Message::ToggleFullCrosshair,
                ))
                .push(toggle_item_btn(
                    app.show_text_grid,
                    None,
                    "Grelha de Texto",
                    "",
                    Message::ToggleTextGrid,
                ))
                .push(toggle_item_btn(
                    app.show_alignment_guides,
                    None,
                    "Ver os Guias de Alinhamento",
                    "",
                    Message::ToggleAlignmentGuides,
                ))
                .push(toggle_item_btn(
                    app.keep_headers_visible,
                    None,
                    "Manter Cabeçalhos Visíveis",
                    "",
                    Message::ToggleKeepHeadersVisible,
                ))
                .push(toggle_item_btn(
                    app.show_cell_bounds,
                    None,
                    "Mostrar Limites das Células",
                    "",
                    Message::ToggleCellBounds,
                ))
                .push(submenu_trigger_btn(
                    Some(crate::icons::Icon::Text),
                    "Mostrar (Textos, Terminais, Cajetín)",
                    "exibicao_mostrar",
                ));

            if app.open_submenu == Some("exibicao_mostrar") {
                let sub = column![
                    toggle_item_btn(
                        app.show_texts,
                        None,
                        "Textos e Identificações",
                        "",
                        Message::ToggleShowTexts
                    ),
                    toggle_item_btn(
                        app.show_terminals,
                        None,
                        "Terminais (Bornes)",
                        "",
                        Message::ToggleShowTerminals
                    ),
                    toggle_item_btn(
                        app.show_cajetin,
                        None,
                        "Cajetín (Bloco de Legenda)",
                        "",
                        Message::ToggleShowCajetin
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col.push(submenu_trigger_btn(
                Some(crate::icons::Icon::Palette),
                "Cor de Fundo da Folha",
                "exibicao_bg",
            ));
            if app.open_submenu == Some("exibicao_bg") {
                let sub = column![
                    toggle_item_btn(
                        app.doc.settings.sheet_bg_color.is_none(),
                        None,
                        "Padrão do Tema",
                        "",
                        Message::SetSheetBgPreset(None)
                    ),
                    toggle_item_btn(
                        app.doc.settings.sheet_bg_color == Some(0xFFFFFF),
                        None,
                        "Branco Puro",
                        "",
                        Message::SetSheetBgPreset(Some(0xFFFFFF))
                    ),
                    toggle_item_btn(
                        app.doc.settings.sheet_bg_color == Some(0xFAF5E4),
                        None,
                        "Papel Creme",
                        "",
                        Message::SetSheetBgPreset(Some(0xFAF5E4))
                    ),
                    toggle_item_btn(
                        app.doc.settings.sheet_bg_color == Some(0xEAF2FA),
                        None,
                        "Azul Técnico",
                        "",
                        Message::SetSheetBgPreset(Some(0xEAF2FA))
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col = col.push(submenu_trigger_btn(
                Some(crate::icons::Icon::ZoomIn),
                "Zoom e Enquadramento",
                "exibicao_zoom",
            ));
            if app.open_submenu == Some("exibicao_zoom") {
                let sub = column![
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
                        Some(crate::icons::Icon::FitToPage),
                        "Ajustar à Folha",
                        "Ctrl+9",
                        Message::ZoomFit
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::ZoomReset),
                        "Zoom do Conteúdo",
                        "Ctrl+8",
                        Message::ZoomContent
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::ZoomReset),
                        "Repor Zoom (100%)",
                        "Ctrl+0",
                        Message::ZoomReset
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Select),
                        "Centrar no Cursor",
                        "",
                        Message::CenterOnCursor
                    ),
                ]
                .spacing(2);
                col = col.push(
                    container(sub)
                        .style(theme::card)
                        .padding(Padding::from([4.0, 4.0]))
                        .width(Length::Fill),
                );
            }

            col.spacing(2).width(325).into()
        }
        ActiveMenu::Configuracoes => column![
            menu_item_btn(
                Some(crate::icons::Icon::Settings),
                "Configurações Gerais...",
                "",
                Message::OpenSettingsModal
            ),
            horizontal_rule(1),
            toggle_item_btn(
                app.theme_mode == ThemeMode::Dark,
                Some(crate::icons::Icon::Theme),
                "Tema Escuro",
                "",
                Message::SetThemeMode(ThemeMode::Dark)
            ),
            toggle_item_btn(
                app.theme_mode == ThemeMode::Light,
                Some(crate::icons::Icon::Theme),
                "Tema Claro",
                "",
                Message::SetThemeMode(ThemeMode::Light)
            ),
            toggle_item_btn(
                app.theme_mode == ThemeMode::System,
                Some(crate::icons::Icon::Theme),
                "Tema do Sistema",
                "",
                Message::SetThemeMode(ThemeMode::System)
            ),
            horizontal_rule(1),
            toggle_item_btn(
                app.show_grid,
                Some(crate::icons::Icon::Grid),
                "Mostrar a Grelha",
                "",
                Message::ToggleGrid(!app.show_grid)
            ),
            toggle_item_btn(
                app.snap_to_grid,
                Some(crate::icons::Icon::Magnet),
                "Snap (Imã à Grelha)",
                "Ctrl+Shift+G",
                Message::ToggleSnap
            ),
        ]
        .spacing(2)
        .width(285)
        .into(),
        ActiveMenu::Janela => {
            let mut col = column![
                toggle_item_btn(
                    !app.left_dock_collapsed,
                    Some(crate::icons::Icon::Window),
                    "Dock Esquerdo (Coleções / Projetos)",
                    "",
                    Message::ToggleLeftDockCollapse
                ),
                toggle_item_btn(
                    !app.right_dock_collapsed,
                    Some(crate::icons::Icon::Window),
                    "Dock Direito (Propriedades / Xref)",
                    "",
                    Message::ToggleRightDockCollapse
                ),
                toggle_item_btn(
                    app.focus_canvas,
                    Some(crate::icons::Icon::FitToPage),
                    "Modo Foco do Fólio (Tela Limpa)",
                    "Esc",
                    Message::ToggleFocusCanvas
                ),
                horizontal_rule(1),
                toggle_item_btn(
                    app.left_dock_tab == 0 && !app.left_dock_collapsed,
                    None,
                    "Aba Esquerda: Coleção de Símbolos",
                    "",
                    Message::SelectLeftDockTab(0)
                ),
                toggle_item_btn(
                    app.left_dock_tab == 1 && !app.left_dock_collapsed,
                    None,
                    "Aba Esquerda: Projetos",
                    "",
                    Message::ShowProjectsDock
                ),
                toggle_item_btn(
                    app.prop_tab == 0 && !app.right_dock_collapsed,
                    None,
                    "Aba Direita: Propriedades",
                    "",
                    Message::SelectPropTab(0)
                ),
                toggle_item_btn(
                    app.prop_tab == 1 && !app.right_dock_collapsed,
                    None,
                    "Aba Direita: Referência Cruzada",
                    "",
                    Message::SelectPropTab(1)
                ),
                horizontal_rule(1),
                container(
                    text("Folhas do Projeto:")
                        .size(10.5)
                        .font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        })
                        .color(t.text2),
                )
                .padding(Padding::from([3.0, 12.0])),
            ];

            for (idx, page) in app.pages.iter().enumerate().take(12) {
                let is_active = idx == app.active_page_index;
                let folio_label = format!("F{}: {}", idx + 1, page.name);
                let mark: Element<'_, Message> = if is_active {
                    container(crate::icons::view_icon(
                        crate::icons::Icon::Check,
                        13,
                        t.primary,
                    ))
                    .width(16)
                    .height(16)
                    .center_x(16)
                    .center_y(16)
                    .into()
                } else {
                    container(Space::with_width(16).height(16)).into()
                };
                col = col.push(
                    button(
                        row![
                            mark,
                            text(folio_label).size(11.5).font(UI).width(Length::Fill)
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .style(theme::menu_item)
                    .padding(Padding::from([4.0, 12.0]))
                    .width(Length::Fill)
                    .on_press(Message::SelectFolio(idx)),
                );
            }

            col.spacing(2).width(310).into()
        }
        ActiveMenu::Ajuda => column![
            menu_item_btn(
                Some(crate::icons::Icon::Keyboard),
                "Atalhos de Teclado...",
                "F1",
                Message::OpenShortcutsModal
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Open),
                "Carregar Exemplo: Partida Direta",
                "",
                Message::LoadPresetDol
            ),
            menu_item_btn(
                Some(crate::icons::Icon::Bolt),
                "Sobre o RustSimu...",
                "",
                Message::ShowAbout
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
        .width(290)
        .into(),
    };

    let left_padding = match menu {
        ActiveMenu::Arquivo => 85.0,
        ActiveMenu::Editar => 145.0,
        ActiveMenu::Projeto => 195.0,
        ActiveMenu::Exibicao => 250.0,
        ActiveMenu::Configuracoes => 315.0,
        ActiveMenu::Janela => 420.0,
        ActiveMenu::Ajuda => 480.0,
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
