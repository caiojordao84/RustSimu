use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, mouse_area, row, scrollable, svg,
    text, text_input,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, Message};
use crate::canvas_view::EditorTool;
use crate::theme::{self, tokens, MONO, UI};

pub fn get_subdivisions_for_family(
    family: Option<&str>,
) -> &'static [(&'static str, Option<&'static str>)] {
    match family {
        None => &[
            ("Todas", None),
            ("Motores", Some("motor")),
            ("Disjuntores", Some("circuit_breaker")),
            ("Fusíveis", Some("fuse")),
            ("Bobinas", Some("relay_coil")),
            ("Contatos", Some("contactor_contact")),
            ("Botoeiras", Some("momentary_button")),
            ("Chaves", Some("latching_switch")),
            ("Sinaleiros", Some("lamp")),
            ("Fontes", Some("source")),
            ("Bornes", Some("terminal")),
            ("Cargas", Some("load")),
            ("Blocos / Montagem", Some("block")),
            ("Válvulas", Some("pneu_valves")),
            ("Atuadores", Some("pneu_actuators")),
            ("Lógica", Some("logic_gates")),
        ],
        Some("electric") => &[
            ("Todas", None),
            ("Motores", Some("motor")),
            ("Disjuntores", Some("circuit_breaker")),
            ("Fusíveis", Some("fuse")),
            ("Bobinas", Some("relay_coil")),
            ("Contatos", Some("contactor_contact")),
            ("Botoeiras", Some("momentary_button")),
            ("Chaves", Some("latching_switch")),
            ("Sinaleiros", Some("lamp")),
            ("Fontes", Some("source")),
            ("Bornes", Some("terminal")),
            ("Cargas", Some("load")),
            ("Blocos / Montagem", Some("block")),
        ],
        Some("pneumatic") => &[
            ("Todas", None),
            ("Válvulas", Some("pneu_valves")),
            ("Ar Comprimido", Some("pneu_air")),
            ("Atuadores / Cilindros", Some("pneu_actuators")),
            ("Sensores", Some("pneu_sensors")),
        ],
        Some("hydraulic") => &[
            ("Todas", None),
            ("Válvulas de Controle", Some("hyd_valves")),
            ("Bombas", Some("hyd_pumps")),
            ("Cilindros", Some("hyd_cylinders")),
            ("Tanques", Some("hyd_tanks")),
            ("Filtros", Some("hyd_filters")),
        ],
        Some("logic") => &[
            ("Todas", None),
            ("Portas Lógicas", Some("logic_gates")),
            ("Ladder", Some("logic_ladder")),
            ("Fluxograma e Grafcet", Some("logic_flowchart")),
        ],
        Some("energy") => &[
            ("Todas", None),
            ("Água e Calefação", Some("energy_water")),
            ("Refrigeração", Some("energy_refrig")),
            ("Solar Térmica", Some("energy_solar")),
            ("Fabricantes", Some("energy_mfg")),
        ],
        Some("cade_native") => &[
            ("Todas", None),
            ("Alimentações", Some("cade_power")),
            ("Disjuntores / Fusíveis", Some("cade_protection")),
            ("Contatores / Relés", Some("cade_control")),
            ("Motores", Some("cade_motors")),
            ("Botoeiras / Chaves", Some("cade_switches")),
            ("Sinalização", Some("cade_signaling")),
        ],
        Some("import") => &[
            ("Todas", None),
            ("Automotivo / Veicular", Some("classic_car")),
            ("Elétrica Importada", Some("10_electric")),
            ("Outros", Some("import")),
        ],
        Some(_) => &[("Todas", None)],
    }
}

pub fn view_collections_panel<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    if app.left_dock_collapsed {
        return container(
            button(crate::icons::view_icon(
                crate::icons::Icon::ChevronRight,
                14,
                t.text,
            ))
            .style(theme::tool)
            .padding(Padding::from([8.0, 5.0]))
            .on_press(Message::ToggleLeftDockCollapse),
        )
        .align_x(iced::alignment::Horizontal::Center)
        .style(theme::bar)
        .width(Length::Fixed(28.0))
        .height(Length::Fill)
        .into();
    }

    let header = container(
        row![
            text("Elementos e Projetos").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::ChevronLeft,
                12,
                t.text
            ))
            .style(theme::tool)
            .padding(Padding::from([3.0, 6.0]))
            .on_press(Message::ToggleLeftDockCollapse),
        ]
        .align_y(Alignment::Center),
    )
    .style(theme::bar)
    .padding(Padding::from([5.0, 10.0]))
    .width(Length::Fill);

    let dock_tabs = container(
        row![
            button(
                row![
                    crate::icons::view_icon(
                        crate::icons::Icon::Bolt,
                        13,
                        if app.left_dock_tab == 0 {
                            t.primary
                        } else {
                            t.text
                        },
                    ),
                    text("Elementos").size(11).font(UI),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
            )
            .style(if app.left_dock_tab == 0 {
                theme::tool_active
            } else {
                theme::tool
            })
            .padding(Padding::from([4.0, 8.0]))
            .width(Length::Fill)
            .on_press(Message::SelectLeftDockTab(0)),
            button(
                row![
                    crate::icons::view_icon(
                        crate::icons::Icon::Folder,
                        13,
                        if app.left_dock_tab == 1 {
                            t.primary
                        } else {
                            t.text
                        },
                    ),
                    text("Projetos").size(11).font(UI),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
            )
            .style(if app.left_dock_tab == 1 {
                theme::tool_active
            } else {
                theme::tool
            })
            .padding(Padding::from([4.0, 8.0]))
            .width(Length::Fill)
            .on_press(Message::SelectLeftDockTab(1)),
        ]
        .spacing(4)
        .width(Length::Fill),
    )
    .style(theme::bar)
    .padding(Padding::from([4.0, 8.0]))
    .width(Length::Fill);

    let content: Element<'_, Message> = match app.left_dock_tab {
        0 => view_unified_library_panel(app),
        _ => {
            // Aba Projetos
            let mut proj_col = column![].spacing(6).padding(Padding::from([8.0, 10.0]));

            proj_col = proj_col.push(
                text("Folha Ativa:")
                    .size(11.5)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    })
                    .color(t.primary),
            );

            for (idx, p) in app.pages.iter().enumerate() {
                let is_cur = idx == app.active_page_index;
                let p_btn = button(
                    row![
                        text(format!("{}. {}", idx + 1, p.name)).size(11.5).font(UI),
                        horizontal_space(),
                        if is_cur {
                            row![
                                crate::icons::view_icon(crate::icons::Icon::Check, 11, t.primary),
                                text("Ativa").size(10.5).color(t.primary)
                            ]
                            .spacing(3)
                            .align_y(Alignment::Center)
                        } else {
                            row![text("").size(10.5)]
                        },
                    ]
                    .align_y(Alignment::Center),
                )
                .style(if is_cur {
                    theme::tool_active
                } else {
                    theme::tool
                })
                .padding(Padding::from([4.0, 8.0]))
                .width(Length::Fill)
                .on_press(Message::SelectPage(idx));
                proj_col = proj_col.push(p_btn);
            }

            proj_col = proj_col.push(horizontal_rule(1));

            proj_col = proj_col.push(
                row![
                    text("Projetos Recentes:")
                        .size(11.5)
                        .font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        })
                        .color(t.primary),
                    horizontal_space(),
                    text(format!("{} arquivos", app.recent_files.len()))
                        .size(10.5)
                        .color(t.text2),
                ]
                .align_y(Alignment::Center),
            );

            let mut files_col = column![].spacing(4);
            if app.recent_files.is_empty() {
                files_col = files_col.push(
                    text("Nenhum arquivo recente na pasta Projetos.")
                        .size(11)
                        .color(t.text2),
                );
            } else {
                for path_str in &app.recent_files {
                    let p = std::path::Path::new(path_str);
                    let file_name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path_str.clone());
                    let ext = p
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("cad")
                        .to_uppercase();

                    let open_btn = button(
                        row![
                            container(text(ext).size(9.5).font(MONO))
                                .padding(Padding::from([1.0, 4.0]))
                                .style(theme::card),
                            text(file_name).size(11).font(UI).width(Length::Fill),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .style(theme::tool)
                    .padding(Padding::from([3.0, 6.0]))
                    .width(Length::Fill)
                    .on_press(Message::RecentProjectSelected(path_str.clone()));

                    let del_btn = button(crate::icons::view_icon(
                        crate::icons::Icon::Close,
                        11,
                        t.text2,
                    ))
                    .style(theme::tool)
                    .padding(Padding::from([3.0, 5.0]))
                    .on_press(Message::DeleteRecentProject(path_str.clone()));

                    files_col = files_col.push(
                        row![open_btn, del_btn]
                            .spacing(2)
                            .align_y(Alignment::Center),
                    );
                }
            }

            let files_scroll = container(scrollable(files_col)).height(Length::Fill);
            column![proj_col, files_scroll].height(Length::Fill).into()
        }
    };

    column![header, content, dock_tabs]
        .height(Length::Fill)
        .into()
}

pub fn view_unified_library_panel<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());
    let is_dark = app.is_dark();

    // 1. Atalhos de cabeamento rápido sempre visíveis
    let wire_shortcuts = row![
        button(text("+ Fase L").size(10.5))
            .style(theme::tool)
            .padding(Padding::from([2.0, 6.0]))
            .on_press(Message::SelectTool(EditorTool::DrawWire(4000))),
        button(text("+ Neutro N").size(10.5))
            .style(theme::tool)
            .padding(Padding::from([2.0, 6.0]))
            .on_press(Message::SelectTool(EditorTool::DrawWire(4009))),
        button(text("+ Terra").size(10.5))
            .style(theme::tool)
            .padding(Padding::from([2.0, 6.0]))
            .on_press(Message::SelectTool(EditorTool::DrawWire(4010))),
        button(text("+ Nó").size(10.5))
            .style(theme::tool)
            .padding(Padding::from([2.0, 6.0]))
            .on_press(Message::SelectTool(EditorTool::PlaceJunction)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let shortcuts_box = container(wire_shortcuts)
        .padding(Padding::from([4.0, 10.0]))
        .width(Length::Fill);

    // 2. Barra de busca com botão de limpar
    let search_input = text_input(
        "Buscar na biblioteca unificada (9.3k)...",
        &app.canonical_search,
    )
    .on_input(Message::CanonicalSearchChanged)
    .padding(Padding::from([5.0, 8.0]))
    .size(11.5)
    .style(theme::field);

    let search_box = if app.canonical_search.is_empty() {
        container(search_input).padding(Padding::from([4.0, 10.0]))
    } else {
        container(
            row![
                search_input,
                button(text("x").size(10.0).font(UI))
                    .style(theme::tool)
                    .padding(Padding::from([4.0, 6.0]))
                    .on_press(Message::CanonicalSearchChanged(String::new())),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding(Padding::from([4.0, 10.0]))
    };

    // 3. Chips de Escopo: Todos / Esquema (Cabáveis) / Painel (Blocos)
    let mut scope_chips = row![].spacing(4).align_y(Alignment::Center);
    for (label, sc) in [
        ("Todos", crate::canonical::CanonicalScope::All),
        (
            "Esquema (Cabáveis)",
            crate::canonical::CanonicalScope::Wireable,
        ),
        ("Painel (Blocos)", crate::canonical::CanonicalScope::Block),
    ] {
        let is_active = app.canonical_scope == sc;
        scope_chips = scope_chips.push(
            button(text(label).size(10.0).font(UI))
                .style(if is_active {
                    theme::tool_active
                } else {
                    theme::tool
                })
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectCanonicalScope(sc)),
        );
    }
    let scope_scroll = container(
        scrollable(scope_chips)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(2.0)
                    .scroller_width(2.0)
                    .margin(0.0),
            ))
            .width(Length::Fill),
    )
    .padding(Padding::from([2.0, 10.0]));

    // 4. Chips de filtro por Família
    let mut fam_chips = row![].spacing(4).align_y(Alignment::Center);
    for (label, fam_opt) in [
        ("Todas", None),
        ("Elétrica", Some("electric")),
        ("Energia", Some("energy")),
        ("Pneumática", Some("pneumatic")),
        ("Hidráulica", Some("hydraulic")),
        ("Lógica", Some("logic")),
        ("Importados", Some("import")),
        ("CADe", Some("cade_native")),
    ] {
        let is_active = app.canonical_family_filter.as_deref() == fam_opt;
        let fam_val = fam_opt.map(|s| s.to_string());
        fam_chips = fam_chips.push(
            button(text(label).size(10.0).font(UI))
                .style(if is_active {
                    theme::tool_active
                } else {
                    theme::tool
                })
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectCanonicalFamily(fam_val)),
        );
    }
    let fam_scroll = container(
        scrollable(fam_chips)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(2.0)
                    .scroller_width(2.0)
                    .margin(0.0),
            ))
            .width(Length::Fill),
    )
    .padding(Padding::from([0.0, 10.0]));

    // 4. Chips de filtro por Subdivisão temática dinâmica
    let mut sub_chips = row![].spacing(4).align_y(Alignment::Center);
    let subdivisions = get_subdivisions_for_family(app.canonical_family_filter.as_deref());
    for &(label, sub_opt) in subdivisions {
        let is_active = app.canonical_kind_filter.as_deref() == sub_opt;
        let sub_val = sub_opt.map(|s| s.to_string());
        sub_chips = sub_chips.push(
            button(text(label).size(10.0).font(UI))
                .style(if is_active {
                    theme::tool_active
                } else {
                    theme::tool
                })
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectCanonicalKind(sub_val)),
        );
    }
    let sub_scroll = container(
        scrollable(sub_chips)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(2.0)
                    .scroller_width(2.0)
                    .margin(0.0),
            ))
            .width(Length::Fill),
    )
    .padding(Padding::from([2.0, 10.0]));

    // 5. Filtragem e Paginação dos Elementos
    let (page_elements, current_page, total_pages, total_count) =
        if let Some(dict) = crate::canonical::global_dictionary() {
            let filtered = dict.filter_elements(
                &app.canonical_search,
                app.canonical_family_filter.as_deref(),
                app.canonical_kind_filter.as_deref(),
                app.canonical_scope,
            );
            let total = filtered.len();
            let total_pages = if total == 0 { 1 } else { total.div_ceil(35) };
            let cur_page = app.canonical_page.min(total_pages.saturating_sub(1));
            let start = cur_page * 35;
            let end = total.min(start + 35);
            let page_slice: Vec<_> = filtered[start..end].to_vec();
            (page_slice, cur_page, total_pages, total)
        } else {
            (Vec::new(), 0, 1, 0)
        };

    // 6. Barra de controle de paginação
    let prev_btn = button(text("◀").size(10.0).font(UI))
        .style(theme::tool)
        .padding(Padding::from([2.0, 6.0]));
    let prev_btn = if current_page > 0 {
        prev_btn.on_press(Message::PrevCanonicalPage)
    } else {
        prev_btn
    };

    let next_btn = button(text("▶").size(10.0).font(UI))
        .style(theme::tool)
        .padding(Padding::from([2.0, 6.0]));
    let next_btn = if current_page + 1 < total_pages {
        next_btn.on_press(Message::NextCanonicalPage)
    } else {
        next_btn
    };

    let pagination_bar = container(
        row![
            prev_btn,
            horizontal_space(),
            text(format!(
                "Pág {}/{} ({} símbolos)",
                current_page + 1,
                total_pages,
                total_count
            ))
            .size(10.5)
            .font(UI)
            .color(t.text2),
            horizontal_space(),
            next_btn,
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(Padding::from([4.0, 10.0]));

    // 7. Lista paginada de símbolos
    let mut list_col = column![].spacing(3).padding(Padding::from([0.0, 10.0]));
    if page_elements.is_empty() {
        list_col = list_col.push(
            container(
                text("Nenhum símbolo encontrado para os filtros atuais.")
                    .size(11.0)
                    .color(t.text2),
            )
            .padding(Padding::from([12.0, 4.0])),
        );
    } else {
        for el in page_elements {
            let is_selected = app.selected_canonical_id.as_deref() == Some(&el.id);
            let pin_count = el.terminals.len();
            let is_wireable = el.is_wireable();

            let badge_or_pins: Element<'_, Message> = if !is_wireable {
                container(text("Bloco").size(9.0).font(UI))
                    .style(theme::tag_badge)
                    .padding(Padding::from([1.0, 4.0]))
                    .into()
            } else {
                text(format!("{pin_count}b"))
                    .size(10.0)
                    .font(MONO)
                    .color(t.text2)
                    .into()
            };

            let item_row = container(
                row![
                    container(text(format!("[{}]", el.id)).size(9.5).font(MONO))
                        .style(theme::tag_badge)
                        .padding(Padding::from([1.0, 3.0])),
                    column![
                        text(el.name_pt.clone())
                            .size(11.0)
                            .font(UI)
                            .width(Length::Fill),
                        text(format!("{}: {}", el.family, el.kind))
                            .size(9.0)
                            .font(MONO)
                            .color(t.text2),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                    badge_or_pins,
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .style(theme::catalog_item(is_selected))
            .padding(Padding::from([3.0, 5.0]))
            .width(Length::Fill);

            let item_area = mouse_area(item_row)
                .interaction(iced::mouse::Interaction::Pointer)
                .on_press(Message::StartCanonicalSymbolDrag(el.id.clone()))
                .on_release(Message::SelectCanonicalSymbol(el.id.clone()));

            list_col = list_col.push(item_area);
        }
    }
    let list_scroll = container(scrollable(list_col)).height(Length::Fill);

    // 8. Preview box na parte inferior do painel
    let preview_content = if let Some(ref id) = app.selected_canonical_id {
        if let Some(dict) = crate::canonical::global_dictionary() {
            if let Some(el) = dict.get_by_id(id) {
                let svg_preview: Element<'_, Message> = if let Some(path) =
                    dict.resolve_svg_path(&el.id, None)
                {
                    if let Ok(raw) = std::fs::read_to_string(path) {
                        let themed = crate::symbols::adapt_svg_colors(&raw, is_dark);
                        container(
                            svg(svg::Handle::from_memory(themed.into_bytes()))
                                .width(Length::Fixed(80.0))
                                .height(Length::Fixed(60.0)),
                        )
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .into()
                    } else {
                        container(text("Erro ao carregar SVG").size(10.5).color(t.danger)).into()
                    }
                } else {
                    container(text("SVG não encontrado").size(10.5).color(t.text2)).into()
                };

                let term_names: Vec<&str> = el.terminals.iter().map(|t| t.name.as_str()).collect();
                let term_str = if !el.is_wireable() {
                    "Bloco de montagem visual (não simulável)".to_string()
                } else if term_names.is_empty() {
                    "Sem bornes".to_string()
                } else {
                    format!("Bornes: {}", term_names.join(", "))
                };

                column![
                    row![
                        text(el.name_pt.clone())
                            .size(11.5)
                            .font(Font {
                                weight: iced::font::Weight::Semibold,
                                ..UI
                            })
                            .width(Length::Fill),
                        text(format!("[{}]", el.id))
                            .size(10.0)
                            .font(MONO)
                            .color(t.primary),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        text(format!("{} • {}", el.family, el.kind))
                            .size(9.5)
                            .font(MONO)
                            .color(t.text2)
                            .width(Length::Fill),
                        text(term_str).size(9.5).font(MONO).color(t.text2),
                    ]
                    .align_y(Alignment::Center),
                    svg_preview,
                    button(text("Inserir no Esquema").size(11.0).font(UI))
                        .style(theme::primary)
                        .padding(Padding::from([3.0, 8.0]))
                        .width(Length::Fill)
                        .on_press(Message::InsertSelectedCanonicalSymbol),
                ]
                .spacing(3)
                .align_x(Alignment::Center)
            } else {
                column![text("Símbolo não encontrado").size(11.5).color(t.text2)]
            }
        } else {
            column![text("Dicionário não carregado").size(11.5).color(t.text2)]
        }
    } else {
        column![text("Selecione um símbolo acima").size(11.5).color(t.text2)]
            .align_x(Alignment::Center)
    };

    let preview_dock = container(preview_content)
        .style(theme::preview_box)
        .padding(Padding::from([6.0, 10.0]))
        .width(Length::Fill)
        .height(145);

    column![
        shortcuts_box,
        horizontal_rule(1),
        search_box,
        scope_scroll,
        fam_scroll,
        sub_scroll,
        horizontal_rule(1),
        pagination_bar,
        list_scroll,
        preview_dock
    ]
    .height(Length::Fill)
    .into()
}
