use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, text, text_input, tooltip,
};
use iced::{Alignment, Element, Length, Padding};

use crate::app::{CadeSimuApp, Message};
use crate::canvas_view::SchematicCanvas;
use crate::theme::{self, tokens, MONO, UI};
use crate::ui::widgets::app_tooltip;

pub fn view_sheet_panel<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let title_line = view_canvas_title_line(app);
    let pages_bar = view_canvas_pages_bar(app);

    let is_dark = app.is_dark();

    let canvas_el = SchematicCanvas {
        doc: &app.doc,
        tool: app.tool.clone(),
        selected_id: app.selected_id,
        selected_ids: &app.selected_ids,
        is_shift_pressed: app.is_shift_pressed,
        is_ctrl_pressed: app.is_ctrl_pressed,
        sim_running: app.sim_running,
        show_grid: app.show_grid,
        grid_density: app.grid_density,
        show_texts: app.show_texts,
        is_dark,
        auto_connect: app.auto_connect,
        auto_disconnect: app.auto_disconnect,
        zoom: app.zoom,
        pan: app.pan,
        placement_orientation: app.placement_orientation,
        dragging_symbol: app.dragging_symbol,
        dragging_canonical_symbol: app.dragging_canonical_id.as_deref(),
        is_blocked: app.open_menu.is_some()
            || app.show_about_modal
            || app.show_export_modal
            || app.show_color_modal
            || app.show_confirm_delete_modal,
    }
    .view()
    .map(Message::Canvas);

    column![
        title_line,
        pages_bar,
        container(canvas_el)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::workspace),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn view_canvas_title_line<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let doc_name = if !app.file_path.is_empty() {
        std::path::Path::new(&app.file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| app.file_path.clone())
    } else if let Some(ref qp) = app.qet_project {
        let cur = app.active_page_index;
        qp.diagrams
            .get(cur)
            .map(|d| d.title.clone())
            .unwrap_or_else(|| "Esquema QET".to_string())
    } else {
        "Novo Esquema".to_string()
    };

    let title_badge = row![
        crate::icons::view_icon(crate::icons::Icon::Copy, 14, t.text2),
        text(doc_name).size(13).font(UI),
        if app.is_modified {
            container(text("Modificado").size(10).color(t.danger))
                .padding(Padding::from([2.0, 6.0]))
                .style(theme::card)
        } else {
            container(text("Salvo").size(10).color(t.primary)).padding(Padding::from([2.0, 6.0]))
        }
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let action_btn = |icon: crate::icons::Icon, label: &'static str, msg: Message| {
        button(
            row![
                crate::icons::view_icon(icon, 13, t.text),
                text(label).size(11).font(UI),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .style(theme::tool)
        .padding(Padding::from([3.0, 8.0]))
        .on_press(msg)
    };

    let actions = row![
        action_btn(crate::icons::Icon::New, "Novo", Message::NewDocument),
        action_btn(crate::icons::Icon::Open, "Abrir", Message::OpenFileDialog),
        action_btn(crate::icons::Icon::Close, "Fechar", Message::CloseDocument),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    container(
        row![title_badge, horizontal_space(), actions,]
            .width(Length::Fill)
            .align_y(Alignment::Center),
    )
    .style(theme::bar)
    .padding(Padding::from([4.0, 10.0]))
    .width(Length::Fill)
    .into()
}

pub fn view_canvas_pages_bar<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());
    let total_pages = app.pages.len();

    let mut tabs_row = row![].spacing(4).align_y(Alignment::Center);

    for (idx, page) in app.pages.iter().enumerate() {
        let is_active = idx == app.active_page_index;
        let is_editing = is_active && app.editing_page_name.is_some();

        let tab_content: Element<'_, Message> = if is_editing {
            let current_input = app.editing_page_name.as_deref().unwrap_or(&page.name);
            row![
                text_input("Nome da folha", current_input)
                    .on_input(Message::UpdateEditingPageName)
                    .on_submit(Message::CommitEditingPageName)
                    .padding(Padding::from([2.0, 6.0]))
                    .width(130),
                button(text("OK").size(11).font(UI))
                    .style(theme::primary)
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::CommitEditingPageName),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
        } else {
            let tab_label = format!("{}. {}", idx + 1, page.name);
            let main_btn = button(text(tab_label).size(12).font(UI))
                .style(theme::tab(is_active))
                .padding(Padding::from([4.0, 8.0]))
                .on_press(Message::SelectPage(idx));

            let edit_btn = button(
                container(crate::icons::view_icon(
                    crate::icons::Icon::Pencil,
                    11,
                    t.text2,
                ))
                .width(14)
                .height(14)
                .center_x(14)
                .center_y(14),
            )
            .style(theme::tool)
            .padding(Padding::from([2.0, 3.0]))
            .on_press(Message::StartEditingPageName(idx));

            let menu_btn = button(text("...").size(10).font(MONO))
                .style(theme::tool)
                .padding(Padding::from([2.0, 4.0]))
                .on_press(Message::TogglePageContextMenu(idx));

            if is_active {
                row![main_btn, edit_btn, menu_btn]
                    .spacing(1)
                    .align_y(Alignment::Center)
                    .into()
            } else {
                main_btn.into()
            }
        };

        let tab_box = container(tab_content)
            .padding(Padding::from([1.0, 2.0]))
            .style(theme::page_tab_box(is_active));

        tabs_row = tabs_row.push(tab_box);
    }

    // Atalhos de ponta: [<-+] e [+->]
    let add_before_btn = app_tooltip(
        button(
            container(crate::icons::view_icon(
                crate::icons::Icon::AddBefore,
                14,
                t.text,
            ))
            .width(18)
            .height(18)
            .center_x(18)
            .center_y(18),
        )
        .style(theme::tool)
        .padding(Padding::from([3.0, 5.0]))
        .on_press(Message::AddPageBefore(app.active_page_index)),
        "Inserir página antes da atual",
        tooltip::Position::Bottom,
    );

    let add_after_btn = app_tooltip(
        button(
            container(crate::icons::view_icon(
                crate::icons::Icon::AddAfter,
                14,
                t.text,
            ))
            .width(18)
            .height(18)
            .center_x(18)
            .center_y(18),
        )
        .style(theme::tool)
        .padding(Padding::from([3.0, 5.0]))
        .on_press(Message::AddPageAfter(app.active_page_index)),
        "Inserir página depois da atual",
        tooltip::Position::Bottom,
    );

    let nav_controls = row![add_before_btn, add_after_btn]
        .spacing(3)
        .align_y(Alignment::Center);

    let context_banner: Option<Element<'_, Message>> =
        if let Some(target_idx) = app.page_context_menu {
            if target_idx < total_pages {
                let p_name = &app.pages[target_idx].name;
                let move_left_btn = button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::ChevronLeft, 12, t.text),
                        text("Esquerda").size(11).font(UI),
                    ]
                    .spacing(3)
                    .align_y(Alignment::Center),
                )
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(if target_idx > 0 {
                    Message::MovePage(target_idx, target_idx - 1)
                } else {
                    Message::MovePage(target_idx, target_idx)
                });

                let move_right_btn = button(
                    row![
                        text("Direita").size(11).font(UI),
                        crate::icons::view_icon(crate::icons::Icon::ChevronRight, 12, t.text),
                    ]
                    .spacing(3)
                    .align_y(Alignment::Center),
                )
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(if target_idx + 1 < total_pages {
                    Message::MovePage(target_idx, target_idx + 1)
                } else {
                    Message::MovePage(target_idx, target_idx)
                });

                let rename_btn = button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Pencil, 12, t.text),
                        text("Renomear").size(11).font(UI),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::StartEditingPageName(target_idx));

                let add_bef = button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::AddBefore, 12, t.text),
                        text("Add Antes").size(11).font(UI),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::AddPageBefore(target_idx));

                let add_aft = button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::AddAfter, 12, t.text),
                        text("Add Depois").size(11).font(UI),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::AddPageAfter(target_idx));

                let del_btn = button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Delete, 12, t.danger),
                        text("Excluir").size(11).font(UI).color(t.danger),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::DeletePage(target_idx));

                let close_btn = button(crate::icons::view_icon(
                    crate::icons::Icon::Close,
                    12,
                    t.text2,
                ))
                .style(theme::tool)
                .padding(Padding::from([3.0, 6.0]))
                .on_press(Message::ClosePageContextMenu);

                Some(
                    container(
                        row![
                            text(format!("Folha {}:", p_name))
                                .size(11)
                                .font(UI)
                                .color(t.text2),
                            rename_btn,
                            move_left_btn,
                            move_right_btn,
                            add_bef,
                            add_aft,
                            del_btn,
                            close_btn,
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .style(theme::card)
                    .padding(Padding::from([2.0, 8.0]))
                    .into(),
                )
            } else {
                None
            }
        } else {
            None
        };

    let pages_scroll = scrollable(tabs_row).direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::default(),
    ));

    let main_bar = row![pages_scroll, horizontal_space(), nav_controls,]
        .width(Length::Fill)
        .align_y(Alignment::Center);

    if let Some(ctx) = context_banner {
        column![
            container(main_bar)
                .style(theme::bar)
                .padding(Padding::from([2.0, 8.0]))
                .width(Length::Fill),
            ctx,
        ]
        .spacing(2)
        .width(Length::Fill)
        .into()
    } else {
        container(main_bar)
            .style(theme::bar)
            .padding(Padding::from([2.0, 8.0]))
            .width(Length::Fill)
            .into()
    }
}
