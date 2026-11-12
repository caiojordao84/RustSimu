use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, mouse_area, pick_list,
    row, scrollable, stack, text, text_input, vertical_space, Space,
};
use iced::{Alignment, Element, Font, Length, Padding};

use crate::app::{CadeSimuApp, ExportFormat, ExportPageOption, ExportScope, Message};
use crate::model::GridDensity;
use crate::theme::{self, tokens, ThemeMode, MONO, UI};

/// Envolve um cartão de modal em overlay escuro centralizado e clique-fora para fechar.
pub(crate) fn wrap_modal<'a>(
    card: impl Into<Element<'a, Message>>,
    on_close: Message,
) -> Element<'a, Message> {
    let modal_backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .style(theme::modal_backdrop)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(on_close);

    stack![
        modal_backdrop,
        container(
            column![
                vertical_space(),
                row![horizontal_space(), card.into(), horizontal_space()]
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

    wrap_modal(about_card, Message::CloseAbout)
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
                text("Exporte para PNG, JPG, BMP, SVG, DXF, CADe_SIMU (.cad) ou PDF vetorial")
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

    wrap_modal(export_card, Message::CloseExportModal)
}

/// Renderiza o modal de impressão vetorial em camada overlay elevada.
pub fn view_print_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
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
        .find(|p| p.index == app.print_page_index)
        .cloned()
        .or_else(|| page_options.first().cloned());

    let is_multipage = app.pages.len() > 1;
    let print_scopes = if is_multipage {
        vec![ExportScope::Page, ExportScope::Project]
    } else {
        vec![ExportScope::Page]
    };

    let mut modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::Print, 22, t.primary),
            column![
                text("Imprimir Esquema Elétrico").size(17).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                text("Gera documento PDF vetorial em alta definição pronto para impressão")
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
            .on_press(Message::ClosePrintModal),
        ]
        .align_y(Alignment::Center),
        horizontal_rule(1),
        column![
            text("Abrangência da Impressão:").size(12).font(Font {
                weight: iced::font::Weight::Semibold,
                ..UI
            }),
            pick_list(
                print_scopes,
                Some(app.print_scope),
                Message::SelectPrintScope,
            )
            .text_size(12.0)
            .padding(Padding::from([5.0, 8.0]))
            .width(Length::Fill),
        ]
        .spacing(4),
    ]
    .spacing(12);

    if app.print_scope == ExportScope::Page && is_multipage {
        modal_content = modal_content.push(
            column![
                text("Folha a Imprimir:").size(12).font(Font {
                    weight: iced::font::Weight::Semibold,
                    ..UI
                }),
                pick_list(page_options, current_page_opt, |opt| {
                    Message::SelectPrintPage(opt.index)
                })
                .text_size(12.0)
                .padding(Padding::from([5.0, 8.0]))
                .width(Length::Fill),
            ]
            .spacing(4),
        );
    }

    modal_content = modal_content
        .push(
            checkbox(
                "Incluir Moldura e Cajetín Técnico",
                app.print_include_cajetin,
            )
            .size(15)
            .text_size(12.0)
            .on_toggle(Message::TogglePrintCajetin),
        )
        .push(
            checkbox(
                "Impressão em Preto e Branco (Alto Contraste)",
                app.print_monochrome,
            )
            .size(15)
            .text_size(12.0)
            .on_toggle(Message::TogglePrintMonochrome),
        )
        .push(
            container(
                row![
                    crate::icons::view_icon(crate::icons::Icon::Info, 16, t.primary),
                    text("O RustSimu renderiza o circuito em escala A4 vetorial sem pixelização e abre o visualizador/spooler de impressão padrão do sistema.")
                        .size(11.0)
                        .color(t.text2),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .style(theme::card)
            .padding(Padding::from([8.0, 10.0])),
        )
        .push(horizontal_rule(1))
        .push(
            row![
                button(text("Cancelar").size(12).font(UI))
                    .style(theme::tool)
                    .padding(Padding::from([6.0, 14.0]))
                    .on_press(Message::ClosePrintModal),
                horizontal_space(),
                button(text("Salvar PDF...").size(12).font(UI))
                    .style(theme::tool)
                    .padding(Padding::from([6.0, 12.0]))
                    .on_press(Message::SavePrintPdf),
                button(text("Imprimir / Abrir PDF").size(12).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }))
                .style(theme::primary)
                .padding(Padding::from([6.0, 16.0]))
                .on_press(Message::ConfirmPrint),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );

    let print_card = container(modal_content.width(500))
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    wrap_modal(print_card, Message::ClosePrintModal)
}

/// Renderiza o modal de propriedades da página ativa (Cajetín, dimensões e linhas/colunas da grade).
pub fn view_page_properties_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());
    let cols = app.doc.settings.effective_grid_cols();
    let rows = app.doc.settings.effective_grid_rows();

    let field_block = |label: &'static str,
                       placeholder: &'static str,
                       value: &str,
                       on_input: fn(String) -> Message| {
        column![
            text(label).size(11.5).color(t.text2),
            text_input(placeholder, value)
                .on_input(on_input)
                .padding(Padding::from([6.0, 8.0]))
                .size(12)
                .style(theme::field),
        ]
        .spacing(4)
        .width(Length::Fill)
    };

    let modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::Grid, 20, t.primary),
            text("Propriedades da Página e Cajetín")
                .size(16)
                .font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::Close,
                14,
                t.text2
            ))
            .style(theme::tool)
            .padding(Padding::from([4.0, 6.0]))
            .on_press(Message::ClosePagePropertiesModal),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        horizontal_rule(1),
        field_block(
            "Nome da Folha:",
            "Ex: F1 - Circuito de Comando",
            &app.page_prop_name,
            Message::EditPagePropName,
        ),
        row![
            field_block(
                "Título do Cajetín:",
                "Título do desenho",
                &app.page_prop_title,
                Message::EditPagePropTitle,
            ),
            field_block(
                "Subtítulo:",
                "Subtítulo técnico",
                &app.page_prop_subtitle,
                Message::EditPagePropSubtitle,
            ),
        ]
        .spacing(10),
        row![
            field_block(
                "Entidade / Empresa:",
                "Empresa ou cliente",
                &app.page_prop_entity,
                Message::EditPagePropEntity,
            ),
            field_block(
                "Autor / Projetista:",
                "Nome do responsável",
                &app.page_prop_author,
                Message::EditPagePropAuthor,
            ),
        ]
        .spacing(10),
        row![
            field_block(
                "Largura da Folha (mm):",
                "297",
                &app.page_prop_width_mm,
                Message::EditPagePropWidthMm,
            ),
            field_block(
                "Altura da Folha (mm):",
                "210",
                &app.page_prop_height_mm,
                Message::EditPagePropHeightMm,
            ),
        ]
        .spacing(10),
        container(
            column![
                text("Linhas e Colunas da Moldura (Referência Cruzada)")
                    .size(12)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }),
                row![
                    row![
                        text(format!("Colunas: {}", cols)).size(12).color(t.text),
                        button(text("-").size(12).font(UI))
                            .style(theme::tool)
                            .padding(Padding::from([4.0, 10.0]))
                            .on_press(Message::RemoveGridCol),
                        button(text("+").size(12).font(UI))
                            .style(theme::tool)
                            .padding(Padding::from([4.0, 10.0]))
                            .on_press(Message::AddGridCol),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    horizontal_space(),
                    row![
                        text(format!("Linhas: {}", rows)).size(12).color(t.text),
                        button(text("-").size(12).font(UI))
                            .style(theme::tool)
                            .padding(Padding::from([4.0, 10.0]))
                            .on_press(Message::RemoveGridRow),
                        button(text("+").size(12).font(UI))
                            .style(theme::tool)
                            .padding(Padding::from([4.0, 10.0]))
                            .on_press(Message::AddGridRow),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                ]
                .align_y(Alignment::Center),
                text("A moldura e a referência cruzada (Xref) atualizam automaticamente.")
                    .size(10.5)
                    .color(t.text2),
            ]
            .spacing(6),
        )
        .style(theme::card)
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill),
        horizontal_rule(1),
        row![
            horizontal_space(),
            button(text("Cancelar").size(12).font(UI))
                .style(theme::pill_ghost)
                .padding(Padding::from([6.0, 14.0]))
                .on_press(Message::ClosePagePropertiesModal),
            button(text("Aplicar Alterações").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }))
            .style(theme::primary)
            .padding(Padding::from([6.0, 16.0]))
            .on_press(Message::ApplyPageProperties),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(520);

    let card = container(modal_content)
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    wrap_modal(card, Message::ClosePagePropertiesModal)
}

/// Renderiza o modal de seleção e personalização de cor do elemento.
pub fn view_color_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let targets_count = app
        .selected_ids
        .len()
        .max(if app.selected_id.is_some() { 1 } else { 0 });

    let presets: [(i32, &'static str, &'static str); 12] = [
        (0x00000000, "Padrão", "Dinâmico"),
        (0x00E53935, "Vermelho", "Comando"),
        (0x001E88E5, "Azul", "Neutro/CC"),
        (0x0000ACC1, "Ciano", "Instrumento"),
        (0x0043A047, "Verde", "Terra PE"),
        (0x00FDD835, "Amarelo", "Alerta"),
        (0x00FB8C00, "Laranja", "Força"),
        (0x008E24AA, "Violeta", "Controle"),
        (0x006D4C41, "Marrom", "Fase L1"),
        (0x00212121, "Preto", "Potência"),
        (0x00757575, "Cinza", "Fase L2"),
        (0x00FFFFFF, "Branco", "Sinal"),
    ];

    let swatch_btn = |code: i32, name: &'static str, sub: &'static str| {
        let is_sel = app.pending_color_int == code;
        let swatch_col = if code != 0 {
            let r = ((code >> 16) & 0xFF) as f32 / 255.0;
            let g = ((code >> 8) & 0xFF) as f32 / 255.0;
            let b = (code & 0xFF) as f32 / 255.0;
            iced::Color::from_rgb(r, g, b)
        } else {
            t.ink
        };

        button(
            row![
                container(Space::with_width(14).height(14)).style(theme::lamp_swatch(swatch_col)),
                column![
                    text(name).size(10.5).font(Font {
                        weight: if is_sel {
                            iced::font::Weight::Bold
                        } else {
                            iced::font::Weight::Normal
                        },
                        ..UI
                    }),
                    text(sub).size(9.0).color(t.text2).font(UI),
                ]
                .spacing(1),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .style(if is_sel {
            theme::tool_active
        } else {
            theme::tool
        })
        .padding(Padding::from([4.0, 6.0]))
        .width(Length::Fill)
        .on_press(Message::SelectColor(code))
    };

    let mut row1 = row![].spacing(6).align_y(Alignment::Center);
    for &(code, name, sub) in &presets[0..4] {
        row1 = row1.push(swatch_btn(code, name, sub));
    }

    let mut row2 = row![].spacing(6).align_y(Alignment::Center);
    for &(code, name, sub) in &presets[4..8] {
        row2 = row2.push(swatch_btn(code, name, sub));
    }

    let mut row3 = row![].spacing(6).align_y(Alignment::Center);
    for &(code, name, sub) in &presets[8..12] {
        row3 = row3.push(swatch_btn(code, name, sub));
    }

    let preview_color = if app.pending_color_int != 0 {
        let r = ((app.pending_color_int >> 16) & 0xFF) as f32 / 255.0;
        let g = ((app.pending_color_int >> 8) & 0xFF) as f32 / 255.0;
        let b = (app.pending_color_int & 0xFF) as f32 / 255.0;
        iced::Color::from_rgb(r, g, b)
    } else {
        t.ink
    };

    let preview_card = container(
        row![
            container(Space::with_width(26).height(26)).style(theme::lamp_swatch(preview_color)),
            column![
                text("Pré-visualização da Cor:")
                    .size(10.5)
                    .color(t.text2)
                    .font(UI),
                text(if app.pending_color_int == 0 {
                    "Cor Dinâmica do Tema".to_string()
                } else {
                    format!("#{:06X}", app.pending_color_int & 0xFFFFFF)
                })
                .size(12.0)
                .font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
            ]
            .spacing(1),
            horizontal_space(),
            row![
                text("Código Hex:").size(11.0).font(UI).color(t.text),
                container(
                    text_input("#000000", &app.pending_color_hex)
                        .on_input(Message::InputColorHex)
                        .size(11.5)
                        .padding(Padding::from([3.0, 6.0]))
                        .width(90)
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(theme::card)
    .padding(Padding::from([8.0, 12.0]))
    .width(Length::Fill);

    let modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::Palette, 20, t.primary),
            column![
                text("Colorir Elemento(s)").size(15).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                text(format!(
                    "Substitui a cor de traço de {targets_count} elemento(s) no esquema elétrico."
                ))
                .size(11.0)
                .color(t.text2),
            ]
            .spacing(1),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::Close,
                12,
                t.text2
            ))
            .style(theme::tool)
            .padding(Padding::from([4.0, 6.0]))
            .on_press(Message::CloseColorModal),
        ]
        .align_y(Alignment::Center),
        horizontal_rule(1),
        text("Paleta Padrão Industrial e de Controle:")
            .size(11.5)
            .font(Font {
                weight: iced::font::Weight::Semibold,
                ..UI
            }),
        column![row1, row2, row3].spacing(6),
        preview_card,
        horizontal_rule(1),
        row![
            button(text("Restaurar Padrão").size(11.5).font(UI))
                .style(theme::tool)
                .padding(Padding::from([6.0, 12.0]))
                .on_press(Message::ApplyColor(0)),
            horizontal_space(),
            button(text("Cancelar").size(11.5).font(UI))
                .style(theme::tool)
                .padding(Padding::from([6.0, 12.0]))
                .on_press(Message::CloseColorModal),
            button(text("Aplicar Cor").size(11.5).font(UI))
                .style(theme::primary)
                .padding(Padding::from([6.0, 16.0]))
                .on_press(Message::ConfirmColorModal),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(520);

    let color_card = container(modal_content)
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    let modal_backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .style(theme::modal_backdrop)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CloseColorModal);

    stack![
        modal_backdrop,
        container(
            column![
                vertical_space(),
                row![horizontal_space(), color_card, horizontal_space()].align_y(Alignment::Center),
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

/// Renderiza o modal de confirmação para remoção de projeto recente.
pub fn view_confirm_delete_project_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let (project_name, project_path) = if let Some(ref path_str) = app.project_to_delete {
        let p = std::path::Path::new(path_str);
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        (name, path_str.clone())
    } else {
        ("Projeto Desconhecido".to_string(), String::new())
    };

    let dialog_card = container(
        column![
            row![
                crate::icons::view_icon(crate::icons::Icon::Delete, 22, t.danger),
                column![
                    text("Remover dos Projetos Recentes").size(16).font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }),
                    text("Confirmação de exclusão da lista de acessos rápidos")
                        .size(11.0)
                        .color(t.text2),
                ]
                .spacing(2),
                horizontal_space(),
                button(crate::icons::view_icon(crate::icons::Icon::Close, 13, t.text2))
                    .style(theme::tool)
                    .padding(Padding::from([4.0, 6.0]))
                    .on_press(Message::CancelDeleteRecentProject),
            ]
            .align_y(Alignment::Center),
            horizontal_rule(1),
            text(format!(
                "Deseja realmente remover o projeto \"{}\" da lista recente?",
                project_name
            ))
            .size(12.5)
            .font(Font {
                weight: iced::font::Weight::Medium,
                ..UI
            }),
            container(
                column![
                    row![
                        text("Caminho:").size(10.5).color(t.text2),
                        text(project_path)
                            .size(10.5)
                            .font(crate::theme::MONO)
                            .color(t.text)
                            .width(Length::Fill),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    text("Nota: Esta ação remove apenas o atalho recente no RustSimu. O arquivo no disco rígido permanece intacto.")
                        .size(10.0)
                        .color(t.text2),
                ]
                .spacing(4)
            )
            .style(theme::card)
            .padding(Padding::from([8.0, 10.0]))
            .width(Length::Fill),
            horizontal_rule(1),
            row![
                horizontal_space(),
                button(text("Cancelar").size(12.0).font(UI))
                    .style(theme::tool)
                    .padding(Padding::from([6.0, 14.0]))
                    .on_press(Message::CancelDeleteRecentProject),
                button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Delete, 13, iced::Color::WHITE),
                        text("Apagar da Lista").size(12.0).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        }),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                )
                .style(theme::danger)
                .padding(Padding::from([6.0, 14.0]))
                .on_press(Message::ConfirmDeleteRecentProject),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(12)
        .width(480),
    )
    .style(theme::dropdown_menu)
    .padding(Padding::from([16.0, 18.0]));

    let modal_backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .style(theme::modal_backdrop)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::CancelDeleteRecentProject);

    stack![
        modal_backdrop,
        container(
            column![
                vertical_space(),
                row![horizontal_space(), dialog_card, horizontal_space()]
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

/// Renderiza o modal de propriedades globais do projeto.
pub fn view_project_properties_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let field_block = |label: &'static str,
                       placeholder: &'static str,
                       value: &str,
                       on_input: fn(String) -> Message| {
        column![
            text(label).size(11.5).color(t.text2),
            text_input(placeholder, value)
                .on_input(on_input)
                .padding(Padding::from([6.0, 8.0]))
                .size(12)
                .style(theme::field),
        ]
        .spacing(4)
        .width(Length::Fill)
    };

    let modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::Folder, 20, t.primary),
            text("Propriedades do Projeto").size(16).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::Close,
                14,
                t.text2
            ))
            .style(theme::tool)
            .padding(Padding::from([4.0, 6.0]))
            .on_press(Message::CloseProjectPropertiesModal),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        horizontal_rule(1),
        container(
            row![
                text(format!("Folhas no projeto: {}", app.pages.len()))
                    .size(11.5)
                    .color(t.text),
                horizontal_space(),
                text(format!("Elementos na folha ativa: {}", app.doc.items.len()))
                    .size(11.5)
                    .color(t.text2),
            ]
            .align_y(Alignment::Center),
        )
        .style(theme::card)
        .padding(Padding::from([8.0, 10.0]))
        .width(Length::Fill),
        field_block(
            "Título Global do Projeto:",
            "Ex: Quadro de Comando Geral QCG-01",
            &app.proj_prop_title,
            Message::EditProjectPropTitle,
        ),
        row![
            field_block(
                "Entidade / Cliente:",
                "Nome da empresa",
                &app.proj_prop_entity,
                Message::EditProjectPropEntity,
            ),
            field_block(
                "Autor / Projetista:",
                "Responsável técnico",
                &app.proj_prop_author,
                Message::EditProjectPropAuthor,
            ),
        ]
        .spacing(10),
        field_block(
            "Revisão / Assinatura Técnica:",
            "Ex: Rev 1.0",
            &app.proj_prop_version,
            Message::EditProjectPropVersion,
        ),
        container(
            column![
                text("Comportamento de Edição e Conexões")
                    .size(12)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }),
                checkbox(
                    "Autoconectar fios ao posicionar sobre terminais",
                    app.auto_connect
                )
                .on_toggle(|_| Message::ToggleAutoConnect)
                .text_size(12),
                checkbox(
                    "Desconectar fios automaticamente ao mover elementos",
                    app.auto_disconnect
                )
                .on_toggle(|_| Message::ToggleAutoDisconnect)
                .text_size(12),
                checkbox(
                    "Habilitar salvamento automático de segurança",
                    app.proj_prop_auto_save
                )
                .on_toggle(Message::ToggleProjectAutoSave)
                .text_size(12),
            ]
            .spacing(6),
        )
        .style(theme::card)
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill),
        horizontal_rule(1),
        row![
            horizontal_space(),
            button(text("Cancelar").size(12).font(UI))
                .style(theme::pill_ghost)
                .padding(Padding::from([6.0, 14.0]))
                .on_press(Message::CloseProjectPropertiesModal),
            button(text("Aplicar ao Projeto").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }))
            .style(theme::primary)
            .padding(Padding::from([6.0, 16.0]))
            .on_press(Message::ApplyProjectProperties),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .width(510);

    let card = container(modal_content)
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    wrap_modal(card, Message::CloseProjectPropertiesModal)
}

/// Renderiza o modal de configurações gerais do aplicativo e preferências visuais.
pub fn view_settings_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let theme_btn = |label: &'static str, mode: ThemeMode| {
        let active = app.theme_mode == mode;
        button(text(label).size(12).font(UI))
            .style(if active { theme::primary } else { theme::tool })
            .padding(Padding::from([6.0, 12.0]))
            .on_press(Message::SetThemeMode(mode))
    };

    let density_btn = |label: &'static str, density: GridDensity| {
        let active = app.grid_density == density;
        button(text(label).size(11.5).font(UI))
            .style(if active { theme::primary } else { theme::tool })
            .padding(Padding::from([5.0, 10.0]))
            .on_press(Message::SetGridDensity(density))
    };

    let bg_btn = |label: &'static str, preset: Option<i32>| {
        let active = app.doc.settings.sheet_bg_color == preset;
        button(text(label).size(11.5).font(UI))
            .style(if active { theme::primary } else { theme::tool })
            .padding(Padding::from([5.0, 10.0]))
            .on_press(Message::SetSheetBgPreset(preset))
    };

    let modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::Palette, 20, t.primary),
            text("Configurações Gerais").size(16).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::Close,
                14,
                t.text2
            ))
            .style(theme::tool)
            .padding(Padding::from([4.0, 6.0]))
            .on_press(Message::CloseSettingsModal),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        horizontal_rule(1),
        container(
            column![
                text("Tema da Interface").size(12).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                row![
                    theme_btn("Automático (Sistema)", ThemeMode::System),
                    theme_btn("Tema Claro", ThemeMode::Light),
                    theme_btn("Tema Escuro", ThemeMode::Dark),
                ]
                .spacing(8),
            ]
            .spacing(8),
        )
        .style(theme::card)
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill),
        container(
            column![
                text("Grelha, Snap e Passo").size(12).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                row![
                    checkbox("Mostrar a Grelha", app.show_grid)
                        .on_toggle(Message::ToggleGrid)
                        .text_size(12),
                    horizontal_space(),
                    checkbox("Ajustar à Grelha (Snap)", app.snap_to_grid)
                        .on_toggle(|_| Message::ToggleSnap)
                        .text_size(12),
                ]
                .align_y(Alignment::Center),
                row![
                    density_btn("Fina (2 mm)", GridDensity::Fine),
                    density_btn("Média (4 mm)", GridDensity::Medium),
                    density_btn("Larga (8 mm)", GridDensity::Coarse),
                ]
                .spacing(8),
            ]
            .spacing(8),
        )
        .style(theme::card)
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill),
        container(
            column![
                text("Auxílios Visuais e Folha").size(12).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                checkbox("Mira em Todo o Fólio (Crosshair)", app.show_full_crosshair)
                    .on_toggle(|_| Message::ToggleFullCrosshair)
                    .text_size(12),
                checkbox("Ver os Guias de Alinhamento", app.show_alignment_guides)
                    .on_toggle(|_| Message::ToggleAlignmentGuides)
                    .text_size(12),
                checkbox(
                    "Manter Cabeçalhos Visíveis ao Rolar",
                    app.keep_headers_visible
                )
                .on_toggle(|_| Message::ToggleKeepHeadersVisible)
                .text_size(12),
                checkbox("Mostrar Limites das Células (Xref)", app.show_cell_bounds)
                    .on_toggle(|_| Message::ToggleCellBounds)
                    .text_size(12),
                text("Cor de Fundo da Folha:").size(11.5).color(t.text2),
                row![
                    bg_btn("Padrão", None),
                    bg_btn("Branco", Some(0xFFFFFF)),
                    bg_btn("Creme", Some(0xFAF5E4)),
                    bg_btn("Azul Técnico", Some(0xEAF2FA)),
                ]
                .spacing(6),
            ]
            .spacing(6),
        )
        .style(theme::card)
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill),
        horizontal_rule(1),
        row![
            horizontal_space(),
            button(text("Fechar").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }))
            .style(theme::primary)
            .padding(Padding::from([6.0, 18.0]))
            .on_press(Message::CloseSettingsModal),
        ],
    ]
    .spacing(10)
    .width(510);

    let card = container(modal_content)
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    wrap_modal(card, Message::CloseSettingsModal)
}

/// Renderiza o guia completo de atalhos de teclado do RustSimu.
pub fn view_shortcuts_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());

    let shortcut_row = |keys: &'static str, desc: &'static str| {
        row![
            container(text(keys).size(11).font(MONO).color(t.primary),)
                .style(theme::card)
                .padding(Padding::from([2.0, 6.0]))
                .width(Length::Fixed(160.0)),
            text(desc).size(12).color(t.text).width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
    };

    let section = |title: &'static str, items: Vec<(&'static str, &'static str)>| {
        let mut col = column![text(title)
            .size(12.5)
            .font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            })
            .color(t.text2)]
        .spacing(5);
        for (k, d) in items {
            col = col.push(shortcut_row(k, d));
        }
        container(col)
            .style(theme::card)
            .padding(Padding::from([8.0, 10.0]))
            .width(Length::Fill)
    };

    let modal_content = column![
        row![
            crate::icons::view_icon(crate::icons::Icon::Info, 20, t.primary),
            text("Atalhos de Teclado").size(16).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }),
            horizontal_space(),
            button(crate::icons::view_icon(
                crate::icons::Icon::Close,
                14,
                t.text2
            ))
            .style(theme::tool)
            .padding(Padding::from([4.0, 6.0]))
            .on_press(Message::CloseShortcutsModal),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        horizontal_rule(1),
        scrollable(
            column![
                section(
                    "Arquivo e Exportação",
                    vec![
                        ("Ctrl + N", "Novo diagrama em branco"),
                        ("Ctrl + O", "Abrir projeto (.cad / .qet)"),
                        ("Ctrl + S", "Salvar projeto atual"),
                        ("Ctrl + Shift + S", "Salvar Como..."),
                        (
                            "Ctrl + Alt + S",
                            "Salvar Cópia (sem alterar o arquivo ativo)"
                        ),
                        ("Ctrl + P", "Imprimir folha / projeto em PDF"),
                        ("Ctrl + E", "Exportar (PNG, JPG, BMP, SVG, DXF, CAD, PDF)"),
                        ("Ctrl + W", "Fechar projeto atual"),
                    ],
                ),
                section(
                    "Edição e Transformação",
                    vec![
                        ("Ctrl + Z / Ctrl + Y", "Desfazer / Refazer"),
                        ("Ctrl + X / Ctrl + C", "Recortar / Copiar elementos"),
                        ("Ctrl + V / Ctrl + Shift + V", "Colar / Colar na Posição"),
                        (
                            "Ctrl + Alt + C",
                            "Copiar propriedades elétricas (Tag/Valor/Bornes)"
                        ),
                        ("Ctrl + D / Delete", "Duplicar / Excluir selecionados"),
                        (
                            "Ctrl + A / Ctrl + Shift + A",
                            "Selecionar Tudo / Desselecionar Tudo"
                        ),
                        ("Ctrl + G / Ctrl + Shift + G", "Agrupar / Desagrupar"),
                        ("R / Shift + R", "Girar 90 graus (Horário / Anti-horário)"),
                        ("M / F", "Espelhar Horizontal / Vertical"),
                        ("Ctrl + Shift + Enter", "Propriedades da Página e Cajetín"),
                    ],
                ),
                section(
                    "Projeto e Visualização",
                    vec![
                        ("Ctrl + T", "Adicionar Nova Folha"),
                        ("Ctrl + PgUp / PgDn", "Folha Anterior / Próxima Folha"),
                        ("Ctrl + Shift + Backspace", "Remover Folha Ativa"),
                        ("Ctrl + + / - / 0", "Aumentar Zoom / Reduzir / Restaurar"),
                        ("Esc", "Cancelar ação ou fechar menu/janela modal"),
                    ],
                ),
            ]
            .spacing(8)
        )
        .height(Length::Fixed(370.0)),
        horizontal_rule(1),
        row![
            horizontal_space(),
            button(text("Fechar").size(12).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }))
            .style(theme::primary)
            .padding(Padding::from([6.0, 18.0]))
            .on_press(Message::CloseShortcutsModal),
        ],
    ]
    .spacing(10)
    .width(560);

    let card = container(modal_content)
        .style(theme::dropdown_menu)
        .padding(Padding::from([16.0, 20.0]));

    wrap_modal(card, Message::CloseShortcutsModal)
}

/// Modal de aviso quando o utilizador tenta excluir a última página restante de um projeto.
pub fn view_close_last_page_modal<'a>(app: &'a CadeSimuApp) -> Element<'a, Message> {
    let t = tokens(&app.theme());
    let page_label = app
        .pages
        .first()
        .map(|p| p.name.as_str())
        .unwrap_or("Folha 1");

    let subtitle_msg = if app.is_modified {
        format!(
            "A folha \"{page_label}\" é a única página do projeto e há alterações não salvas. Excluir esta página fechará o projeto atual."
        )
    } else {
        format!(
            "A folha \"{page_label}\" é a única página restante do projeto. Excluir esta página fechará o projeto atual."
        )
    };

    let card = container(
        column![
            row![
                crate::icons::view_icon(crate::icons::Icon::Warning, 18, t.amber),
                text("Excluir Última Página (Fechar Projeto)")
                    .size(14.5)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }),
                horizontal_space(),
                button(crate::icons::view_icon(
                    crate::icons::Icon::Close,
                    14,
                    t.text2
                ))
                .style(theme::tool)
                .padding(Padding::from([3.0, 6.0]))
                .on_press(Message::CancelCloseLastPage),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            horizontal_rule(1),
            text(subtitle_msg).size(12.0).color(t.text).font(UI),
            container(
                text("Escolha se deseja salvar o projeto antes de encerrar ou descartar e iniciar uma área vazia.")
                    .size(11.0)
                    .color(t.text2)
                    .font(UI),
            )
            .style(theme::card)
            .padding(Padding::from([8.0, 10.0]))
            .width(Length::Fill),
            horizontal_rule(1),
            row![
                horizontal_space(),
                button(text("Cancelar").size(12.0).font(UI))
                    .style(theme::tool)
                    .padding(Padding::from([6.0, 12.0]))
                    .on_press(Message::CancelCloseLastPage),
                button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Close, 13, t.on_danger),
                        text("Fechar sem Salvar").size(12.0).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        }),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .style(theme::danger)
                .padding(Padding::from([6.0, 12.0]))
                .on_press(Message::ConfirmCloseLastPage),
                button(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Save, 13, t.on_primary),
                        text("Salvar e Fechar").size(12.0).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        }),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .style(theme::primary)
                .padding(Padding::from([6.0, 14.0]))
                .on_press(Message::SaveAndCloseLastPage),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(12)
        .width(520),
    )
    .style(theme::dropdown_menu)
    .padding(Padding::from([16.0, 18.0]));

    wrap_modal(card, Message::CancelCloseLastPage)
}
