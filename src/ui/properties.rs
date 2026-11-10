use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, pick_list, row,
    scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, Message, PinSequenceOption};
use crate::symbols::{
    classify_apparatus_role, detect_contact_kind, detect_current_sequence, extract_tag_prefix,
    find_duplicate_apparatus, find_occupied_sequences_for_tag, find_parent_apparatus, find_symbol,
    list_available_parent_tags, next_sequential_tag_for_role,
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
        row![
            tab_btn(0, "Propriedades"),
            tab_btn(1, "Ref. Cruzada"),
            tab_btn(2, "Réguas de Bornes"),
        ]
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
                    } else if item.is_wire() {
                        "Cabo Condutor".to_string()
                    } else {
                        "Elemento".to_string()
                    };

                    let (sheet_w, sheet_h) = crate::xref::sheet_dimensions(&app.doc);
                    let grid_ref = crate::xref::format_grid_ref(
                        app.active_page_index,
                        item.x1 as f32,
                        item.y1 as f32,
                        sheet_w,
                        sheet_h,
                    );

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

                    if !item.is_wire() && !item.is_junction() {
                        // Tag / Etiqueta
                        body_col = body_col.push(
                            column![
                                text("Etiqueta / Tag (ex: -KM1, -Q1):")
                                    .size(11)
                                    .color(t.text2),
                                text_input("Tag", &item.tag)
                                    .on_input(Message::EditTag)
                                    .padding(Padding::from([5.0, 8.0]))
                                    .size(12)
                                    .style(theme::field),
                                checkbox("Exibir Tag no esquema", item.show_tag == 1)
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
                                            .spacing(4)
                                        )
                                        .padding(Padding::from([4.0, 6.0]))
                                        .style(theme::card)
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

                        // Valor / Temporização
                        body_col = body_col.push(
                            column![
                                text("Valor / Temporização / Referência:")
                                    .size(11)
                                    .color(t.text2),
                                text_input("Valor", &item.value)
                                    .on_input(Message::EditValue)
                                    .padding(Padding::from([5.0, 8.0]))
                                    .size(12)
                                    .style(theme::field),
                                checkbox("Exibir Valor no esquema", item.show_value == 1)
                                    .on_toggle(Message::ToggleShowValue)
                                    .text_size(11.5),
                            ]
                            .spacing(4),
                        );

                        // Bornes / Terminais
                        if let Some(sym) = find_symbol(item.tipo) {
                            if !sym.pins.is_empty() {
                                body_col = body_col.push(horizontal_rule(1));
                                body_col = body_col.push(
                                    text("Numeração dos Bornes:").size(12).font(Font {
                                        weight: iced::font::Weight::Semibold,
                                        ..UI
                                    }),
                                );
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

                                // Dropdown sequencial inteligente IEC 60947 (1 a 9)
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

                                    body_col = body_col.push(
                                        column![
                                            text("Sequência de Bornes (IEC 60947):")
                                                .size(11)
                                                .color(t.text2)
                                                .font(Font {
                                                    weight: iced::font::Weight::Medium,
                                                    ..UI
                                                }),
                                            seq_pick,
                                        ]
                                        .spacing(4),
                                    );

                                    if let Some(cur_seq) = current_seq {
                                        if occupied.contains(&cur_seq) {
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
                                                        .spacing(4)
                                                    )
                                                    .padding(Padding::from([4.0, 6.0]))
                                                    .style(theme::card)
                                                );
                                        } else {
                                            body_col = body_col.push(
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
                                        body_col = body_col.push(
                                            text("Bornes com numeração personalizada.")
                                                .size(10.0)
                                                .color(t.text2),
                                        );
                                    }
                                }
                            }
                        } else if let Some(id_or_slug) = item.symbol_id_or_slug() {
                            let canonical_terms = crate::canonical::global_dictionary()
                                .and_then(|dict| dict.get(id_or_slug))
                                .map(|el| el.terminals.clone())
                                .unwrap_or_default();

                            let pin_count = canonical_terms.len().min(9);
                            if pin_count > 0 {
                                body_col = body_col.push(horizontal_rule(1));
                                body_col = body_col.push(
                                    text("Numeração dos Bornes:").size(12).font(Font {
                                        weight: iced::font::Weight::Semibold,
                                        ..UI
                                    }),
                                );
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

                        // Controles para Sinalizadores Luminosos / Lâmpadas
                        let is_lamp = item.is_lamp_or_signaling();

                        if is_lamp {
                            body_col = body_col.push(horizontal_rule(1));
                            body_col = body_col.push(
                                text("Cor e Pulso do Sinaleiro Luminoso:")
                                    .size(11.5)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    }),
                            );

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
                            ]
                            .spacing(3);

                            let pulse_row = row![
                                text("Pulso (s):").size(11).color(t.text2),
                                text_input("ex: 0.5", &item.value)
                                    .on_input(Message::SetPulseDuration)
                                    .padding(Padding::from([3.0, 6.0]))
                                    .size(11.5)
                                    .width(80)
                                    .style(theme::field),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center);

                            body_col = body_col.push(colors_row).push(pulse_row);
                        }

                        if crate::symbols::is_terminal_item(item) {
                            let strip_kind = crate::xref::TerminalStripKind::from_tag(&item.tag);
                            let strip_tag_for_btn = item.tag.trim().to_string();
                            body_col = body_col.push(horizontal_rule(1));
                            body_col = body_col.push(
                                column![
                                    row![
                                        text("Régua de Bornes (IEC 81346):").size(11.5).font(
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
                                    row![
                                        text("Tipo:").size(10.5).color(t.text2),
                                        text(strip_kind.display_name()).size(10.5).color(t.text),
                                    ]
                                    .spacing(4),
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
