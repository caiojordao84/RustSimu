use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, mouse_area, pick_list,
    row, stack, text, vertical_space, Space,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, ExportFormat, ExportPageOption, ExportScope, Message};
use crate::theme::{self, tokens, UI};

/// Renderiza o modal "Sobre o RustSimu" em camada overlay elevada.
pub fn view_about_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let about_card = container(
        column![
            row![
                crate::icons::view_icon(crate::icons::Icon::Bolt, 24, t.primary),
                column![
                    text("RustSimu").size(18).font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }),
                    text("Simulador e Editor CAD Elétrico e Industrial de Alta Performance em Rust")
                        .size(11.5)
                        .color(t.text2),
                ]
                .spacing(2),
                horizontal_space(),
                button(crate::icons::view_icon(crate::icons::Icon::Close, 13, t.text2))
                    .style(theme::tool)
                    .padding(Padding::from([4.0, 6.0]))
                    .on_press(Message::CloseAbout),
            ]
            .align_y(Alignment::Center),
            horizontal_rule(1),
            text("Recursos e Conformidade Técnica:").size(12).font(Font {
                weight: iced::font::Weight::Semibold,
                ..UI
            }),
            text("- Suporte completo a arquivos CADe_SIMU (.cad) e esquemas industriais multipáginas do QElectroTech (.qet).\n- Exportação e importação em formatos modernos abertos (.json, .toml).\n- Conformidade com normas de numeração IEC 60947-1 e identificação IEC 81346.\n- Motor de simulação física em tempo real para comando, potência trifásica, motores, inversores e softstarters.\n- Arquitetura 100% nativa em Rust com interface reativa acelerada por GPU.")
                .size(11.5)
                .color(t.text),
            horizontal_rule(1),
            row![
                horizontal_space(),
                button(text("OK / Fechar").size(12).font(UI))
                    .style(theme::primary)
                    .padding(Padding::from([6.0, 16.0]))
                    .on_press(Message::CloseAbout),
            ]
        ]
        .spacing(10)
        .width(540),
    )
    .style(theme::dropdown_menu)
    .padding(Padding::from([16.0, 20.0]));

    let modal_backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .style(theme::modal_backdrop)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseAbout);

    stack![
        modal_backdrop,
        container(
            column![
                vertical_space(),
                row![horizontal_space(), about_card, horizontal_space()].align_y(Alignment::Center),
                vertical_space(),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Renderiza o modal de exportação de esquemas (Decisão D1).
pub fn view_export_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let page_options: Vec<ExportPageOption> = app
        .pages
        .iter()
        .enumerate()
        .map(|(i, p)| ExportPageOption {
            index: i,
            label: format!("Folha {}: {}", i + 1, p.name),
        })
        .collect();

    let current_page_opt = page_options
        .iter()
        .find(|p| p.index == app.export_page_index)
        .cloned()
        .or_else(|| page_options.first().cloned());

    let is_cad = app.export_format == ExportFormat::Cad;
    let is_multipage = app.pages.len() > 1;
    let valid_scopes = ExportScope::valid_scopes(app.export_format, is_multipage);

    let mut modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::ExportPdf, 22, t.primary),
            column![
                text("Exportar Esquema Elétrico").size(17).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                text("Exporte para SVG, PNG, PDF ou formato original CADe_SIMU (.cad)")
                    .size(11.5)
                    .color(t.text2),
            ]
            .spacing(2),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::Close,
                13,
                t.text2
            ))
            .style(theme::tool)
            .padding(Padding::from([4.0, 6.0]))
            .on_press(Message::CloseExportModal),
        ]
        .align_y(Alignment::Center),
        horizontal_rule(1),
        // 1. Formato
        column![
            text("Formato de Exportação:").size(12).font(Font {
                weight: iced::font::Weight::Semibold,
                ..UI
            }),
            pick_list(
                &ExportFormat::ALL[..],
                Some(app.export_format),
                Message::SelectExportFormat,
            )
            .text_size(12.0)
            .padding(Padding::from([5.0, 8.0]))
            .width(Length::Fill),
        ]
        .spacing(4),
        // 2. Escopo
        column![
            text("Abrangência / Escopo:").size(12).font(Font {
                weight: iced::font::Weight::Semibold,
                ..UI
            }),
            pick_list(
                valid_scopes,
                Some(app.export_scope),
                Message::SelectExportScope,
            )
            .text_size(12.0)
            .padding(Padding::from([5.0, 8.0]))
            .width(Length::Fill),
        ]
        .spacing(4),
    ]
    .spacing(12);

    // Se escopo for Página:
    if app.export_scope == ExportScope::Page {
        if is_multipage {
            let page_picker = column![
                text("Selecione a Folha a Exportar:").size(12).font(Font {
                    weight: iced::font::Weight::Semibold,
                    ..UI
                }),
                pick_list(page_options, current_page_opt, |opt| {
                    Message::SelectExportPage(opt.index)
                },)
                .text_size(12.0)
                .padding(Padding::from([5.0, 8.0]))
                .width(Length::Fill),
            ]
            .spacing(4);
            modal_content = modal_content.push(page_picker);
        } else {
            let single_page_info = text(format!(
                "Folha a ser exportada: Folha 1 ({})",
                app.pages
                    .first()
                    .map(|p| p.name.as_str())
                    .unwrap_or("Principal")
            ))
            .size(11.5)
            .color(t.text2);
            modal_content = modal_content.push(single_page_info);
        }
    }

    // Se escopo for Seleção:
    if app.export_scope == ExportScope::Selection {
        let sel_info = if let Some(id) = app.selected_id {
            let item_label = app
                .doc
                .items
                .iter()
                .find(|i| i.id == id)
                .map(|i| {
                    if !i.tag.is_empty() {
                        format!("{} (Tipo {})", i.tag, i.tipo)
                    } else {
                        format!("Item #{} (Tipo {})", i.id, i.tipo)
                    }
                })
                .unwrap_or_else(|| format!("Item #{id}"));

            container(
                row![
                    crate::icons::view_icon(crate::icons::Icon::Info, 16, t.primary),
                    column![
                        text(format!("Exportando elemento selecionado: {item_label}"))
                            .size(11.5)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            })
                            .color(t.primary),
                        text("As dimensões do arquivo gerado serão calculadas dinamicamente com base nas bordas do componente.")
                            .size(11.0)
                            .color(t.text2),
                    ]
                    .spacing(2),
                ]
                .spacing(8)
                .align_y(Alignment::Start),
            )
            .style(theme::card)
            .padding(Padding::from([8.0, 10.0]))
        } else {
            container(
                row![
                    crate::icons::view_icon(crate::icons::Icon::Info, 16, t.amber),
                    column![
                        text("Nenhum elemento selecionado na tela!")
                            .size(11.5)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            })
                            .color(t.amber),
                        text("Clique em um componente no canvas antes de exportar a seleção, ou alterne o escopo para Folha Atual.")
                            .size(11.0)
                            .color(t.text2),
                    ]
                    .spacing(2),
                ]
                .spacing(8)
                .align_y(Alignment::Start),
            )
            .style(theme::card)
            .padding(Padding::from([8.0, 10.0]))
        };
        modal_content = modal_content.push(sel_info);
    }

    // Se escopo for Projeto:
    if app.export_scope == ExportScope::Project {
        let project_notice = container(
            row![
                crate::icons::view_icon(crate::icons::Icon::ExportPdf, 16, t.primary),
                column![
                    text(format!(
                        "Exportando Projeto Completo ({} folhas):",
                        app.pages.len()
                    ))
                    .size(11.5)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    })
                    .color(t.primary),
                    text("Todas as páginas do projeto serão empacotadas em ordem cronológica num único documento PDF multipágina.")
                        .size(11.0)
                        .color(t.text2),
                ]
                .spacing(2),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
        )
        .style(theme::card)
        .padding(Padding::from([8.0, 10.0]));
        modal_content = modal_content.push(project_notice);
    }

    // Opção Cajetín (apenas se não for seleção)
    if app.export_scope != ExportScope::Selection {
        let cajetin_box = column![
            checkbox(
                "Incluir Moldura e Cajetín (Margens, Réguas e Carimbo)",
                app.export_include_cajetin,
            )
            .size(15)
            .text_size(12.0)
            .on_toggle(Message::ToggleExportCajetin),
            text("Se desmarcado, exporta a folha limpa contendo estritamente os circuitos sem moldura externa.")
                .size(10.5)
                .color(t.text2),
        ]
        .spacing(3);
        modal_content = modal_content.push(cajetin_box);
    }

    // Opções visuais para formatos gráficos (SVG, PNG, PDF)
    if !is_cad {
        let visual_options = column![
            column![
                checkbox(
                    "Mostrar Bornes de Conexão (Pontos Vermelhos)",
                    app.export_show_terminals,
                )
                .size(15)
                .text_size(12.0)
                .on_toggle(Message::ToggleExportShowTerminals),
                text("Por padrão desativado para saída limpa de impressão. Ative se desejar exibir os pontos de terminal.")
                    .size(10.5)
                    .color(t.text2),
            ]
            .spacing(3),
            column![
                checkbox(
                    "Modo Monocromático / Alto Contraste (Impressão P&B)",
                    app.export_monochrome,
                )
                .size(15)
                .text_size(12.0)
                .on_toggle(Message::ToggleExportMonochrome),
                text("Converte traços e anotações para preto e tons de cinza para melhor legibilidade em impressões.")
                    .size(10.5)
                    .color(t.text2),
            ]
            .spacing(3),
        ]
        .spacing(8);
        modal_content = modal_content.push(visual_options);
    }

    if is_cad {
        let cad_notice = container(
            row![
                crate::icons::view_icon(crate::icons::Icon::Info, 16, t.amber),
                column![
                    text("Compatibilidade CADe_SIMU 4.0 (.cad):")
                        .size(11.5)
                        .font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        })
                        .color(t.amber),
                    text("O formato legado original é estritamente monofolha. A folha selecionada será exportada em formato byte-exato (100% compatível com o CADe_SIMU.exe). Para salvar o projeto completo com todas as páginas, utilize Salvar (JSON).")
                        .size(11.0)
                        .color(t.text2),
                ]
                .spacing(2),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
        )
        .style(theme::card)
        .padding(Padding::from([8.0, 10.0]));

        modal_content = modal_content.push(cad_notice);
    }

    modal_content = modal_content.push(horizontal_rule(1)).push(
        row![
            button(text("Cancelar").size(12).font(UI))
                .style(theme::tool)
                .padding(Padding::from([6.0, 14.0]))
                .on_press(Message::CloseExportModal),
            horizontal_space(),
            button(text("Exportar...").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }))
            .style(theme::primary)
            .padding(Padding::from([6.0, 18.0]))
            .on_press(Message::ConfirmExport),
        ]
        .align_y(Alignment::Center),
    );

    let export_card = container(modal_content.width(520))
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    let modal_backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .style(theme::modal_backdrop)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseExportModal);

    stack![
        modal_backdrop,
        container(
            column![
                vertical_space(),
                row![horizontal_space(), export_card, horizontal_space()]
                    .align_y(Alignment::Center),
                vertical_space(),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
