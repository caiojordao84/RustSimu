//! RustSimu - layout responsivo com painéis laterais recolhíveis e área de trabalho expansível.
//! Colunas: Coleções (DOCK_W ou 28px recolhido) | Folha (Fill) | Propriedades (DOCK_W ou 28px recolhido).

use iced::widget::{container, mouse_area, Row, Space};
use iced::{mouse, Element, Length};

use crate::theme::{self, DOCK_W};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Collections,
    Sheet,
    Properties,
}

impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Kind::Collections => "Elementos e Projetos",
            Kind::Sheet => "Folha",
            Kind::Properties => "Propriedades do Elemento",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    DragStart(Side),
    DragMove(f32),
    DragEnd,
    WindowResized(f32),
}

pub struct Layout {
    pub left_width: f32,
    pub right_width: f32,
    pub window_w: f32,
    pub drag: Option<Side>,
}

impl Layout {
    /// Inicializa o layout com as larguras padrão `DOCK_W` para os painéis laterais e largura da janela.
    pub fn new(window_width: f32) -> Self {
        Self {
            left_width: DOCK_W,
            right_width: DOCK_W,
            window_w: window_width,
            drag: None,
        }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::DragStart(side) => {
                self.drag = Some(side);
            }
            Msg::DragMove(x) => match self.drag {
                Some(Side::Left) => {
                    self.left_width = x.clamp(160.0, 480.0);
                }
                Some(Side::Right) => {
                    self.right_width = (self.window_w - x).clamp(160.0, 480.0);
                }
                None => {}
            },
            Msg::DragEnd => {
                self.drag = None;
            }
            Msg::WindowResized(w) => {
                self.window_w = w;
            }
        }
    }

    /// Constrói a visualização em 3 colunas:
    /// - Painel esquerdo recolhe para 28px quando `left_collapsed = true`
    /// - Painel direito recolhe para 28px quando `right_collapsed = true`
    /// - Separadores de 6px interativos com cursor horizontal quando expandidos
    /// - Painel central (Folha) consome todo o espaço restante (`Length::Fill`)
    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        left_collapsed: bool,
        right_collapsed: bool,
        wrap: impl Fn(Msg) -> M + Copy + 'a,
        body: impl Fn(Kind) -> Element<'a, M> + 'a,
    ) -> Element<'a, M> {
        let left_w = if left_collapsed {
            28.0
        } else {
            self.left_width
        };
        let right_w = if right_collapsed {
            28.0
        } else {
            self.right_width
        };

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

        let mut row_children: Vec<Element<'a, M>> = Vec::with_capacity(5);
        row_children.push(left_panel.into());

        if !left_collapsed {
            let left_splitter = mouse_area(
                container(Space::new(Length::Fixed(6.0), Length::Fill))
                    .width(Length::Fixed(6.0))
                    .height(Length::Fill)
                    .style(theme::splitter),
            )
            .interaction(mouse::Interaction::ResizingHorizontally)
            .on_press(wrap(Msg::DragStart(Side::Left)));

            row_children.push(left_splitter.into());
        }

        row_children.push(center_panel.into());

        if !right_collapsed {
            let right_splitter = mouse_area(
                container(Space::new(Length::Fixed(6.0), Length::Fill))
                    .width(Length::Fixed(6.0))
                    .height(Length::Fill)
                    .style(theme::splitter),
            )
            .interaction(mouse::Interaction::ResizingHorizontally)
            .on_press(wrap(Msg::DragStart(Side::Right)));

            row_children.push(right_splitter.into());
        }

        row_children.push(right_panel.into());

        Row::with_children(row_children)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_drag_and_clamp() {
        let mut layout = Layout::new(1200.0);
        assert_eq!(layout.left_width, DOCK_W);
        assert_eq!(layout.right_width, DOCK_W);

        // Arraste do painel esquerdo
        layout.update(Msg::DragStart(Side::Left));
        layout.update(Msg::DragMove(100.0)); // Deve limitar no mínimo (160.0)
        assert_eq!(layout.left_width, 160.0);

        layout.update(Msg::DragMove(300.0));
        assert_eq!(layout.left_width, 300.0);

        layout.update(Msg::DragMove(600.0)); // Deve limitar no máximo (480.0)
        assert_eq!(layout.left_width, 480.0);
        layout.update(Msg::DragEnd);
        assert!(layout.drag.is_none());

        // Arraste do painel direito
        layout.update(Msg::DragStart(Side::Right));
        layout.update(Msg::DragMove(1150.0)); // 1200 - 1150 = 50 -> clamp 160.0
        assert_eq!(layout.right_width, 160.0);
        layout.update(Msg::DragEnd);

        // Redimensionamento de janela
        layout.update(Msg::WindowResized(1600.0));
        assert_eq!(layout.window_w, 1600.0);
    }
}
