use iced::widget::{
    button, column, container, mouse_area, row, scrollable, stack, text, text_input, tooltip,
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
        snap_to_grid: app.snap_to_grid,
        grid_density: app.grid_density,
        show_texts: app.show_texts,
        show_terminals: app.show_terminals,
        show_cajetin: app.show_cajetin,
        show_full_crosshair: app.show_full_crosshair,
        show_text_grid: app.show_text_grid,
        show_alignment_guides: app.show_alignment_guides,
        keep_headers_visible: app.keep_headers_visible,
        show_cell_bounds: app.show_cell_bounds,
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
            || app.show_print_modal
            || app.show_page_properties_modal
            || app.show_project_properties_modal
            || app.show_settings_modal
            || app.show_shortcuts_modal
            || app.show_color_modal
            || app.show_confirm_delete_modal
            || app.show_close_last_page_modal,
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
    const TITLE_LINE_H: f32 = 30.0;
    const TITLE_BTN_SIZE: f32 = 24.0;
    let t = tokens(&app.theme());

    let doc_name = if let Some(ref p) = app.current_file {
        p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.display().to_string())
    } else if !app.file_path.is_empty() {
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

    let mut title_row = row![
        crate::icons::view_icon(crate::icons::Icon::Copy, 14, t.text2),
        text(doc_name)
            .size(13)
            .font(iced::font::Font {
                weight: iced::font::Weight::Medium,
                ..UI
            })
            .color(t.text),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if app.is_modified {
        title_row = title_row.push(
            container(
                text("[Modificado]")
                    .size(11)
                    .font(iced::font::Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    })
                    .color(t.amber),
            )
            .padding(Padding::from([1.0, 6.0]))
            .style(theme::card),
        );
    }

    let square_action_btn = |icon: crate::icons::Icon, tip: &'static str, msg: Message| {
        app_tooltip(
            button(
                container(crate::icons::view_icon(icon, 14, t.text))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .style(theme::tool)
            .padding(0)
            .width(Length::Fixed(TITLE_BTN_SIZE))
            .height(Length::Fixed(TITLE_BTN_SIZE))
            .on_press(msg),
            tip,
            tooltip::Position::Bottom,
        )
    };

    let actions = row![
        square_action_btn(
            crate::icons::Icon::New,
            "Novo Esquema (Ctrl+N)",
            Message::NewDocument
        ),
        square_action_btn(
            crate::icons::Icon::Open,
            "Abrir Esquema (Ctrl+O)",
            Message::OpenFileDialog
        ),
        square_action_btn(
            crate::icons::Icon::Close,
            "Fechar Esquema (Ctrl+W)",
            Message::CloseDocument
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let bar_content = stack![
        container(title_row)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        container(actions)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .center_y(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(bar_content)
        .style(theme::bar)
        .padding(Padding::from([3.0, 8.0]))
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_LINE_H))
        .into()
}

pub fn view_canvas_pages_bar<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());
    let total_pages = app.pages.len();

    let mut tabs_row = row![].spacing(4).align_y(Alignment::Center);

    for (idx, page) in app.pages.iter().enumerate() {
        let is_active = idx == app.active_page_index;
        let is_editing_name =
            app.editing_page_target == Some(idx) && app.editing_page_name.is_some();
        let is_editing_number = app
            .editing_page_number
            .as_ref()
            .is_some_and(|(target, _)| *target == idx);

        let tab_content: Element<'_, Message> = if is_editing_name {
            let current_input = app.editing_page_name.as_deref().unwrap_or(&page.name);
            row![
                text(format!("{}.", idx + 1))
                    .size(11)
                    .font(MONO)
                    .color(t.text2),
                text_input("Nome da página", current_input)
                    .on_input(Message::UpdateEditingPageName)
                    .on_submit(Message::CommitEditingPageName)
                    .padding(Padding::from([2.0, 6.0]))
                    .width(130),
                button(text("OK").size(11).font(UI))
                    .style(theme::primary)
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::CommitEditingPageName),
            ]
            .spacing(3)
            .align_y(Alignment::Center)
            .into()
        } else if is_editing_number {
            let current_num = app
                .editing_page_number
                .as_ref()
                .map(|(_, s)| s.as_str())
                .unwrap_or("");
            row![
                text("Nº:").size(11).font(UI).color(t.text2),
                text_input("1", current_num)
                    .on_input(Message::UpdateEditingPageNumber)
                    .on_submit(Message::CommitEditingPageNumber)
                    .padding(Padding::from([2.0, 6.0]))
                    .width(42),
                button(text("OK").size(11).font(UI))
                    .style(theme::primary)
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::CommitEditingPageNumber),
            ]
            .spacing(3)
            .align_y(Alignment::Center)
            .into()
        } else {
            let tab_label = format!("{}. {}", idx + 1, page.name);
            let main_btn = button(text(tab_label).size(12).font(UI))
                .style(theme::tab(is_active))
                .padding(Padding::from([4.0, 10.0]))
                .on_press(Message::SelectPage(idx));

            mouse_area(main_btn)
                .on_right_press(Message::TogglePageContextMenu(idx))
                .into()
        };

        let tab_box = container(tab_content)
            .padding(Padding::from([1.0, 2.0]))
            .style(theme::page_tab_box(is_active));

        tabs_row = tabs_row.push(tab_box);
    }

    // Atalho no canto esquerdo: criar página antes
    let add_before_btn = app_tooltip(
        button(
            container(crate::icons::view_icon(
                crate::icons::Icon::AddBefore,
                14,
                t.text,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .style(theme::tool)
        .padding(0)
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .on_press(Message::AddPageBefore(app.active_page_index)),
        "Criar página antes da atual",
        tooltip::Position::Bottom,
    );

    // Atalho no canto direito: criar página depois
    let add_after_btn = app_tooltip(
        button(
            container(crate::icons::view_icon(
                crate::icons::Icon::AddAfter,
                14,
                t.text,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .style(theme::tool)
        .padding(0)
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .on_press(Message::AddPageAfter(app.active_page_index)),
        "Criar página depois da atual",
        tooltip::Position::Bottom,
    );

    let context_banner: Option<Element<'_, Message>> =
        if let Some(target_idx) = app.page_context_menu {
            if target_idx < total_pages {
                let p_name = &app.pages[target_idx].name;

                let edit_name_btn = button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Pencil, 12, t.text),
                        text("Editar nome").size(11).font(UI),
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
                        text("Criar página antes").size(11).font(UI),
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
                        text("Criar página depois").size(11).font(UI),
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
                        text("Deletar página").size(11).font(UI).color(t.danger),
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

                let mut ctx_row = row![
                    text(format!("Folha {} ({}):", target_idx + 1, p_name))
                        .size(11)
                        .font(UI)
                        .color(t.text2),
                    edit_name_btn,
                ]
                .spacing(4)
                .align_y(Alignment::Center);

                if total_pages > 1 {
                    let edit_num_btn = button(
                        row![
                            crate::icons::view_icon(crate::icons::Icon::Group, 12, t.text),
                            text("Editar número").size(11).font(UI),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .style(theme::tool)
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::StartEditingPageNumber(target_idx));
                    ctx_row = ctx_row.push(edit_num_btn);
                }

                ctx_row = ctx_row
                    .push(add_bef)
                    .push(add_aft)
                    .push(del_btn)
                    .push(close_btn);

                Some(
                    container(ctx_row)
                        .style(theme::card)
                        .padding(Padding::from([3.0, 8.0]))
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

    let main_bar = row![
        add_before_btn,
        container(pages_scroll).width(Length::Fill),
        add_after_btn,
    ]
    .spacing(6)
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
