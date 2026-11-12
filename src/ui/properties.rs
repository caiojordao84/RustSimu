use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, pick_list, row,
    scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, CoilPattern, Message, PinSequenceOption, TimerScaleMultiplier};
use crate::symbols::{
    classify_apparatus_role, detect_contact_kind, detect_current_sequence, extract_tag_prefix,
    find_duplicate_apparatus, find_occupied_sequences_for_tag, find_parent_apparatus, find_symbol,
    is_coil_or_timer_apparatus, is_power_apparatus, is_terminal_item, list_available_parent_tags,
    next_sequential_tag_for_role,
};
use crate::theme::{self, tokens, MONO, UI};

pub fn view_properties_panel<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    if app.right_dock_collapsed {
        return container(
            button(crate::icons::view_icon(
                crate::icons::Icon::ChevronLeft,
                14,
                t.text,
            ))
            .style(theme::tool)
            .padding(Padding::from([8.0, 5.0]))
            .on_press(Message::ToggleRightDockCollapse),
        )
        .align_x(iced::alignment::Horizontal::Center)
        .style(theme::bar)
        .width(Length::Fixed(28.0))
        .height(Length::Fill)
        .into();
    }

    let header = container(
        row![
            text("Propriedades do Elemento").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::ChevronRight,
                12,
                t.text
            ))
            .style(theme::tool)
            .padding(Padding::from([3.0, 6.0]))
            .on_press(Message::ToggleRightDockCollapse),
        ]
        .align_y(Alignment::Center),
    )
    .style(theme::bar)
    .padding(Padding::from([5.0, 10.0]))
    .width(Length::Fill);

    // Barra de 3 Abas (Informações | Ref. Cruzada | Geral)
    let tab_btn = |idx: usize, label: &'static str| {
        let active = app.prop_tab == idx;
        column![
            button(text(label).size(12).font(UI))
                .style(theme::tab(active))
                .padding(Padding::from([6.0, 10.0]))
                .on_press(Message::SelectPropTab(idx)),
            container(Space::with_height(2))
                .style(theme::tab_underline(active))
                .width(Length::Fill),
        ]
        .spacing(0)
        .width(Length::Fill)
    };

    let tabs_bar = container(
        row![tab_btn(0, "Propriedades"), tab_btn(1, "Referência Cruzada"),]
            .spacing(4)
            .width(Length::Fill),
    )
    .style(theme::bar)
    .width(Length::Fill);

    // Corpo do painel de propriedades
    let panel_body: Element<'_, Message> = match app.prop_tab {
        0 => {
            // Aba Propriedades
            let mut body_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));

            if let Some(sel_id) = app.selected_id {
                if let Some(item) = app.doc.items.iter().find(|i| i.id == sel_id) {
                    let title = if let Some(sym) = find_symbol(item.tipo) {
                        sym.name.to_string()
                    } else if let Some(qet_id) = item.symbol_id_or_slug() {
                        if let Some(dict) = crate::canonical::global_dictionary() {
                            if let Some(el) = dict.get(qet_id) {
                                el.name_pt.clone()
                            } else {
                                "Elemento QET".to_string()
                            }
                        } else {
                            "Elemento QET".to_string()
                        }
                    } else if item.is_graphic() {
                        match (item.tipo, item.subvariant) {
                            (8, 0) => "Texto em Linha".to_string(),
                            (8, _) => "Campo de Texto".to_string(),
                            (5, _) => "Imagem".to_string(),
                            (1, _) => "Linha Gráfica".to_string(),
                            (2, _) => "Retângulo Gráfico".to_string(),
                            (3, _) => "Círculo / Elipse".to_string(),
                            (4, _) => "Arco Gráfico".to_string(),
                            _ => "Elemento Gráfico".to_string(),
                        }
                    } else if item.is_wire() {
                        "Cabo Condutor".to_string()
                    } else {
                        "Elemento".to_string()
                    };

                    let grid_ref = crate::xref::format_grid_ref_doc(
                        app.active_page_index,
                        item.x1 as f32,
                        item.y1 as f32,
                        &app.doc,
                    );

                    if app.selected_ids.len() > 1 {
                        body_col = body_col.push(
                            container(
                                row![
                                    container(crate::icons::view_icon(
                                        crate::icons::Icon::Select,
                                        13,
                                        t.primary
                                    ))
                                    .width(14)
                                    .height(14),
                                    text(format!(
                                        "Seleção múltipla: {} elementos selecionados",
                                        app.selected_ids.len()
                                    ))
                                    .size(11.5)
                                    .color(t.primary),
                                ]
                                .spacing(6)
                                .align_y(Alignment::Center),
                            )
                            .style(theme::card)
                            .padding(Padding::from([4.0, 8.0]))
                            .width(Length::Fill),
                        );
                    }

                    let selected_groups: std::collections::HashSet<u32> = app
                        .doc
                        .items
                        .iter()
                        .filter(|i| app.selected_ids.contains(&i.id))
                        .filter_map(|i| i.group_id)
                        .collect();

                    if !selected_groups.is_empty() {
                        let member_count = app
                            .doc
                            .items
                            .iter()
                            .filter(|i| {
                                i.group_id.is_some_and(|gid| selected_groups.contains(&gid))
                            })
                            .count();
                        let group_label = if selected_groups.len() == 1 {
                            format!(
                                "Grupo #{} ({} elementos)",
                                selected_groups.iter().next().unwrap(),
                                member_count
                            )
                        } else {
                            format!(
                                "{} Grupos ({} elementos)",
                                selected_groups.len(),
                                member_count
                            )
                        };
                        let mut group_row = row![
                            container(crate::icons::view_icon(
                                crate::icons::Icon::Group,
                                14,
                                t.primary
                            ))
                            .width(16)
                            .height(16),
                            text(group_label).size(11.5).color(t.text),
                            horizontal_space(),
                            button(
                                row![
                                    crate::icons::view_icon(
                                        crate::icons::Icon::Ungroup,
                                        12,
                                        t.danger
                                    ),
                                    text("Desagrupar").size(10.5).font(UI).color(t.danger),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center)
                            )
                            .style(theme::tool)
                            .padding(Padding::from([2.0, 6.0]))
                            .on_press(Message::UngroupSelected),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center);

                        if app.selected_ids.len() >= 2
                            && (selected_groups.len() > 1 || app.selected_ids.len() > member_count)
                        {
                            group_row = group_row.push(
                                button(
                                    row![
                                        crate::icons::view_icon(
                                            crate::icons::Icon::Group,
                                            12,
                                            t.primary
                                        ),
                                        text("Re-Agrupar").size(10.5).font(UI).color(t.primary),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                )
                                .style(theme::tool_active)
                                .padding(Padding::from([2.0, 6.0]))
                                .on_press(Message::GroupSelected),
                            );
                        }

                        body_col = body_col.push(
                            container(group_row)
                                .style(theme::card)
                                .padding(Padding::from([4.0, 8.0]))
                                .width(Length::Fill),
                        );
                    } else if app.selected_ids.len() >= 2 {
                        body_col = body_col.push(
                            container(
                                row![
                                    text("Elementos independentes").size(11.0).color(t.text2),
                                    horizontal_space(),
                                    button(
                                        row![
                                            crate::icons::view_icon(
                                                crate::icons::Icon::Group,
                                                12,
                                                t.primary
                                            ),
                                            text("Agrupar (Ctrl+G)")
                                                .size(10.5)
                                                .font(UI)
                                                .color(t.primary),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center)
                                    )
                                    .style(theme::tool_active)
                                    .padding(Padding::from([2.0, 6.0]))
                                    .on_press(Message::GroupSelected),
                                ]
                                .spacing(6)
                                .align_y(Alignment::Center),
                            )
                            .style(theme::card)
                            .padding(Padding::from([4.0, 8.0]))
                            .width(Length::Fill),
                        );
                    }

                    body_col = body_col.push(
                        text(format!("#{sel_id} · {title} · {grid_ref}"))
                            .size(13)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            }),
                    );

                    if let Some(qet_id) = item.symbol_id_or_slug() {
                        if let Some(dict) = crate::canonical::global_dictionary() {
                            if let Some(el) = dict.get(qet_id) {
                                if !el.is_wireable() {
                                    body_col = body_col.push(
                                            container(
                                                row![
                                                    crate::icons::view_icon(
                                                        crate::icons::Icon::Info,
                                                        12,
                                                        t.primary
                                                    ),
                                                    text("Bloco de Montagem Visual (Painel/Layout). Não simulável.")
                                                        .size(10.5)
                                                        .color(t.primary),
                                                ]
                                                .spacing(4)
                                                .align_y(Alignment::Center),
                                            )
                                            .style(theme::card)
                                            .padding(Padding::from([4.0, 6.0])),
                                        );
                                }
                            }
                        }
                    }

                    if item.is_graphic() {
                        if item.tipo == 8 {
                            let world_h = item.text_world_height();
                            let is_bold = item.text_is_bold();
                            let rot_idx = item.orientation.rem_euclid(4);
                            let mode_btn = |label: &'static str, target_sub: i32| {
                                button(text(label).size(10.5).font(UI))
                                    .style(if item.subvariant == target_sub {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([4.0, 8.0]))
                                    .width(Length::Fill)
                                    .on_press(Message::SetItemTextSubvariant(target_sub))
                            };
                            let size_btn = |label: &'static str, preset_h: f32| {
                                let active = (world_h - preset_h).abs() < 0.25;
                                button(text(label).size(10.5).font(MONO))
                                    .style(if active {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([3.0, 6.0]))
                                    .on_press(Message::SetItemTextSize(preset_h))
                            };
                            let rot_btn = |label: &'static str, idx: i32| {
                                button(text(label).size(10.5).font(MONO))
                                    .style(if rot_idx == idx {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([3.0, 6.0]))
                                    .on_press(Message::SetItemTextOrientation(idx))
                            };
                            let single_line_preview = item.trailing_str.replace('\n', " / ");
                            body_col = body_col.push(
                                column![
                                    row![mode_btn("Em Linha", 0), mode_btn("Campo (Caixa)", 1),]
                                        .spacing(4),
                                    text(if item.subvariant == 0 {
                                        "Conteúdo do Texto:"
                                    } else {
                                        "Conteúdo do Campo de Texto:"
                                    })
                                    .size(11)
                                    .color(t.text2),
                                    text_input(
                                        "Digite o texto ou anotação...",
                                        &single_line_preview
                                    )
                                    .on_input(Message::EditItemTrailingText)
                                    .padding(Padding::from([5.0, 8.0]))
                                    .size(12)
                                    .style(theme::field),
                                    button(
                                        row![
                                            crate::icons::view_icon(
                                                crate::icons::Icon::Pencil,
                                                12,
                                                t.on_primary
                                            ),
                                            text("Abrir Editor Multilinha...").size(11.0).font(UI),
                                        ]
                                        .spacing(6)
                                        .align_y(Alignment::Center),
                                    )
                                    .style(theme::primary)
                                    .padding(Padding::from([5.0, 10.0]))
                                    .width(Length::Fill)
                                    .on_press(Message::OpenTextEditorModal(item.id)),
                                    row![
                                        text("Tamanho:").size(11).color(t.text2),
                                        horizontal_space(),
                                        text(format!("{world_h:.1} u"))
                                            .size(11)
                                            .font(MONO)
                                            .color(t.text),
                                    ]
                                    .align_y(Alignment::Center),
                                    row![
                                        size_btn("P", 2.4),
                                        size_btn("M", 3.2),
                                        size_btn("G", 4.8),
                                        size_btn("GG", 6.4),
                                        button(text("-").size(11).font(MONO))
                                            .style(theme::tool)
                                            .padding(Padding::from([3.0, 7.0]))
                                            .on_press(Message::AdjustItemTextSize(-0.4)),
                                        button(text("+").size(11).font(MONO))
                                            .style(theme::tool)
                                            .padding(Padding::from([3.0, 7.0]))
                                            .on_press(Message::AdjustItemTextSize(0.4)),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                    row![
                                        button(
                                            text(if is_bold { "Negrito: Sim" } else { "Negrito" })
                                                .size(10.5)
                                                .font(Font {
                                                    weight: if is_bold {
                                                        iced::font::Weight::Bold
                                                    } else {
                                                        iced::font::Weight::Normal
                                                    },
                                                    ..UI
                                                }),
                                        )
                                        .style(if is_bold {
                                            theme::tool_active
                                        } else {
                                            theme::tool
                                        })
                                        .padding(Padding::from([4.0, 8.0]))
                                        .on_press(Message::ToggleItemTextBold),
                                        horizontal_space(),
                                        rot_btn("0°", 0),
                                        rot_btn("90°", 1),
                                        rot_btn("180°", 2),
                                        rot_btn("270°", 3),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                ]
                                .spacing(6),
                            );
                        } else if item.tipo == 5 {
                            let img_label = if item.trailing_str.trim().is_empty() {
                                "Nenhuma imagem vinculada".to_string()
                            } else {
                                std::path::Path::new(&item.trailing_str)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| item.trailing_str.clone())
                            };
                            body_col = body_col.push(
                                column![
                                    text("Arquivo da Imagem:").size(11).color(t.text2),
                                    text(img_label).size(11.5).color(t.text).font(MONO),
                                    button(text("Substituir Imagem...").size(11.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([5.0, 10.0]))
                                        .on_press(Message::InsertImageDialog),
                                ]
                                .spacing(4),
                            );
                        } else if item.tipo == 4 {
                            let (start_deg, sweep_deg) =
                                if item.aux_coords[0] == 0 && item.aux_coords[1] == 0 {
                                    (0, 180)
                                } else {
                                    (item.aux_coords[0], item.aux_coords[1])
                                };
                            let arc_preset = |label: &'static str, preset: i32| {
                                button(text(label).size(10.5).font(MONO))
                                    .style(if sweep_deg == preset {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([3.0, 6.0]))
                                    .on_press(Message::SetItemArcSweepPreset(preset))
                            };
                            body_col = body_col.push(
                                column![
                                    row![
                                        column![
                                            text("Ângulo Inicial (graus):").size(11).color(t.text2),
                                            text_input("0", &start_deg.to_string())
                                                .on_input(Message::EditItemArcStart)
                                                .padding(Padding::from([5.0, 8.0]))
                                                .size(12)
                                                .style(theme::field),
                                        ]
                                        .spacing(4)
                                        .width(Length::Fill),
                                        column![
                                            text("Varredura (graus):").size(11).color(t.text2),
                                            text_input("180", &sweep_deg.to_string())
                                                .on_input(Message::EditItemArcSweep)
                                                .padding(Padding::from([5.0, 8.0]))
                                                .size(12)
                                                .style(theme::field),
                                        ]
                                        .spacing(4)
                                        .width(Length::Fill),
                                    ]
                                    .spacing(6),
                                    row![
                                        arc_preset("90°", 90),
                                        arc_preset("180°", 180),
                                        arc_preset("270°", 270),
                                        button(text("Inverter").size(10.5).font(UI))
                                            .style(theme::tool)
                                            .padding(Padding::from([3.0, 7.0]))
                                            .on_press(Message::SetItemArcSweepPreset(-sweep_deg)),
                                        button(text("+90°").size(10.5).font(MONO))
                                            .style(theme::tool)
                                            .padding(Padding::from([3.0, 6.0]))
                                            .on_press(Message::RotateClockwise),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                ]
                                .spacing(6),
                            );
                        }

                        if matches!(item.tipo, 1..=4) {
                            let cur_style = item.graphic_line_style();
                            let cur_width = item.graphic_stroke_width();
                            let style_btn = |st: crate::model::GraphicLineStyle| {
                                button(text(st.label()).size(10.5).font(UI))
                                    .style(if cur_style == st {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([4.0, 6.0]))
                                    .on_press(Message::SetItemGraphicLineStyle(st))
                            };
                            let width_btn = |label: &'static str, w: f32| {
                                button(text(label).size(10.5).font(MONO))
                                    .style(if (cur_width - w).abs() < 0.15 {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([3.0, 6.0]))
                                    .on_press(Message::SetItemGraphicStrokeWidth(w))
                            };
                            body_col = body_col.push(
                                column![
                                    text("Estilo do Traço:").size(11).color(t.text2),
                                    row![
                                        style_btn(crate::model::GraphicLineStyle::Solid),
                                        style_btn(crate::model::GraphicLineStyle::Dash),
                                    ]
                                    .spacing(4),
                                    row![
                                        style_btn(crate::model::GraphicLineStyle::Dot),
                                        style_btn(crate::model::GraphicLineStyle::DashDot),
                                    ]
                                    .spacing(4),
                                    row![
                                        text("Espessura:").size(11).color(t.text2),
                                        horizontal_space(),
                                        text(format!("{cur_width:.1} px"))
                                            .size(11)
                                            .font(MONO)
                                            .color(t.text),
                                    ]
                                    .align_y(Alignment::Center),
                                    row![
                                        width_btn("1.0", 1.0),
                                        width_btn("1.5", 1.5),
                                        width_btn("2.0", 2.0),
                                        width_btn("3.0", 3.0),
                                        button(text("-").size(11).font(MONO))
                                            .style(theme::tool)
                                            .padding(Padding::from([3.0, 7.0]))
                                            .on_press(Message::AdjustItemGraphicStrokeWidth(-0.5)),
                                        button(text("+").size(11).font(MONO))
                                            .style(theme::tool)
                                            .padding(Padding::from([3.0, 7.0]))
                                            .on_press(Message::AdjustItemGraphicStrokeWidth(0.5)),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                ]
                                .spacing(5),
                            );
                        }

                        if matches!(item.tipo, 2 | 3) {
                            let is_filled = item.graphic_has_fill();
                            let mut opts_row = row![button(
                                text(if is_filled {
                                    "Preenchimento: Sim"
                                } else {
                                    "Sem Preenchimento"
                                })
                                .size(10.5)
                                .font(UI),
                            )
                            .style(if is_filled {
                                theme::tool_active
                            } else {
                                theme::tool
                            })
                            .padding(Padding::from([4.0, 8.0]))
                            .on_press(Message::ToggleItemGraphicFill)]
                            .spacing(4)
                            .align_y(Alignment::Center);
                            if item.tipo == 2 {
                                let is_round = item.graphic_rounded_corners();
                                opts_row = opts_row.push(
                                    button(
                                        text(if is_round {
                                            "Arredondado"
                                        } else {
                                            "Canto Reto"
                                        })
                                        .size(10.5)
                                        .font(UI),
                                    )
                                    .style(if is_round {
                                        theme::tool_active
                                    } else {
                                        theme::tool
                                    })
                                    .padding(Padding::from([4.0, 8.0]))
                                    .on_press(Message::ToggleItemRectRounded),
                                );
                            }
                            body_col = body_col.push(opts_row);
                        }

                        let dim_summary = if item.tipo == 1 {
                            let len =
                                ((item.x2 - item.x1) as f32).hypot((item.y2 - item.y1) as f32);
                            format!(
                                "P1({},{}) -> P2({},{}) | L={len:.1}",
                                item.x1, item.y1, item.x2, item.y2
                            )
                        } else {
                            format!(
                                "({},{}) -> ({},{}) | {}x{}",
                                item.x1,
                                item.y1,
                                item.x2,
                                item.y2,
                                (item.x2 - item.x1).abs(),
                                (item.y2 - item.y1).abs()
                            )
                        };

                        body_col = body_col.push(
                            column![
                                text(dim_summary).size(11).font(MONO).color(t.text2),
                                button(
                                    row![
                                        crate::icons::view_icon(
                                            crate::icons::Icon::Palette,
                                            13,
                                            t.text
                                        ),
                                        text("Alterar Cor...").size(11.5).font(UI),
                                    ]
                                    .spacing(6)
                                    .align_y(Alignment::Center),
                                )
                                .style(theme::tool)
                                .padding(Padding::from([5.0, 10.0]))
                                .on_press(Message::ColorElement),
                            ]
                            .spacing(6),
                        );
                    } else if !item.is_wire() && !item.is_junction() {
                        // 1. Nome do Elemento (Tag)
                        body_col = body_col.push(
                            column![
                                text("Nome:").size(11).color(t.text2),
                                text_input("Nome / Tag (ex: -KM1, -Q1)", &item.tag)
                                    .on_input(Message::EditTag)
                                    .padding(Padding::from([5.0, 8.0]))
                                    .size(12)
                                    .style(theme::field),
                                checkbox("Exibir no esquema", item.show_tag == 1)
                                    .on_toggle(Message::ToggleShowTag)
                                    .text_size(11.5),
                            ]
                            .spacing(4),
                        );

                        // Validação Inteligente da Tag: Verificação de Bobina e Anti-Duplicidade
                        let role = classify_apparatus_role(item.tipo);
                        if role.is_subordinate_contact() {
                            // Contato ou polo que necessita de aparelho pai (ex: bobina -KM1)
                            if let Some(parent) = find_parent_apparatus(&app.doc.items, item) {
                                let parent_name = find_symbol(parent.tipo)
                                    .map(|s| s.name)
                                    .unwrap_or("Aparelho");
                                let p_page = app.active_page_index;
                                let p_id = parent.id;
                                body_col = body_col.push(
                                    row![
                                        crate::icons::view_icon(
                                            crate::icons::Icon::Check,
                                            12,
                                            t.success
                                        ),
                                        text(format!(
                                            "Vinculado a '{}' ({})",
                                            parent.tag, parent_name
                                        ))
                                        .size(10.5)
                                        .color(t.success),
                                        horizontal_space(),
                                        button(text("Ir").size(10.0).font(UI))
                                            .style(theme::tool)
                                            .padding(Padding::from([2.0, 6.0]))
                                            .on_press(Message::NavigateToElement {
                                                page_index: p_page,
                                                item_id: p_id,
                                            }),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                );
                            } else {
                                let parent_display = role.parent_display_name();
                                let mut alert_col = column![row![
                                    crate::icons::view_icon(
                                        crate::icons::Icon::Warning,
                                        12,
                                        t.amber
                                    ),
                                    text(format!(
                                        "Contato sem bobina: Nenhuma {} com a tag '{}' no esquema.",
                                        parent_display, item.tag
                                    ))
                                    .size(10.5)
                                    .color(t.amber),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),]
                                .spacing(4);

                                let available = list_available_parent_tags(&app.doc.items, role);
                                if !available.is_empty() {
                                    let mut link_row =
                                        row![text("Vincular a:").size(10.5).color(t.text2)]
                                            .spacing(4)
                                            .align_y(Alignment::Center);
                                    for p_tag in available {
                                        let p_tag_clone = p_tag.clone();
                                        link_row = link_row.push(
                                            button(text(p_tag).size(10.0).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::EditTag(p_tag_clone)),
                                        );
                                    }
                                    alert_col = alert_col.push(link_row);
                                }

                                body_col = body_col.push(
                                    container(alert_col)
                                        .padding(Padding::from([4.0, 6.0]))
                                        .style(theme::card),
                                );
                            }
                        } else if role.is_independent_apparatus() {
                            // Aparelho primário (Bobina, Disjuntor, Motor, Botão...)
                            if find_duplicate_apparatus(&app.doc.items, item) {
                                let prefix = extract_tag_prefix(&item.tag);
                                let suggested_tag =
                                    next_sequential_tag_for_role(&app.doc.items, role, prefix);
                                let suggested_tag_clone = suggested_tag.clone();
                                body_col = body_col.push(
                                    container(
                                        column![
                                            row![
                                                crate::icons::view_icon(
                                                    crate::icons::Icon::Warning,
                                                    12,
                                                    t.amber
                                                ),
                                                text(format!(
                                                    "Tag duplicada: já existe outro aparelho primário com a tag '{}'",
                                                    item.tag
                                                ))
                                                .size(10.5)
                                                .color(t.amber),
                                            ]
                                            .spacing(4)
                                            .align_y(Alignment::Center),
                                            button(
                                                row![
                                                    crate::icons::view_icon(
                                                        crate::icons::Icon::Bolt,
                                                        12,
                                                        t.primary
                                                    ),
                                                    text(format!("Renomear para {suggested_tag}"))
                                                        .size(10.5)
                                                        .font(UI),
                                                ]
                                                .spacing(4)
                                                .align_y(Alignment::Center)
                                            )
                                            .style(theme::tool_active)
                                            .padding(Padding::from([2.0, 6.0]))
                                            .on_press(Message::EditTag(suggested_tag_clone)),
                                        ]
                                        .spacing(4),
                                    )
                                    .padding(Padding::from([4.0, 6.0]))
                                    .style(theme::card),
                                );
                            } else {
                                let sub_contacts_count = app
                                    .doc
                                    .items
                                    .iter()
                                    .filter(|other| {
                                        other.id != item.id
                                            && other
                                                .tag
                                                .trim()
                                                .eq_ignore_ascii_case(item.tag.trim())
                                    })
                                    .count();
                                if sub_contacts_count > 0 {
                                    body_col = body_col.push(
                                        row![
                                            crate::icons::view_icon(
                                                crate::icons::Icon::Check,
                                                12,
                                                t.success
                                            ),
                                            text(format!(
                                                "Contatos associados no esquema: {} elemento(s)",
                                                sub_contacts_count
                                            ))
                                            .size(10.5)
                                            .color(t.success),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center),
                                    );
                                }
                            }
                        }

                        // 2. Função do Elemento
                        let func_str =
                            item.function
                                .as_deref()
                                .unwrap_or(if item.value.is_empty() {
                                    ""
                                } else {
                                    &item.value
                                });
                        body_col = body_col.push(
                            column![
                                text("Função:").size(11).color(t.text2),
                                text_input(
                                    "Função descritiva (ex: Alimentação Motor Principal)",
                                    func_str,
                                )
                                .on_input(Message::EditFunction)
                                .padding(Padding::from([5.0, 8.0]))
                                .size(12)
                                .style(theme::field),
                                checkbox("Exibir no esquema", item.show_value == 1)
                                    .on_toggle(Message::ToggleShowValue)
                                    .text_size(11.5),
                            ]
                            .spacing(4),
                        );

                        // 3. Padrões de Bobinas Especiais (IEC / EN 50012)
                        let is_coil_timer = is_coil_or_timer_apparatus(item);
                        if is_coil_timer {
                            body_col = body_col.push(horizontal_rule(1));
                            let pattern = CoilPattern::from_item(item);
                            let coil_options = CoilPattern::ALL.to_vec();
                            let pick =
                                pick_list(coil_options, Some(pattern), Message::SetCoilPattern)
                                    .text_size(11.0)
                                    .padding(Padding::from([3.0, 6.0]))
                                    .width(Length::Fill);

                            let mut coil_box = column![
                                text("Padrões de Bobinas Especiais (IEC / EN 50012):")
                                    .size(11.5)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    }),
                                pick,
                            ]
                            .spacing(4);

                            if pattern != CoilPattern::Standard && pattern != CoilPattern::Latching
                            {
                                let base_preset = item.timer_base_value();
                                let base_str =
                                    crate::sim::timers::format_timer_seconds(base_preset);
                                let scale = item.timer_scale_multiplier();
                                let eff_sec = base_preset * scale.factor();
                                let eff_ms = (eff_sec * 1000.0).round() as u64;
                                let eff_sec_str = crate::sim::timers::format_timer_seconds(eff_sec);

                                let base_row = row![
                                    text(if pattern == CoilPattern::Cyclic {
                                        "Tempo Ligado (T_ON):"
                                    } else {
                                        "Tempo Base:"
                                    })
                                    .size(10.5)
                                    .color(t.text2),
                                    horizontal_space(),
                                    button(text("-").size(10.5).font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    }))
                                    .style(theme::tool)
                                    .padding(Padding::from([2.0, 6.0]))
                                    .on_press(Message::AdjustTimerPreset(-1.0)),
                                    text_input("ex: 3", &base_str)
                                        .on_input(Message::SetTimerBaseValue)
                                        .padding(Padding::from([2.0, 5.0]))
                                        .size(11.0)
                                        .width(55)
                                        .style(theme::field),
                                    button(text("+").size(10.5).font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    }))
                                    .style(theme::tool)
                                    .padding(Padding::from([2.0, 6.0]))
                                    .on_press(Message::AdjustTimerPreset(1.0)),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center);

                                let step_chips = row![
                                    text("Passo:").size(9.5).color(t.text2),
                                    button(text("-5s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::AdjustTimerPreset(-5.0)),
                                    button(text("-0.5s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::AdjustTimerPreset(-0.5)),
                                    button(text("+0.5s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::AdjustTimerPreset(0.5)),
                                    button(text("+5s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::AdjustTimerPreset(5.0)),
                                ]
                                .spacing(3)
                                .align_y(Alignment::Center);

                                let scale_options = TimerScaleMultiplier::ALL.to_vec();
                                let scale_pick = pick_list(
                                    scale_options.clone(),
                                    Some(scale),
                                    Message::SetTimerScaleMultiplier,
                                )
                                .text_size(10.5)
                                .padding(Padding::from([2.0, 5.0]))
                                .width(Length::Fill);

                                let scale_box = column![
                                    text("Escala Multiplicadora:").size(10.0).color(t.text2),
                                    scale_pick,
                                ]
                                .spacing(2);

                                let eff_badge = container(
                                    row![
                                        text(if pattern == CoilPattern::Cyclic {
                                            "T_ON Efetivo:"
                                        } else {
                                            "Tempo Efetivo:"
                                        })
                                        .size(10.0)
                                        .color(t.text2),
                                        horizontal_space(),
                                        text(format!("{eff_sec_str} s ({eff_ms} ms)"))
                                            .size(11.0)
                                            .font(Font {
                                                weight: iced::font::Weight::Bold,
                                                ..MONO
                                            })
                                            .color(t.primary),
                                    ]
                                    .align_y(Alignment::Center),
                                )
                                .style(theme::card)
                                .padding(Padding::from([3.0, 6.0]))
                                .width(Length::Fill);

                                let presets_row1 = row![
                                    button(text("0.5s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(0.5)),
                                    button(text("1s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(1.0)),
                                    button(text("2s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(2.0)),
                                    button(text("3s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(3.0)),
                                ]
                                .spacing(3);

                                let presets_row2 = row![
                                    button(text("5s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(5.0)),
                                    button(text("10s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(10.0)),
                                    button(text("30s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(30.0)),
                                    button(text("60s").size(9.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 4.0]))
                                        .on_press(Message::ApplyTimerQuickPreset(60.0)),
                                ]
                                .spacing(3);

                                let presets_box = column![
                                    text("Atalhos Rápidos de Tempo:").size(10.0).color(t.text2),
                                    presets_row1,
                                    presets_row2,
                                ]
                                .spacing(3);

                                let mut timer_ctrl_box = column![
                                    text("Controle de Tempo:").size(11.0).font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    }),
                                    base_row,
                                    step_chips,
                                    scale_box,
                                    eff_badge,
                                    presets_box,
                                ]
                                .spacing(5);

                                // Se for temporizador cíclico, adiciona controles de T_OFF
                                if pattern == CoilPattern::Cyclic {
                                    let base_off = item.timer_off_base_value();
                                    let base_off_str =
                                        crate::sim::timers::format_timer_seconds(base_off);
                                    let scale_off = item.timer_off_scale_multiplier();
                                    let eff_off_sec = base_off * scale_off.factor();
                                    let eff_off_ms = (eff_off_sec * 1000.0).round() as u64;
                                    let eff_off_sec_str =
                                        crate::sim::timers::format_timer_seconds(eff_off_sec);

                                    let off_base_row = row![
                                        text("Tempo Desligado (T_OFF):").size(10.5).color(t.text2),
                                        horizontal_space(),
                                        button(text("-").size(10.5).font(Font {
                                            weight: iced::font::Weight::Bold,
                                            ..UI
                                        }))
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 6.0]))
                                        .on_press(Message::AdjustTimerCyclicOff(-1.0)),
                                        text_input("ex: 1", &base_off_str)
                                            .on_input(Message::SetTimerCyclicOffBaseValue)
                                            .padding(Padding::from([2.0, 5.0]))
                                            .size(11.0)
                                            .width(55)
                                            .style(theme::field),
                                        button(text("+").size(10.5).font(Font {
                                            weight: iced::font::Weight::Bold,
                                            ..UI
                                        }))
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 6.0]))
                                        .on_press(Message::AdjustTimerCyclicOff(1.0)),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center);

                                    let scale_off_pick = pick_list(
                                        scale_options,
                                        Some(scale_off),
                                        Message::SetTimerCyclicOffMultiplier,
                                    )
                                    .text_size(10.5)
                                    .padding(Padding::from([2.0, 5.0]))
                                    .width(Length::Fill);

                                    let eff_off_badge = container(
                                        row![
                                            text("T_OFF Efetivo:").size(10.0).color(t.text2),
                                            horizontal_space(),
                                            text(format!("{eff_off_sec_str} s ({eff_off_ms} ms)"))
                                                .size(11.0)
                                                .font(Font {
                                                    weight: iced::font::Weight::Bold,
                                                    ..MONO
                                                })
                                                .color(t.primary),
                                        ]
                                        .align_y(Alignment::Center),
                                    )
                                    .style(theme::card)
                                    .padding(Padding::from([3.0, 6.0]))
                                    .width(Length::Fill);

                                    let period = (eff_sec + eff_off_sec).max(0.01);
                                    let freq = 1.0 / period;
                                    let duty = (eff_sec / period) * 100.0;

                                    let cyclic_summary = container(
                                        text(format!(
                                            "Ciclo T={period:.1}s | f={freq:.2}Hz | D={duty:.0}%"
                                        ))
                                        .size(9.5)
                                        .font(MONO)
                                        .color(t.text2),
                                    )
                                    .style(theme::card)
                                    .padding(Padding::from([2.0, 5.0]))
                                    .width(Length::Fill);

                                    timer_ctrl_box = timer_ctrl_box.push(horizontal_rule(1)).push(
                                        column![
                                            off_base_row,
                                            scale_off_pick,
                                            eff_off_badge,
                                            cyclic_summary,
                                        ]
                                        .spacing(3),
                                    );
                                }

                                // Monitoramento ao vivo durante a simulação
                                if app.sim_running {
                                    let sim_st = app.doc.sim.get(item.id);
                                    let preset_ms = crate::sim::timers::timer_preset_ms(
                                        &item.pins,
                                        &item.value,
                                    );
                                    let elapsed_ms = sim_st.timer_elapsed_ms;
                                    let elapsed_sec = elapsed_ms as f64 / 1000.0;
                                    let eff_preset_sec = preset_ms as f64 / 1000.0;

                                    let (status_text, status_color, pct) = if pattern
                                        == CoilPattern::Cyclic
                                    {
                                        let off_preset_ms =
                                            crate::sim::timers::timer_off_preset_ms(&item.pins);
                                        let total_cycle_ms =
                                            preset_ms.saturating_add(off_preset_ms).max(100);
                                        let phase_str = if sim_st.timer_phase == 0 {
                                            "T_ON (Pulso)"
                                        } else {
                                            "T_OFF (Pausa)"
                                        };
                                        if !sim_st.active {
                                            ("Em Repouso / Desenergizado".to_string(), t.text2, 0.0)
                                        } else {
                                            let cur = (elapsed_ms % total_cycle_ms) as f64 / 1000.0;
                                            let phase_dur = if sim_st.timer_phase == 0 {
                                                eff_preset_sec
                                            } else {
                                                off_preset_ms as f64 / 1000.0
                                            };
                                            let phase_pct =
                                                (cur / phase_dur.max(0.1)).clamp(0.0, 1.0);
                                            (
                                                format!("{phase_str}: {cur:.1}s / {phase_dur:.1}s"),
                                                t.amber,
                                                phase_pct,
                                            )
                                        }
                                    } else if !sim_st.active {
                                        ("Em Repouso / Desenergizado".to_string(), t.text2, 0.0)
                                    } else if elapsed_ms >= preset_ms {
                                        ("Disparado / Concluído".to_string(), t.success, 1.0)
                                    } else {
                                        let frac = (elapsed_ms as f64 / preset_ms.max(1) as f64)
                                            .clamp(0.0, 1.0);
                                        (
                                            format!(
                                                "Contando: {elapsed_sec:.1}s / {eff_preset_sec:.1}s"
                                            ),
                                            t.amber,
                                            frac,
                                        )
                                    };

                                    let bar_total_w = 190.0;
                                    let bar_filled_w = ((bar_total_w * pct as f32)
                                        .max(if pct > 0.0 { 3.0 } else { 0.0 }))
                                    .min(bar_total_w);

                                    let sim_monitor_box = column![
                                        horizontal_rule(1),
                                        row![
                                            text("Monitoramento ao Vivo:").size(10.5).font(Font {
                                                weight: iced::font::Weight::Bold,
                                                ..UI
                                            }),
                                            horizontal_space(),
                                            text(format!("{:.0}%", pct * 100.0))
                                                .size(10.5)
                                                .font(MONO)
                                                .color(status_color),
                                        ]
                                        .align_y(Alignment::Center),
                                        text(status_text).size(10.0).color(status_color),
                                        container(row![
                                            container(Space::with_width(Length::Fixed(
                                                bar_filled_w
                                            )))
                                            .height(Length::Fixed(6.0))
                                            .style(
                                                move |_| iced::widget::container::Style {
                                                    background: Some(iced::Background::Color(
                                                        status_color
                                                    )),
                                                    border: iced::border::rounded(3.0),
                                                    ..Default::default()
                                                }
                                            ),
                                            Space::with_width(Length::Fill),
                                        ])
                                        .width(Length::Fixed(bar_total_w))
                                        .height(Length::Fixed(6.0))
                                        .style(
                                            move |_| iced::widget::container::Style {
                                                background: Some(iced::Background::Color(t.border)),
                                                border: iced::border::rounded(3.0),
                                                ..Default::default()
                                            }
                                        ),
                                        row![button(
                                            text("Reiniciar Temporizador").size(9.0).font(UI)
                                        )
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 5.0]))
                                        .on_press(Message::ResetTimerSimulation(item.id)),]
                                    ]
                                    .spacing(3);

                                    timer_ctrl_box = timer_ctrl_box.push(sim_monitor_box);
                                }

                                coil_box = coil_box.push(timer_ctrl_box);
                            }

                            coil_box = coil_box.push(
                                row![
                                    text("Preset Bornes:").size(10.0).color(t.text2),
                                    button(text("A1-A2 (IEC)").size(9.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 5.0]))
                                        .on_press(Message::ApplyIecNumberingPreset("coil")),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );

                            body_col = body_col.push(
                                container(coil_box)
                                    .style(theme::card)
                                    .padding(Padding::from([6.0, 8.0]))
                                    .width(Length::Fill),
                            );
                        } else if !item.is_lamp_or_signaling() && !is_terminal_item(item) {
                            // Valor / Referência técnica para outros aparelhos (motores, disjuntores, etc.)
                            body_col = body_col.push(
                                column![
                                    text("Valor Técnico / Referência:").size(11).color(t.text2),
                                    text_input("Valor", &item.value)
                                        .on_input(Message::EditValue)
                                        .padding(Padding::from([5.0, 8.0]))
                                        .size(12)
                                        .style(theme::field),
                                ]
                                .spacing(4),
                            );
                        }

                        // 4. Sinalizadores Luminosos e Pulsantes (IEC / EN 60073)
                        let is_lamp = item.is_lamp_or_signaling();
                        if is_lamp {
                            body_col = body_col.push(horizontal_rule(1));
                            let mut lamp_box =
                                column![text("Sinalizador Luminoso (IEC / EN 60073):")
                                    .size(11.5)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    }),]
                                .spacing(4);

                            let color_btn = |code: i32, label: &'static str| {
                                let is_sel = item.color == code;
                                button(
                                    row![
                                        container(Space::with_width(8).height(8))
                                            .style(theme::lamp_swatch(t.lamp_color(code))),
                                        text(label).size(10.0).font(UI),
                                    ]
                                    .spacing(3)
                                    .align_y(Alignment::Center),
                                )
                                .style(if is_sel {
                                    theme::tool_active
                                } else {
                                    theme::tool
                                })
                                .padding(Padding::from([2.0, 4.0]))
                                .on_press(Message::SetLampColor(code))
                            };

                            let colors_row = row![
                                color_btn(1, "Verde"),
                                color_btn(2, "Vermelho"),
                                color_btn(3, "Amarelo"),
                                color_btn(4, "Azul"),
                                color_btn(0, "Branco"),
                                color_btn(5, "Laranja"),
                            ]
                            .spacing(3);

                            lamp_box = lamp_box.push(colors_row);

                            let is_pulsing = item.extra_int1 == 1;
                            lamp_box = lamp_box.push(
                                checkbox("Sinalizador Pulsante (Intermitente)", is_pulsing)
                                    .on_toggle(Message::TogglePulsingSignaling)
                                    .text_size(11.0),
                            );

                            if is_pulsing {
                                lamp_box = lamp_box.push(
                                    row![
                                        text("Tempo de Pulso (s):").size(11.0).color(t.text2),
                                        text_input("ex: 0.5", &item.value)
                                            .on_input(Message::SetPulseDuration)
                                            .padding(Padding::from([3.0, 6.0]))
                                            .size(11.5)
                                            .width(80)
                                            .style(theme::field),
                                    ]
                                    .spacing(6)
                                    .align_y(Alignment::Center),
                                );
                            }

                            lamp_box = lamp_box.push(
                                row![
                                    text("Preset Bornes:").size(10.0).color(t.text2),
                                    button(text("X1-X2 (IEC)").size(9.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 5.0]))
                                        .on_press(Message::ApplyIecNumberingPreset("lamp")),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );

                            body_col = body_col.push(
                                container(lamp_box)
                                    .style(theme::card)
                                    .padding(Padding::from([6.0, 8.0]))
                                    .width(Length::Fill),
                            );
                        }

                        // 5. Borne N (Identificação estruturada IEC 60947-1 e IEC 81346)
                        let has_pins = if let Some(sym) = find_symbol(item.tipo) {
                            !sym.pins.is_empty()
                        } else if let Some(id_or_slug) = item.symbol_id_or_slug() {
                            crate::canonical::global_dictionary()
                                .and_then(|dict| dict.get(id_or_slug))
                                .map(|el| !el.terminals.is_empty())
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        if has_pins {
                            body_col = body_col.push(horizontal_rule(1));
                            body_col = body_col.push(
                                text("Numeração dos Bornes (IEC 60947-1 / IEC 81346):")
                                    .size(12)
                                    .font(Font {
                                        weight: iced::font::Weight::Semibold,
                                        ..UI
                                    }),
                            );

                            if let Some(sym) = find_symbol(item.tipo) {
                                for (p_idx, p_def) in sym.pins.iter().enumerate() {
                                    let current_pin_val =
                                        item.pins.get(p_idx).cloned().unwrap_or_default();
                                    body_col = body_col.push(
                                        row![
                                            text(format!("Borne {}:", p_idx + 1))
                                                .size(11)
                                                .font(MONO)
                                                .width(Length::Fixed(65.0)),
                                            text_input(p_def.default_label, &current_pin_val)
                                                .on_input(move |v| Message::EditPin(p_idx, v))
                                                .padding(Padding::from([3.0, 6.0]))
                                                .size(11.5)
                                                .style(theme::field),
                                        ]
                                        .spacing(6)
                                        .align_y(Alignment::Center),
                                    );
                                }

                                // Circuitos de Potência (Força - IEC 60947-1): Presets e regras
                                if is_power_apparatus(item) {
                                    let power_box = column![
                                        text("Circuitos de Potência (Força - IEC 60947-1):")
                                            .size(11.0)
                                            .font(Font {
                                                weight: iced::font::Weight::Bold,
                                                ..UI
                                            })
                                            .color(t.primary),
                                        text("• Ímpares (1, 3, 5 / L1, L2, L3): Entrada de alimentação\n• Pares (2, 4, 6 / T1, T2, T3): Saída para carga/motor")
                                            .size(10.0)
                                            .color(t.text2),
                                        row![
                                            text("Presets:").size(10.0).color(t.text2),
                                            button(text("1-2, 3-4, 5-6").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("power_3p")),
                                            button(text("L1-T1, L2-T2, L3-T3").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("power_3p_phases")),
                                            button(text("1-2").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("power_1p")),
                                            button(text("L1-T1").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("power_1p_phases")),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center),
                                    ]
                                    .spacing(4);

                                    body_col = body_col.push(
                                        container(power_box)
                                            .style(theme::card)
                                            .padding(Padding::from([5.0, 7.0]))
                                            .width(Length::Fill),
                                    );
                                }

                                // Dropdown sequencial inteligente IEC 60947 (1 a 9) para Contatos de Comando
                                if let Some(kind) = detect_contact_kind(item.tipo, &sym.pins) {
                                    let occupied = find_occupied_sequences_for_tag(
                                        &app.doc.items,
                                        &item.tag,
                                        Some(item.id),
                                    );
                                    let current_seq = detect_current_sequence(&item.pins, kind);
                                    let suggested_digit =
                                        (1..=9).find(|d| !occupied.contains(d)).unwrap_or(1);

                                    let mut options = Vec::with_capacity(9);
                                    for d in 1..=9 {
                                        let in_use = occupied.contains(&d);
                                        let is_sug = !in_use && d == suggested_digit;
                                        let preview = kind.generate_pins(d).join("-");
                                        let label = if in_use {
                                            format!("Sequência {} * ({} em uso)", d, preview)
                                        } else if is_sug {
                                            format!("Sequência {} ({} sugerido)", d, preview)
                                        } else {
                                            format!("Sequência {} ({})", d, preview)
                                        };
                                        options.push(PinSequenceOption {
                                            digit: d,
                                            in_use,
                                            is_suggested: is_sug,
                                            label,
                                        });
                                    }

                                    let selected_opt = current_seq.and_then(|s| {
                                        options.iter().find(|o| o.digit == s).cloned()
                                    });

                                    let seq_pick = pick_list(options, selected_opt, |opt| {
                                        Message::SelectPinSequence(opt.digit)
                                    })
                                    .text_size(11.0)
                                    .padding(Padding::from([3.0, 6.0]))
                                    .width(Length::Fill);

                                    let mut cmd_box = column![
                                        text("Circuitos de Comando (Contatos - IEC 60947-1):")
                                            .size(11.0)
                                            .font(Font {
                                                weight: iced::font::Weight::Bold,
                                                ..UI
                                            })
                                            .color(t.primary),
                                        text("• 1º Dígito: Sequência (1..9)\n• 2º Dígito: Função (1-2: NF, 3-4: NA, 5-6: NF esp., 7-8: NA esp.)")
                                            .size(10.0)
                                            .color(t.text2),
                                        row![
                                            text("Presets:").size(10.0).color(t.text2),
                                            button(text("13-14 (NA)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("aux_no")),
                                            button(text("21-22 (NF)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("aux_nc")),
                                            button(text("95-98 (Térmico)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyIecNumberingPreset("thermal")),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center),
                                        seq_pick,
                                    ]
                                    .spacing(4);

                                    if let Some(cur_seq) = current_seq {
                                        if occupied.contains(&cur_seq) {
                                            cmd_box = cmd_box.push(
                                                column![
                                                    row![
                                                        crate::icons::view_icon(
                                                            crate::icons::Icon::Warning,
                                                            12,
                                                            t.amber
                                                        ),
                                                        text(format!(
                                                            "Sequência {} já em uso por outro contato de '{}'",
                                                            cur_seq, item.tag
                                                        ))
                                                        .size(10.5)
                                                        .color(t.amber),
                                                    ]
                                                    .spacing(4)
                                                    .align_y(Alignment::Center),
                                                    button(
                                                        row![
                                                            crate::icons::view_icon(
                                                                crate::icons::Icon::Bolt,
                                                                12,
                                                                t.primary
                                                            ),
                                                            text(format!(
                                                                "Usar Sequência {} recomendada ({})",
                                                                suggested_digit,
                                                                kind.generate_pins(suggested_digit).join("-")
                                                            ))
                                                            .size(10.5)
                                                            .font(UI),
                                                        ]
                                                        .spacing(4)
                                                        .align_y(Alignment::Center)
                                                    )
                                                    .style(theme::tool_active)
                                                    .padding(Padding::from([2.0, 6.0]))
                                                    .on_press(Message::SelectPinSequence(suggested_digit)),
                                                ]
                                                .spacing(4),
                                            );
                                        } else {
                                            cmd_box = cmd_box.push(
                                                row![
                                                    crate::icons::view_icon(
                                                        crate::icons::Icon::Check,
                                                        12,
                                                        t.success
                                                    ),
                                                    text(format!(
                                                        "Sequência {} conforme IEC 60947",
                                                        cur_seq
                                                    ))
                                                    .size(10.5)
                                                    .color(t.success),
                                                ]
                                                .spacing(4)
                                                .align_y(Alignment::Center),
                                            );
                                        }
                                    } else {
                                        cmd_box = cmd_box.push(
                                            text("Bornes com numeração personalizada.")
                                                .size(10.0)
                                                .color(t.text2),
                                        );
                                    }

                                    body_col = body_col.push(
                                        container(cmd_box)
                                            .style(theme::card)
                                            .padding(Padding::from([5.0, 7.0]))
                                            .width(Length::Fill),
                                    );
                                }
                            } else if let Some(id_or_slug) = item.symbol_id_or_slug() {
                                let canonical_terms = crate::canonical::global_dictionary()
                                    .and_then(|dict| dict.get(id_or_slug))
                                    .map(|el| el.terminals.clone())
                                    .unwrap_or_default();

                                let pin_count = canonical_terms.len().min(9);
                                for p_idx in 0..pin_count {
                                    let default_label = canonical_terms
                                        .get(p_idx)
                                        .map(|t| t.name.as_str())
                                        .unwrap_or("");
                                    let current_pin_val =
                                        item.pins.get(p_idx).cloned().unwrap_or_default();
                                    body_col = body_col.push(
                                        row![
                                            text(format!("Borne {}:", p_idx + 1))
                                                .size(11)
                                                .font(MONO)
                                                .width(Length::Fixed(65.0)),
                                            text_input(default_label, &current_pin_val)
                                                .on_input(move |v| Message::EditPin(p_idx, v))
                                                .padding(Padding::from([3.0, 6.0]))
                                                .size(11.5)
                                                .style(theme::field),
                                        ]
                                        .spacing(6)
                                        .align_y(Alignment::Center),
                                    );
                                }
                            }
                        }

                        // 6. Réguas de Bornes em Painéis (IEC 81346 - Letra X)
                        if is_terminal_item(item) {
                            let strip_kind = crate::xref::TerminalStripKind::from_tag(&item.tag);
                            let strip_tag_for_btn = item.tag.trim().to_string();
                            body_col = body_col.push(horizontal_rule(1));
                            body_col = body_col.push(
                                container(
                                    column![
                                        row![
                                            text("Régua de Bornes em Painéis (IEC 81346):").size(11.5).font(
                                                Font {
                                                    weight: iced::font::Weight::Bold,
                                                    ..UI
                                                }
                                            ),
                                            horizontal_space(),
                                            container(
                                                text(strip_kind.badge_label())
                                                    .size(10.0)
                                                    .color(t.primary)
                                            )
                                            .style(theme::card)
                                            .padding(Padding::from([2.0, 6.0])),
                                        ]
                                        .align_y(Alignment::Center),
                                        text("Tabela de Subdivisão Funcional:\n• XP (ou X0): Força (Alimentação geral e motores)\n• XC: Comando (Botoeiras, sensores e lógicas)\n• XT: Medição (Transformadores e instrumentos)")
                                            .size(10.0)
                                            .color(t.text2),
                                        row![
                                            text("Classificar:").size(10.0).color(t.text2),
                                            button(text("XP (Força)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyTerminalStripTagPreset("-XP")),
                                            button(text("XC (Comando)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyTerminalStripTagPreset("-XC")),
                                            button(text("XT (Medição)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyTerminalStripTagPreset("-XT")),
                                            button(text("X1 (Geral)").size(9.5).font(UI))
                                                .style(theme::tool)
                                                .padding(Padding::from([2.0, 5.0]))
                                                .on_press(Message::ApplyTerminalStripTagPreset("-X1")),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center),
                                        button(
                                            row![
                                                crate::icons::view_icon(
                                                    crate::icons::Icon::Bolt,
                                                    11,
                                                    t.primary
                                                ),
                                                text(format!("Renumerar régua '{}'", item.tag.trim()))
                                                    .size(10.5)
                                                    .font(UI),
                                            ]
                                            .spacing(4)
                                            .align_y(Alignment::Center)
                                        )
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 6.0]))
                                        .on_press(Message::RenumberTerminalStrip(strip_tag_for_btn)),
                                    ]
                                    .spacing(4),
                                )
                                .style(theme::card)
                                .padding(Padding::from([6.0, 8.0]))
                                .width(Length::Fill),
                            );
                        }
                    }
                }
            } else {
                // Sem elemento selecionado -> Propriedades da Folha (Cajetín)
                body_col = body_col.push(text("Legenda da Folha (Cajetín)").size(13).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }));
                body_col = body_col.push(
                    column![
                        text("Título do Projeto:").size(11).color(t.text2),
                        text_input("Título", &app.doc.title_block.title)
                            .on_input(Message::EditTitleBlockTitle)
                            .padding(Padding::from([5.0, 8.0]))
                            .size(12)
                            .style(theme::field),
                    ]
                    .spacing(3),
                );
                body_col = body_col.push(
                    column![
                        text("Subtítulo:").size(11).color(t.text2),
                        text_input("Subtítulo", &app.doc.title_block.subtitle)
                            .on_input(Message::EditTitleBlockSubtitle)
                            .padding(Padding::from([5.0, 8.0]))
                            .size(12)
                            .style(theme::field),
                    ]
                    .spacing(3),
                );
                body_col = body_col.push(
                    column![
                        text("Desenhista / Autor:").size(11).color(t.text2),
                        text_input("Autor", &app.doc.title_block.drawn_by)
                            .on_input(Message::EditTitleBlockAuthor)
                            .padding(Padding::from([5.0, 8.0]))
                            .size(12)
                            .style(theme::field),
                    ]
                    .spacing(3),
                );
                body_col = body_col.push(horizontal_rule(1));
                body_col = body_col.push(
                        text(
                            "Dica:\n- Clique em qualquer símbolo na folha para editar suas propriedades.\n- Arraste com botão direito ou scroll para navegar.",
                        )
                        .size(11.5)
                        .color(t.text2),
                    );
            }

            container(scrollable(body_col)).height(Length::Fill).into()
        }
        1 => {
            // Aba Referência Cruzada (XRef) Inter-Folhas e Ligações
            let mut xref_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));

            if let Some(sel_id) = app.selected_id {
                // Elemento Selecionado -> Análise contextual inter-folhas e equipotenciais
                if let Some(ctx) = crate::xref::analyze_element_xref(
                    sel_id,
                    &app.pages,
                    app.active_page_index,
                    &app.doc,
                ) {
                    let sel = &ctx.selected_item;
                    xref_col = xref_col.push(
                        container(
                            column![
                                row![
                                    crate::icons::view_icon(
                                        crate::icons::Icon::Search,
                                        13,
                                        t.primary
                                    ),
                                    text(format!("#{sel_id} {} ({})", sel.tag, sel.symbol_name))
                                        .size(12.5)
                                        .font(Font {
                                            weight: iced::font::Weight::Bold,
                                            ..UI
                                        })
                                        .color(t.primary),
                                    horizontal_space(),
                                    button(text("XRef Global").size(10.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 6.0]))
                                        .on_press(Message::DeselectAll),
                                ]
                                .align_y(Alignment::Center),
                                row![
                                    text(format!(
                                        "Localização: Folha {} · {}",
                                        sel.page_index + 1,
                                        sel.grid_ref
                                    ))
                                    .size(10.5)
                                    .color(t.text2),
                                    horizontal_space(),
                                    text(sel.role.display_name())
                                        .size(10.5)
                                        .font(Font {
                                            weight: iced::font::Weight::Medium,
                                            ..UI
                                        })
                                        .color(t.text),
                                ]
                                .align_y(Alignment::Center),
                            ]
                            .spacing(4),
                        )
                        .style(theme::card)
                        .padding(Padding::from([8.0, 10.0]))
                        .width(Length::Fill),
                    );

                    // 1. Aparelho e Contatos Vinculados (Inter-Folhas)
                    let mut assoc_col =
                        column![text("Vínculo de Aparelho & Contatos (Inter-Folhas)")
                            .size(11.5)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            }),]
                        .spacing(4);

                    if let Some(ref parent) = ctx.parent_apparatus {
                        let p_page = parent.page_index;
                        let p_id = parent.id;
                        assoc_col = assoc_col.push(
                            row![
                                crate::icons::view_icon(crate::icons::Icon::Check, 12, t.success),
                                text(format!(
                                    "Aparelho Pai: #{} {} na Folha {} ({})",
                                    parent.id,
                                    parent.tag,
                                    p_page + 1,
                                    parent.grid_ref
                                ))
                                .size(10.5)
                                .color(t.success),
                                horizontal_space(),
                                button(text("Ir").size(10.0).font(UI))
                                    .style(theme::tool)
                                    .padding(Padding::from([2.0, 6.0]))
                                    .on_press(Message::NavigateToElement {
                                        page_index: p_page,
                                        item_id: p_id,
                                    }),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center),
                        );
                    } else if ctx.selected_item.role.is_subordinate_contact() {
                        assoc_col = assoc_col.push(
                            row![
                                crate::icons::view_icon(crate::icons::Icon::Warning, 12, t.amber),
                                text("Contato órfão: Nenhuma bobina vinculada no projeto.")
                                    .size(10.5)
                                    .color(t.amber),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center),
                        );
                    }

                    if !ctx.sibling_contacts.is_empty() {
                        assoc_col = assoc_col.push(
                            text(format!(
                                "Contatos associados no projeto ({}):",
                                ctx.sibling_contacts.len()
                            ))
                            .size(10.5)
                            .color(t.text2),
                        );
                        for sibling in &ctx.sibling_contacts {
                            let sib_page = sibling.page_index;
                            let sib_id = sibling.id;
                            let p_str = if sibling.pins.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", sibling.pins.join("-"))
                            };
                            assoc_col = assoc_col.push(
                                row![
                                    text(format!(
                                        "• #{} {}{}: Folha {} ({})",
                                        sibling.id,
                                        sibling.symbol_name,
                                        p_str,
                                        sib_page + 1,
                                        sibling.grid_ref
                                    ))
                                    .size(10.5)
                                    .font(MONO),
                                    horizontal_space(),
                                    button(text("Ir").size(10.0).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([2.0, 6.0]))
                                        .on_press(Message::NavigateToElement {
                                            page_index: sib_page,
                                            item_id: sib_id,
                                        }),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );
                        }
                    } else if !ctx.selected_item.role.is_subordinate_contact() {
                        assoc_col = assoc_col.push(
                            text(
                                "Nenhum contato auxiliar subordinado associado a esta tag no projeto.",
                            )
                            .size(10.5)
                            .color(t.text2),
                        );
                    }

                    xref_col = xref_col.push(
                        container(assoc_col)
                            .style(theme::card)
                            .padding(Padding::from([8.0, 10.0]))
                            .width(Length::Fill),
                    );

                    // 2. Ligações Elétricas dos Bornes (Série e Paralelo)
                    let mut conn_col =
                        column![text("Ligações Elétricas dos Bornes (Série / Paralelo)")
                            .size(11.5)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            }),]
                        .spacing(4);

                    if ctx.pin_connections.is_empty() {
                        conn_col = conn_col.push(
                            text("Componente sem bornes elétricos configurados.")
                                .size(10.5)
                                .color(t.text2),
                        );
                    } else {
                        for pin_sum in &ctx.pin_connections {
                            let mut pin_box = column![row![
                                text(format!("Borne {}:", pin_sum.pin_label))
                                    .size(11.0)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..MONO
                                    }),
                                horizontal_space(),
                                if !pin_sum.is_connected {
                                    text("[Aberto / Não Conectado]").size(10.0).color(t.text2)
                                } else {
                                    text(format!("{} conexão(ões)", pin_sum.targets.len()))
                                        .size(10.0)
                                        .color(t.text2)
                                },
                            ]
                            .align_y(Alignment::Center),]
                            .spacing(2);

                            for tgt in &pin_sum.targets {
                                let t_page = app.active_page_index;
                                let t_id = tgt.item_id;
                                let badge_style = match tgt.conn_type {
                                    crate::xref::ElectricalConnectionType::Parallel => t.primary,
                                    crate::xref::ElectricalConnectionType::Series => t.success,
                                    crate::xref::ElectricalConnectionType::Direct => t.text2,
                                };
                                pin_box = pin_box.push(
                                    row![
                                        container(
                                            text(tgt.conn_type.badge_label())
                                                .size(9.5)
                                                .color(badge_style)
                                        )
                                        .style(theme::card)
                                        .padding(Padding::from([1.0, 4.0])),
                                        text(format!(
                                            "{} : {} ({}) · {}",
                                            tgt.tag, tgt.pin_label, tgt.symbol_name, tgt.grid_ref
                                        ))
                                        .size(10.5)
                                        .font(MONO),
                                        horizontal_space(),
                                        button(text("Ir").size(9.5).font(UI))
                                            .style(theme::tool)
                                            .padding(Padding::from([1.0, 5.0]))
                                            .on_press(Message::NavigateToElement {
                                                page_index: t_page,
                                                item_id: t_id,
                                            }),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
                                );
                            }

                            conn_col = conn_col.push(
                                container(pin_box)
                                    .style(theme::card)
                                    .padding(Padding::from([4.0, 6.0]))
                                    .width(Length::Fill),
                            );
                        }
                    }

                    xref_col = xref_col.push(
                        container(conn_col)
                            .style(theme::card)
                            .padding(Padding::from([8.0, 10.0]))
                            .width(Length::Fill),
                    );

                    // 3. Informações de Régua de Bornes (se for borne)
                    if let Some(ref strip) = ctx.terminal_strip {
                        xref_col = xref_col.push(
                            container(
                                column![
                                    row![
                                        text(format!("Régua de Bornes: {}", strip.strip_tag))
                                            .size(11.5)
                                            .font(Font {
                                                weight: iced::font::Weight::Bold,
                                                ..UI
                                            }),
                                        horizontal_space(),
                                        container(
                                            text(strip.kind.badge_label())
                                                .size(10.0)
                                                .color(t.primary)
                                        )
                                        .style(theme::card)
                                        .padding(Padding::from([2.0, 6.0])),
                                    ]
                                    .align_y(Alignment::Center),
                                    text(format!(
                                        "Total: {} bornes nesta régua em todo o projeto.",
                                        strip.terminals.len()
                                    ))
                                    .size(10.5)
                                    .color(t.text2),
                                    button(
                                        row![
                                            crate::icons::view_icon(
                                                crate::icons::Icon::Bolt,
                                                11,
                                                t.primary
                                            ),
                                            text("Renumerar Esta Régua Sequencialmente")
                                                .size(10.5)
                                                .font(UI),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center)
                                    )
                                    .style(theme::tool)
                                    .padding(Padding::from([3.0, 8.0]))
                                    .on_press(
                                        Message::RenumberTerminalStrip(strip.strip_tag.clone())
                                    ),
                                ]
                                .spacing(4),
                            )
                            .style(theme::card)
                            .padding(Padding::from([8.0, 10.0]))
                            .width(Length::Fill),
                        );
                    }
                }
            } else {
                // Nenhum elemento selecionado -> Diretório Global de Referências Cruzadas
                let directory =
                    crate::xref::collect_project_xrefs(&app.pages, app.active_page_index, &app.doc);

                xref_col = xref_col.push(
                    row![
                        text("Diretório Global de Referências").size(13).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        }),
                        horizontal_space(),
                        text(format!(
                            "{} tags · {} folhas",
                            directory.total_tags, directory.total_pages
                        ))
                        .size(10.5)
                        .color(t.text2),
                    ]
                    .align_y(Alignment::Center),
                );

                // Barra de busca
                xref_col = xref_col.push(
                    text_input(
                        "Filtrar por etiqueta (ex: KM1, Q1, S1)...",
                        &app.xref_search,
                    )
                    .on_input(Message::XrefSearchChanged)
                    .padding(Padding::from([4.0, 8.0]))
                    .size(11.5)
                    .style(theme::field),
                );

                let filter = app.xref_search.trim().to_uppercase();
                let filtered_groups: Vec<_> = directory
                    .groups
                    .iter()
                    .filter(|g| {
                        if filter.is_empty() {
                            true
                        } else {
                            g.tag.to_uppercase().contains(&filter)
                        }
                    })
                    .collect();

                if filtered_groups.is_empty() {
                    xref_col = xref_col.push(
                        text("Nenhum aparelho encontrado com a etiqueta informada.")
                            .size(11.5)
                            .color(t.text2),
                    );
                } else {
                    for group in filtered_groups {
                        let mut card_col = column![row![
                            text(format!("Etiqueta: {}", group.tag))
                                .size(12.0)
                                .font(Font {
                                    weight: iced::font::Weight::Bold,
                                    ..MONO
                                })
                                .color(t.primary),
                            horizontal_space(),
                            if group.is_orphan_contact {
                                container(text("Sem Bobina").size(9.5).color(t.amber))
                                    .style(theme::card)
                                    .padding(Padding::from([1.0, 4.0]))
                            } else if group.has_duplicate_primary {
                                container(text("Bobina Duplicada").size(9.5).color(t.danger))
                                    .style(theme::card)
                                    .padding(Padding::from([1.0, 4.0]))
                            } else {
                                container(Space::with_width(0))
                            },
                        ]
                        .align_y(Alignment::Center),]
                        .spacing(4);

                        if let Some(ref primary) = group.primary {
                            let p_page = primary.page_index;
                            let p_id = primary.id;
                            let p_str = if primary.pins.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", primary.pins.join("-"))
                            };
                            card_col = card_col.push(
                                row![
                                    text(format!(
                                        "• Aparelho: #{} {}{} · Folha {} ({})",
                                        primary.id,
                                        primary.symbol_name,
                                        p_str,
                                        p_page + 1,
                                        primary.grid_ref
                                    ))
                                    .size(10.5)
                                    .font(MONO),
                                    horizontal_space(),
                                    button(text("Ir").size(9.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 5.0]))
                                        .on_press(Message::NavigateToElement {
                                            page_index: p_page,
                                            item_id: p_id,
                                        }),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );
                        }

                        for contact in &group.contacts {
                            let c_page = contact.page_index;
                            let c_id = contact.id;
                            let c_str = if contact.pins.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", contact.pins.join("-"))
                            };
                            card_col = card_col.push(
                                row![
                                    text(format!(
                                        "  - Contato: #{} {}{} · Folha {} ({})",
                                        contact.id,
                                        contact.symbol_name,
                                        c_str,
                                        c_page + 1,
                                        contact.grid_ref
                                    ))
                                    .size(10.5)
                                    .font(MONO),
                                    horizontal_space(),
                                    button(text("Ir").size(9.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 5.0]))
                                        .on_press(Message::NavigateToElement {
                                            page_index: c_page,
                                            item_id: c_id,
                                        }),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );
                        }

                        for other in &group.other_instances {
                            let o_page = other.page_index;
                            let o_id = other.id;
                            let o_str = if other.pins.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", other.pins.join("-"))
                            };
                            card_col = card_col.push(
                                row![
                                    text(format!(
                                        "• Instância: #{} {}{} · Folha {} ({})",
                                        other.id,
                                        other.symbol_name,
                                        o_str,
                                        o_page + 1,
                                        other.grid_ref
                                    ))
                                    .size(10.5)
                                    .font(MONO),
                                    horizontal_space(),
                                    button(text("Ir").size(9.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 5.0]))
                                        .on_press(Message::NavigateToElement {
                                            page_index: o_page,
                                            item_id: o_id,
                                        }),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );
                        }

                        xref_col = xref_col.push(
                            container(card_col)
                                .style(theme::card)
                                .padding(Padding::from([6.0, 8.0]))
                                .width(Length::Fill),
                        );
                    }

                    if !directory.terminal_strips.is_empty() {
                        let mut strips_box = column![row![
                            text("Réguas de Bornes do Projeto (IEC 81346)")
                                .size(11.5)
                                .font(Font {
                                    weight: iced::font::Weight::Bold,
                                    ..UI
                                }),
                            horizontal_space(),
                            button(
                                row![
                                    crate::icons::view_icon(
                                        crate::icons::Icon::Bolt,
                                        11,
                                        t.primary
                                    ),
                                    text("Renumerar Todas").size(10.0).font(UI),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            )
                            .style(theme::tool)
                            .padding(Padding::from([2.0, 6.0]))
                            .on_press(Message::RenumberAllTerminalStrips),
                        ]
                        .align_y(Alignment::Center),]
                        .spacing(4);

                        for strip in &directory.terminal_strips {
                            let s_tag = strip.strip_tag.clone();
                            strips_box = strips_box.push(
                                row![
                                    text(format!("• Régua {}", strip.strip_tag))
                                        .size(10.5)
                                        .font(MONO)
                                        .color(t.primary),
                                    container(
                                        text(strip.kind.badge_label()).size(9.0).color(t.primary)
                                    )
                                    .style(theme::card)
                                    .padding(Padding::from([1.0, 4.0])),
                                    text(format!("({} bornes)", strip.terminals.len()))
                                        .size(10.0)
                                        .color(t.text2),
                                    horizontal_space(),
                                    button(text("Renumerar").size(9.5).font(UI))
                                        .style(theme::tool)
                                        .padding(Padding::from([1.0, 5.0]))
                                        .on_press(Message::RenumberTerminalStrip(s_tag)),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            );
                        }

                        xref_col = xref_col.push(
                            container(strips_box)
                                .style(theme::card)
                                .padding(Padding::from([6.0, 8.0]))
                                .width(Length::Fill),
                        );
                    }
                }
            }

            container(scrollable(xref_col)).height(Length::Fill).into()
        }
        2 => {
            // Aba Réguas de Bornes (IEC 81346)
            let mut bornes_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));
            let directory =
                crate::xref::collect_project_xrefs(&app.pages, app.active_page_index, &app.doc);

            bornes_col = bornes_col.push(
                column![
                    text("Réguas de Bornes (IEC 81346)").size(13).font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }),
                    text("Subdivisão: XP (Potência), XC (Comando), XT (Medição)")
                        .size(10.5)
                        .color(t.text2),
                ]
                .spacing(2),
            );

            bornes_col = bornes_col.push(
                row![
                    text(format!(
                        "{} régua(s) · {} borne(s)",
                        directory.terminal_strips.len(),
                        directory.total_terminals
                    ))
                    .size(11.0)
                    .color(t.text2),
                    horizontal_space(),
                    if !directory.terminal_strips.is_empty() {
                        button(
                            row![
                                crate::icons::view_icon(crate::icons::Icon::Bolt, 11, t.primary),
                                text("Renumerar Todas (1..N)").size(10.5).font(UI),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center),
                        )
                        .style(theme::tool)
                        .padding(Padding::from([3.0, 8.0]))
                        .on_press(Message::RenumberAllTerminalStrips)
                    } else {
                        button(text("Sem réguas").size(10.5).font(UI))
                            .style(theme::tool)
                            .padding(Padding::from([3.0, 8.0]))
                    },
                ]
                .align_y(Alignment::Center),
            );

            if directory.terminal_strips.is_empty() {
                bornes_col = bornes_col.push(
                    container(
                        column![
                            row![
                                crate::icons::view_icon(crate::icons::Icon::Info, 13, t.primary),
                                text("Nenhuma régua de bornes identificada no projeto.")
                                    .size(11.5)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    })
                                    .color(t.primary),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center),
                            text(
                                "Para criar uma régua de bornes:\n- Insira símbolos de borne no esquema ou painel.\n- Atribua a etiqueta da régua (ex: -XP para força, -XC para comando, -XT para medição, ou -X1).\n- Utilize a numeração sequencial automática.",
                            )
                            .size(10.5)
                            .color(t.text2),
                        ]
                        .spacing(6),
                    )
                    .style(theme::card)
                    .padding(Padding::from([10.0, 12.0]))
                    .width(Length::Fill),
                );
            } else {
                for strip in &directory.terminal_strips {
                    let s_tag = strip.strip_tag.clone();
                    let mut strip_card = column![
                        row![
                            text(format!("Régua {}", strip.strip_tag))
                                .size(12.5)
                                .font(Font {
                                    weight: iced::font::Weight::Bold,
                                    ..MONO
                                })
                                .color(t.primary),
                            container(text(strip.kind.badge_label()).size(9.5).color(t.primary))
                                .style(theme::card)
                                .padding(Padding::from([1.0, 5.0])),
                            horizontal_space(),
                            button(
                                row![
                                    crate::icons::view_icon(
                                        crate::icons::Icon::Bolt,
                                        10,
                                        t.primary
                                    ),
                                    text("Renumerar").size(10.0).font(UI),
                                ]
                                .spacing(3)
                                .align_y(Alignment::Center)
                            )
                            .style(theme::tool)
                            .padding(Padding::from([2.0, 6.0]))
                            .on_press(Message::RenumberTerminalStrip(s_tag)),
                        ]
                        .align_y(Alignment::Center),
                        row![
                            text(strip.kind.display_name()).size(10.0).color(t.text2),
                            horizontal_space(),
                            text(format!("{} bornes", strip.terminals.len()))
                                .size(10.0)
                                .color(t.text2),
                        ]
                        .align_y(Alignment::Center),
                    ]
                    .spacing(4);

                    if strip.has_duplicates {
                        strip_card = strip_card.push(
                            row![
                                crate::icons::view_icon(crate::icons::Icon::Warning, 11, t.amber),
                                text("Aviso: Existem bornes com numeração repetida nesta régua.")
                                    .size(10.0)
                                    .color(t.amber),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center),
                        );
                    }

                    // Tabela de bornes da régua
                    for term in &strip.terminals {
                        let t_page = term.page_index;
                        let t_id = term.item_id;
                        let dup_badge = if term.is_duplicate {
                            container(text("Repetido").size(9.0).color(t.amber))
                                .style(theme::card)
                                .padding(Padding::from([1.0, 3.0]))
                        } else {
                            container(Space::with_width(0))
                        };

                        let conn_info = if term.connected_targets.is_empty() {
                            String::new()
                        } else {
                            format!(" -> {}", term.connected_targets.join(", "))
                        };

                        strip_card = strip_card.push(
                            row![
                                text(format!("• Borne {}", term.terminal_number))
                                    .size(10.5)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..MONO
                                    }),
                                dup_badge,
                                text(format!(
                                    "Folha {} ({}){}",
                                    term.page_index + 1,
                                    term.grid_ref,
                                    conn_info
                                ))
                                .size(10.0)
                                .font(MONO)
                                .color(t.text2),
                                horizontal_space(),
                                button(text("Ir").size(9.5).font(UI))
                                    .style(theme::tool)
                                    .padding(Padding::from([1.0, 5.0]))
                                    .on_press(Message::NavigateToElement {
                                        page_index: t_page,
                                        item_id: t_id,
                                    }),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center),
                        );
                    }

                    bornes_col = bornes_col.push(
                        container(strip_card)
                            .style(theme::card)
                            .padding(Padding::from([6.0, 8.0]))
                            .width(Length::Fill),
                    );
                }
            }

            container(scrollable(bornes_col))
                .height(Length::Fill)
                .into()
        }
        _ => {
            // Aba Geral
            let mut geral_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));

            geral_col =
                geral_col.push(text("Geometria & Propriedades Gerais").size(13).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }));

            if let Some(sel_id) = app.selected_id {
                if let Some(item) = app.doc.items.iter().find(|i| i.id == sel_id) {
                    geral_col = geral_col.push(
                        container(
                            column![
                                row![
                                    text("ID interno:")
                                        .size(11)
                                        .color(t.text2)
                                        .width(Length::Fixed(90.0)),
                                    text(format!("#{sel_id}")).size(11.5).font(MONO),
                                ],
                                row![
                                    text("Tipo CADe:")
                                        .size(11)
                                        .color(t.text2)
                                        .width(Length::Fixed(90.0)),
                                    text(format!("{}", item.tipo)).size(11.5).font(MONO),
                                ],
                                row![
                                    text("Posição (X, Y):")
                                        .size(11)
                                        .color(t.text2)
                                        .width(Length::Fixed(90.0)),
                                    text(format!("X: {}, Y: {}", item.x1, item.y1))
                                        .size(11.5)
                                        .font(MONO),
                                ],
                                row![
                                    text("Orientação:")
                                        .size(11)
                                        .color(t.text2)
                                        .width(Length::Fixed(90.0)),
                                    text(format!("{}°", item.orientation * 90))
                                        .size(11.5)
                                        .font(MONO),
                                ],
                            ]
                            .spacing(4),
                        )
                        .style(theme::card)
                        .padding(Padding::from([8.0, 10.0])),
                    );

                    geral_col = geral_col.push(
                        button(
                            row![
                                crate::icons::view_icon(
                                    crate::icons::Icon::Delete,
                                    14,
                                    t.on_danger
                                ),
                                text("Excluir Elemento").size(12).font(UI),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                        )
                        .style(theme::pill_stop(true))
                        .padding(Padding::from([5.0, 10.0]))
                        .width(Length::Fill)
                        .on_press(Message::DeleteSelected),
                    );
                }
            } else {
                geral_col = geral_col.push(
                    container(
                        column![
                            row![
                                text("Dimensões:")
                                    .size(11)
                                    .color(t.text2)
                                    .width(Length::Fixed(110.0)),
                                text("DIN A4 (1040 × 720 px)").size(11.5).font(MONO),
                            ],
                            row![
                                text("Elementos:")
                                    .size(11)
                                    .color(t.text2)
                                    .width(Length::Fixed(110.0)),
                                text(format!("{} itens", app.doc.items.len()))
                                    .size(11.5)
                                    .font(MONO),
                            ],
                            row![
                                text("Snap de Grade:")
                                    .size(11)
                                    .color(t.text2)
                                    .width(Length::Fixed(110.0)),
                                text(if app.show_grid {
                                    "Ativo (5px)"
                                } else {
                                    "Desligado"
                                })
                                .size(11.5)
                                .font(MONO),
                            ],
                        ]
                        .spacing(4),
                    )
                    .style(theme::card)
                    .padding(Padding::from([8.0, 10.0])),
                );
            }

            container(scrollable(geral_col)).height(Length::Fill).into()
        }
    };

    // Rodapé do dock com indicador LED do simulador
    let sim_status_text = if app.sim_running {
        "Simulador ativo (80ms)"
    } else {
        "Simulador parado"
    };
    let led_color = if app.sim_running {
        t.primary
    } else {
        t.border_strong
    };
    let dock_dot = container(Space::new(7, 7)).style(theme::status_dot(led_color));
    let dock_foot = container(
        row![
            dock_dot,
            text(sim_status_text).size(11).font(MONO).color(t.text2),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .style(theme::bar)
    .padding(Padding::from([6.0, 12.0]))
    .width(Length::Fill);

    column![header, tabs_bar, panel_body, dock_foot]
        .height(Length::Fill)
        .into()
}
