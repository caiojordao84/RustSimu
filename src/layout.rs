//! RustSimu — layout responsivo com painéis laterais recolhíveis e área de trabalho expansível.
//! Colunas: Coleções (DOCK_W ou 28px recolhido) | Folha (Fill) | Propriedades (DOCK_W ou 28px recolhido).

use iced::widget::pane_grid::ResizeEvent;
use iced::widget::{container, row};
use iced::{Element, Length};

use crate::theme::{self, DOCK_W};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Collections,
    Sheet,
    Properties,
}

impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Kind::Collections => "Coleções / Projetos",
            Kind::Sheet => "Folha",
            Kind::Properties => "Propriedades do Elemento",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Resized(ResizeEvent),
    /// Esconde/mostra os docks (maximiza a folha). Atalho: F11 ou Tab.
    #[allow(dead_code)]
    ToggleFocus,
}

pub struct Layout {
    pub left_width: f32,
    pub right_width: f32,
}

impl Layout {
    /// Inicializa o layout com as larguras padrão `DOCK_W` para os painéis laterais.
    pub fn new(_window_width: f32) -> Self {
        Self {
            left_width: DOCK_W,
            right_width: DOCK_W,
        }
    }

    pub fn update(&mut self, _msg: Msg) {}

    /// Constrói a visualização em 3 colunas:
    /// - Painel esquerdo recolhe para 28px quando `left_collapsed = true`
    /// - Painel direito recolhe para 28px quando `right_collapsed = true`
    /// - Painel central (Folha) consome todo o espaço restante (`Length::Fill`)
    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        left_collapsed: bool,
        right_collapsed: bool,
        _wrap: impl Fn(Msg) -> M + Copy + 'a,
        body: impl Fn(Kind) -> Element<'a, M> + 'a,
    ) -> Element<'a, M> {
        let left_w = if left_collapsed { 28.0 } else { self.left_width };
        let right_w = if right_collapsed { 28.0 } else { self.right_width };

        let left_panel = container(body(Kind::Collections))
            .width(Length::Fixed(left_w))
            .height(Length::Fill)
            .style(theme::bar);

        let center_panel = container(body(Kind::Sheet))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::workspace);

        let right_panel = container(body(Kind::Properties))
            .width(Length::Fixed(right_w))
            .height(Length::Fill)
            .style(theme::bar);

        row![left_panel, center_panel, right_panel]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
