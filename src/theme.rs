//! RustSimu - tema e estilos (iced 0.13.x)
//! Fonte: styles.css (variáveis em :root / [data-theme]) + DESIGN.md.
//! Uso: `Theme` = `AppTheme::theme()`; cada widget lê os tokens com `tokens(theme)`.
//! Nota: não compilado neste ambiente - confirme com `cargo check`.

use iced::theme::Palette;
use iced::widget::{button, container, text, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector};

// ---------- Constantes de layout (CSS: --r, --dock-w, ...) ----------
pub const R: f32 = 4.0;
pub const R_LG: f32 = 8.0;
pub const PILL: f32 = 999.0;
pub const SP_XS: f32 = 4.0;
pub const SP_SM: f32 = 8.0;
pub const SP_MD: f32 = 12.0;
pub const SP_LG: f32 = 16.0;
pub const DOCK_W: f32 = 288.0;
pub const MENU_H: f32 = 40.0;
pub const TOOL_H: f32 = 46.0;
pub const STATUS_H: f32 = 28.0;
pub const TOUCH: f32 = 36.0;

// ---------- Fontes (embutir com .font(include_bytes!(...))) ----------
pub const UI: Font = Font::with_name("Inter");
pub const MONO: Font = Font::with_name("JetBrains Mono");

// ---------- Modo de tema ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    /// Ciclo do botão de tema: sistema -> claro -> escuro.
    pub fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    pub fn resolve(self, system_is_dark: bool) -> AppTheme {
        match self {
            Self::Light => AppTheme::Light,
            Self::Dark => AppTheme::Dark,
            Self::System => {
                if system_is_dark {
                    AppTheme::Dark
                } else {
                    AppTheme::Light
                }
            }
        }
    }
}

/// Detecta a preferência do sistema operacional para modo escuro (Dark/Light).
pub fn detect_system_dark() -> bool {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let run = |prog: &str, args: &[&str]| -> String {
        let mut c = Command::new(prog);
        c.args(args);
        #[cfg(target_os = "windows")]
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: sem janela de consola no Windows
        c.output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
            .unwrap_or_default()
    };

    #[cfg(target_os = "windows")]
    {
        // 0x0 = tema de apps escuro no Windows Personalize
        return run(
            "reg",
            &[
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ],
        )
        .contains("0x0");
    }
    #[cfg(target_os = "linux")]
    {
        // Portal freedesktop: 1 = escuro, 2 = claro (KDE Plasma 6 e GNOME modernos)
        let p = run(
            "gdbus",
            &[
                "call",
                "--session",
                "--dest",
                "org.freedesktop.portal.Desktop",
                "--object-path",
                "/org/freedesktop/portal/desktop",
                "--method",
                "org.freedesktop.portal.Settings.Read",
                "org.freedesktop.appearance",
                "color-scheme",
            ],
        );
        if p.contains("uint32 1") {
            return true;
        }
        if p.contains("uint32 2") {
            return false;
        }
        // KDE Plasma via kdeglobals
        if run(
            "kreadconfig6",
            &[
                "--file",
                "kdeglobals",
                "--group",
                "General",
                "--key",
                "ColorScheme",
            ],
        )
        .contains("dark")
        {
            return true;
        }
        // Variável de ambiente GTK_THEME
        if let Ok(theme) = std::env::var("GTK_THEME") {
            if theme.to_lowercase().contains("dark") {
                return true;
            }
        }
        // GNOME desktop interface fallback
        return run(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "color-scheme"],
        )
        .contains("dark");
    }
    #[allow(unreachable_code)]
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    pub fn tokens(self) -> Tokens {
        match self {
            Self::Dark => Tokens::dark(),
            Self::Light => Tokens::light(),
        }
    }

    /// `Theme` do iced com a paleta do RustSimu (is_dark vem da luminância do fundo).
    pub fn theme(self) -> Theme {
        let t = self.tokens();
        let name = match self {
            Self::Dark => "RustSimu Dark",
            Self::Light => "RustSimu Light",
        };
        Theme::custom(
            name.to_string(),
            Palette {
                background: t.s1,
                text: t.text,
                primary: t.primary,
                success: t.primary,
                danger: t.danger,
            },
        )
    }
}

// ---------- Tokens (1:1 com as variáveis CSS) ----------
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    pub bg_canvas: Color,
    pub workspace: Color,
    pub grid: Color,
    pub s1: Color,
    pub s2: Color,
    pub s3: Color,
    pub border: Color,
    pub border_strong: Color,
    pub field_bg: Color,
    pub field_border: Color,
    pub text: Color,
    pub text2: Color,
    pub primary: Color,
    pub primary_solid: Color,
    pub on_primary: Color,
    pub primary_soft: Color,
    pub danger: Color,
    pub on_danger: Color,
    pub amber: Color,
    pub focus: Color,
    pub sel_bg: Color,
    pub sel_text: Color,
    pub sel_border: Color,
    pub wire_off: Color,
    pub w_power: Color,
    pub w_cmd: Color,
    pub w_dc: Color,
    pub w_pe: Color,
    pub ink: Color,
    pub sheet: Color,
    pub sheet_line: Color,
}

fn rgb(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
fn rgba(hex: u32, a: f32) -> Color {
    Color::from_rgba8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, a)
}

impl Tokens {
    pub fn dark() -> Self {
        Self {
            bg_canvas: rgb(0x0B0D11),
            workspace: rgb(0x0F1217),
            grid: rgb(0x1E232B),
            s1: rgb(0x161A22),
            s2: rgb(0x212733),
            s3: rgb(0x2C3444),
            border: rgb(0x262E3B),
            border_strong: rgb(0x3B4657),
            field_bg: rgb(0x0F131A),
            field_border: rgb(0x343E52),
            text: rgb(0xF0F3F8),
            text2: rgb(0x9AA5B8),
            primary: rgb(0x00E599),
            primary_solid: rgb(0x00E599),
            on_primary: rgb(0x0B0D11),
            primary_soft: rgba(0x00E599, 0.14),
            danger: rgb(0xFF3B30),
            on_danger: rgb(0xFFFFFF),
            amber: rgb(0xFFB300),
            focus: rgb(0x2979FF),
            sel_bg: rgb(0x0B3D2B),
            sel_text: rgb(0x6DFFBA),
            sel_border: rgb(0x00E599),
            wire_off: rgb(0x606B7D),
            w_power: rgb(0xF87171),
            w_cmd: rgb(0x38BDF8),
            w_dc: rgb(0xFBBF24),
            w_pe: rgb(0x4ADE80),
            ink: rgb(0xE1E2E9),
            sheet: rgb(0x0B0D11),
            sheet_line: rgb(0xC9D1DD),
        }
    }

    pub fn light() -> Self {
        Self {
            bg_canvas: rgb(0xFFFFFF),
            workspace: rgb(0xCBD3DE),
            grid: rgb(0xE2E6ED),
            s1: rgb(0xF4F6F9),
            s2: rgb(0xE9EDF2),
            s3: rgb(0xDDE3EB),
            border: rgb(0xD3DAE4),
            border_strong: rgb(0xB4BECC),
            field_bg: rgb(0xFFFFFF),
            field_border: rgb(0xB4BECC),
            text: rgb(0x0F141C),
            text2: rgb(0x5C6675),
            primary: rgb(0x009959),
            primary_solid: rgb(0x007A47),
            on_primary: rgb(0xFFFFFF),
            primary_soft: rgba(0x009959, 0.12),
            danger: rgb(0xD32F2F),
            on_danger: rgb(0xFFFFFF),
            amber: rgb(0xB87500),
            focus: rgb(0x2563EB),
            sel_bg: rgb(0xDDF3E8),
            sel_text: rgb(0x005A34),
            sel_border: rgb(0x009959),
            wire_off: rgb(0x4A5568),
            w_power: rgb(0xDC2626),
            w_cmd: rgb(0x2563EB),
            w_dc: rgb(0xD97706),
            w_pe: rgb(0x15803D),
            ink: rgb(0x0F141C),
            sheet: rgb(0xFFFFFF),
            sheet_line: rgb(0x1B2230),
        }
    }
}

/// Tokens a partir do `Theme` recebido pelos closures de estilo.
pub fn tokens(theme: &Theme) -> Tokens {
    if theme.extended_palette().is_dark {
        Tokens::dark()
    } else {
        Tokens::light()
    }
}

/// Tipo de fio no canvas (cores semânticas por tema).
#[derive(Debug, Clone, Copy)]
pub enum Wire {
    Power,
    Command,
    Dc,
    Earth,
    Off,
}

impl Tokens {
    pub fn wire(&self, w: Wire) -> Color {
        match w {
            Wire::Power => self.w_power,
            Wire::Command => self.w_cmd,
            Wire::Dc => self.w_dc,
            Wire::Earth => self.w_pe,
            Wire::Off => self.wire_off,
        }
    }
}

// ---------- Helpers ----------
fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: radius.into(),
    }
}

fn dim(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

fn bg(c: Color) -> Option<Background> {
    Some(Background::Color(c))
}

// ---------- Botões ----------
/// Botão de ferramenta da toolbar (.tbtn).
pub fn tool(theme: &Theme, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let s = button::Style {
        background: if hovered { bg(t.s3) } else { None },
        text_color: t.text,
        border: border(Color::TRANSPARENT, 0.0, R),
        shadow: Shadow::default(),
    };
    if matches!(status, button::Status::Disabled) {
        button::Style {
            text_color: dim(t.text, 0.4),
            ..s
        }
    } else {
        s
    }
}

/// Ferramenta ativa (aria-pressed="true").
pub fn tool_active(theme: &Theme, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let _ = status;
    button::Style {
        background: bg(t.primary_soft),
        text_color: t.primary,
        border: border(t.primary, 1.0, R),
        shadow: Shadow::default(),
    }
}

/// Item de menu (Arquivo, Editar, ...).
pub fn menu_item(theme: &Theme, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let on = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: if on { bg(t.s3) } else { None },
        text_color: t.text,
        border: border(Color::TRANSPARENT, 0.0, R),
        shadow: Shadow::default(),
    }
}

/// Play: verde; com brilho quando a simulação está a correr.
pub fn pill_play(running: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = tokens(theme);
        let pressed = matches!(status, button::Status::Pressed);
        let base = if pressed {
            dim(t.primary_solid, 0.85)
        } else if running {
            t.primary_solid
        } else {
            dim(t.primary_solid, 0.70)
        };
        button::Style {
            background: bg(base),
            text_color: t.on_primary,
            border: border(
                if running {
                    Color::WHITE
                } else {
                    Color::TRANSPARENT
                },
                if running { 1.5 } else { 0.0 },
                PILL,
            ),
            shadow: if running {
                Shadow {
                    color: rgba(0x00E599, 0.65),
                    offset: Vector::new(0.0, 0.0),
                    blur_radius: 14.0,
                }
            } else {
                Shadow::default()
            },
        }
    }
}

/// Stop: vermelho; esmaecido quando parado.
pub fn pill_stop(running: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, _| {
        let t = tokens(theme);
        let a = if running { 1.0 } else { 0.55 };
        button::Style {
            background: bg(dim(t.danger, a)),
            text_color: dim(t.on_danger, a.max(0.8)),
            border: border(Color::TRANSPARENT, 0.0, PILL),
            shadow: Shadow::default(),
        }
    }
}

/// Pausa / passo / reset (.pill.ghost).
pub fn pill_ghost(theme: &Theme, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let on = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: bg(if on { t.border_strong } else { t.s3 }),
        text_color: t.text,
        border: border(t.border_strong, 1.0, PILL),
        shadow: Shadow::default(),
    }
}

/// Ação primária de formulário ("Aplicar propriedades").
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let t = tokens(theme);
    let c = match status {
        button::Status::Hovered => dim(t.primary_solid, 0.92),
        button::Status::Pressed => dim(t.primary_solid, 0.85),
        _ => t.primary_solid,
    };
    button::Style {
        background: bg(c),
        text_color: t.on_primary,
        border: border(Color::TRANSPARENT, 0.0, R),
        shadow: Shadow::default(),
    }
}

/// Linha da árvore de coleções (.t-item / .t-cat).
pub fn tree_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = tokens(theme);
        if selected {
            return button::Style {
                background: bg(t.sel_bg),
                text_color: t.sel_text,
                border: border(t.sel_border, 1.0, R),
                shadow: Shadow::default(),
            };
        }
        let on = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if on { bg(t.s3) } else { None },
            text_color: t.text,
            border: border(Color::TRANSPARENT, 1.0, R),
            shadow: Shadow::default(),
        }
    }
}

/// Aba de propriedades; o sublinhado vem de um container de 2 px por baixo.
pub fn tab(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = tokens(theme);
        let on = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if on { bg(t.s3) } else { None },
            text_color: if selected { t.text } else { t.text2 },
            border: border(Color::TRANSPARENT, 0.0, R),
            shadow: Shadow::default(),
        }
    }
}

/// Barra de abas: item da navegação inferior (mobile/estreito).
pub fn nav_item(current: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, _| {
        let t = tokens(theme);
        button::Style {
            background: if current { bg(t.primary_soft) } else { None },
            text_color: if current { t.primary } else { t.text2 },
            border: border(Color::TRANSPARENT, 0.0, 0.0),
            shadow: Shadow::default(),
        }
    }
}

// ---------- Containers ----------
fn boxed(bg_c: Color, bd: Color, radius: f32, theme: &Theme) -> container::Style {
    let t = tokens(theme);
    container::Style {
        text_color: Some(t.text),
        background: bg(bg_c),
        border: border(bd, 1.0, radius),
        shadow: Shadow::default(),
    }
}

/// Menubar, toolbar, docks, statusbar (superfície 1 com filete).
pub fn bar(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    boxed(t.s1, t.border, 0.0, theme)
}

/// Superfície 2: cartões, cápsula de transporte, flyouts.
pub fn card(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    boxed(t.s2, t.border, R_LG, theme)
}

/// Menu suspenso dropdown flutuante (superfície 2 elevada com borda e sombra suave).
pub fn dropdown_menu(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    let shadow_color = if theme.extended_palette().is_dark {
        Color::from_rgba(0.0, 0.0, 0.0, 0.55)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.22)
    };
    container::Style {
        text_color: Some(t.text),
        background: bg(t.s2),
        border: border(t.border_strong, 1.0, R_LG),
        shadow: Shadow {
            color: shadow_color,
            offset: Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
    }
}

/// Tooltip flutuante com fundo elevado seguindo o tema, borda e cantos arredondados.
pub fn tooltip(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    let shadow_color = if theme.extended_palette().is_dark {
        Color::from_rgba(0.0, 0.0, 0.0, 0.55)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.18)
    };
    container::Style {
        text_color: Some(t.text),
        background: bg(t.s2),
        border: border(t.border_strong, 1.0, R),
        shadow: Shadow {
            color: shadow_color,
            offset: Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
    }
}

/// Separador / divisor de colunas redimensionáveis.
pub fn splitter(_theme: &Theme) -> container::Style {
    container::Style::default()
}

/// Grupo da toolbar (.tgroup).
pub fn tool_group(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    boxed(t.s1, t.border, R_LG, theme)
}

/// Cápsula de simulação (.transport).
pub fn transport(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    boxed(t.s2, t.border, PILL, theme)
}

/// Fundo do espaço de trabalho à volta da folha.
pub fn workspace(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    container::Style {
        background: bg(t.workspace),
        ..boxed(t.workspace, t.workspace, 0.0, theme)
    }
}

/// Caixa de pré-visualização do símbolo (128 px).
pub fn preview_box(theme: &Theme) -> container::Style {
    let t = tokens(theme);
    boxed(t.bg_canvas, t.border, R, theme)
}

/// Selo "Modo: Edição" / "Simulação".
pub fn mode_badge(sim: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = tokens(theme);
        let c = if sim { t.amber } else { t.primary };
        container::Style {
            text_color: Some(t.text),
            background: bg(if sim {
                dim(t.amber, 0.14)
            } else {
                t.primary_soft
            }),
            border: border(c, 1.0, R),
            shadow: Shadow::default(),
        }
    }
}

/// Sublinhado da aba ativa.
pub fn tab_underline(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = tokens(theme);
        container::Style {
            background: bg(if active {
                t.primary
            } else {
                Color::TRANSPARENT
            }),
            ..Default::default()
        }
    }
}

/// Filete horizontal/vertical de 1 px.
pub fn divider(theme: &Theme) -> container::Style {
    container::Style {
        background: bg(tokens(theme).border),
        ..Default::default()
    }
}

// ---------- Campos de texto ----------
pub fn field(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let t = tokens(theme);
    let c = match status {
        text_input::Status::Focused => t.focus,
        text_input::Status::Hovered => t.border_strong,
        _ => t.field_border,
    };
    text_input::Style {
        background: Background::Color(t.field_bg),
        border: border(c, 1.0, R),
        icon: t.text2,
        placeholder: t.text2,
        value: if matches!(status, text_input::Status::Disabled) {
            t.text2
        } else {
            t.text
        },
        selection: dim(t.focus, 0.35),
    }
}

// ---------- Texto ----------
/// Rótulos técnicos em maiúsculas (usar com MONO, size 10).
pub fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(tokens(theme).text2),
    }
}

pub fn accent(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(tokens(theme).primary),
    }
}

pub fn warning(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(tokens(theme).amber),
    }
}
