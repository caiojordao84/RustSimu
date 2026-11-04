//! RustSimu — layout com pane_grid redimensionável (iced 0.13.x)
//! Colunas: Coleções | Folha | Propriedades (larguras iniciais = DOCK_W do CSS).
//! Depende do theme.rs já entregue (tokens, bar, workspace, tool, UI, DOCK_W).

use iced::widget::pane_grid::{self, Axis, Configuration, Content, Pane, ResizeEvent};
use iced::widget::{container, pane_grid as grid};
use iced::{Background, Border, Element, Length, Theme};

use crate::theme::{self, tokens, DOCK_W};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Collections,
    Sheet,
    Properties,
}

impl Kind {
    #[allow(dead_code)]
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
    /// Esconde/mostra os docks (maximiza a folha). Atalho sugerido: F11 ou Tab.
    #[allow(dead_code)]
    ToggleFocus,
}

pub struct Layout {
    panes: pane_grid::State<Kind>,
    sheet: Pane,
}

impl Layout {
    /// `window_width` define as razões iniciais para os docks começarem com DOCK_W px.
    pub fn new(window_width: f32) -> Self {
        let w = window_width.max(3.0 * DOCK_W);
        let left = DOCK_W / w;
        // A 2.ª divisão mede-se sobre o espaço que sobra à direita da 1.ª:
        // folha = (w - 2*DOCK_W), resto = (w - DOCK_W).
        let sheet_share = (w - 2.0 * DOCK_W) / (w - DOCK_W);

        let config = Configuration::Split {
            axis: Axis::Vertical,
            ratio: left,
            a: Box::new(Configuration::Pane(Kind::Collections)),
            b: Box::new(Configuration::Split {
                axis: Axis::Vertical,
                ratio: sheet_share,
                a: Box::new(Configuration::Pane(Kind::Sheet)),
                b: Box::new(Configuration::Pane(Kind::Properties)),
            }),
        };
        let panes = pane_grid::State::with_configuration(config);
        let sheet = panes
            .iter()
            .find(|(_, k)| **k == Kind::Sheet)
            .map(|(p, _)| *p)
            .expect("a folha existe na configuração inicial");
        Self { panes, sheet }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Resized(ev) => self.panes.resize(ev.split, ev.ratio),
            Msg::ToggleFocus => {
                if self.panes.maximized().is_some() {
                    self.panes.restore();
                } else {
                    self.panes.maximize(self.sheet);
                }
            }
        }
    }

    /// `wrap` converte `Msg` na mensagem da app; `body` devolve o conteúdo de cada painel.
    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        wrap: impl Fn(Msg) -> M + Copy + 'a,
        body: impl Fn(Kind) -> Element<'a, M> + 'a,
    ) -> Element<'a, M> {
        grid(&self.panes, move |_pane, kind, _maximized| {
            let inner: Element<'a, M> = body(*kind);
            let style: fn(&Theme) -> container::Style = match kind {
                Kind::Sheet => theme::workspace,
                _ => theme::bar,
            };
            Content::new(inner).style(style)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(1)
        .on_resize(6, move |ev| wrap(Msg::Resized(ev)))
        .style(dock_grid)
        .into()
    }
}

/// Divisórias e destaque de arrasto do pane_grid (acentos do tema).
fn dock_grid(theme: &Theme) -> pane_grid::Style {
    let t = tokens(theme);
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Background::Color(t.primary_soft),
            border: Border { color: t.primary, width: 1.0, radius: 0.0.into() },
        },
        picked_split: pane_grid::Line { color: t.primary, width: 2.0 },
        hovered_split: pane_grid::Line { color: t.primary, width: 2.0 },
    }
}
