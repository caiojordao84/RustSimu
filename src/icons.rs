//! Ícones vetoriais SVG nativos para a interface do RustSimu.
//!
//! Substitui completamente caracteres emoji/Unicode que causavam "tofu"
//! (retângulos de caractere não encontrado) em sistemas Linux sem fontes de emoji coloridas.
//! Todas as formas SVG são compatíveis 100% com o layout de `Layout RustSimu HTML/index.html`.

use iced::widget::svg;
use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    New,
    Open,
    Save,
    SaveAs,
    Close,
    Print,
    ExportPdf,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    RotateCw,
    RotateCcw,
    MirrorH,
    MirrorV,
    Palette,
    Select,
    Pan,
    WireL,
    WireN,
    WirePe,
    WireDc,
    Junction,
    DeleteWire,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    FitToPage,
    Grid,
    Magnet,
    Text,
    Theme,
    Play,
    Pause,
    Stop,
    Step,
    Reset,
    ChevronLeft,
    ChevronRight,
    AddBefore,
    AddAfter,
    Search,
    Pencil,
    Folder,
    Bolt,
    Info,
    Warning,
    Check,
    Group,
    Ungroup,
}

impl Icon {
    pub const fn bytes(self) -> &'static [u8] {
        match self {
            Icon::New => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/>"#,
                r#"<path d="M14 3v5h5"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Open => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Save => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M5 3h11l3 3v13a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/>"#,
                r#"<path d="M7 3v6h8V3M7 21v-7h10v7"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::SaveAs => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M14 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6"/>"#,
                r#"<path d="M14 3l5 5v4"/>"#,
                r#"<path d="M7 3v6h7V3M7 21v-5h4"/>"#,
                r#"<path d="M19 15v6M16 18h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Close => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M18 6L6 18M6 6l12 12"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Print => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M6 9V2h12v7M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>"#,
                r#"<path d="M6 14h12v8H6z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::ExportPdf => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>"#,
                r#"<path d="M14 2v6h6M12 18v-6M9 15l3 3 3-3"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Undo => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M9 14L4 9l5-5"/>"#,
                r#"<path d="M4 9h10a6 6 0 0 1 0 12h-3"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Redo => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m15 14 5-5-5-5"/>"#,
                r#"<path d="M20 9H10a6 6 0 0 0 0 12h3"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Cut => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="6" cy="6" r="3"/>"#,
                r#"<circle cx="6" cy="18" r="3"/>"#,
                r#"<path d="M20 4L8.1 15.9M14.5 14.5L20 20M8.1 8.1L12 12"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Copy => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<rect x="9" y="9" width="13" height="13" rx="2"/>"#,
                r#"<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Paste => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>"#,
                r#"<rect x="8" y="2" width="8" height="4" rx="1"/>"#,
                r#"<path d="M12 11v6M9 14l3 3 3-3"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Duplicate => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<rect x="8" y="8" width="13" height="13" rx="2"/>"#,
                r#"<path d="M4 16H3a1 1 0 0 1-1-1V4a2 2 0 0 1 2-2h11a1 1 0 0 1 1 1v1"/>"#,
                r#"<path d="M14.5 11.5v6M11.5 14.5h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Delete => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"#,
                r#"<path d="M10 11v6M14 11v6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::RotateCw => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M21 12a9 9 0 1 1-3-6.7L21 8"/>"#,
                r#"<path d="M21 3v5h-5"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::RotateCcw => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/>"#,
                r#"<path d="M3 3v5h5"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::MirrorH => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m8 7-5 5 5 5M16 7l5 5-5 5M3 12h18"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::MirrorV => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m7 8 5-5 5 5M7 16l5 5 5-5M12 3v18"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Palette => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/>"#,
                r#"<circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/>"#,
                r#"<circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/>"#,
                r#"<circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/>"#,
                r#"<path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.9 0 1.5-.7 1.5-1.5 0-.4-.2-.8-.4-1.1-.3-.4-.4-.8-.4-1.4 0-1.1.9-2 2-2h1.8c3.6 0 6.5-2.9 6.5-6.5C23 6 18 2 12 2z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Select => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m5 3 14 8-6 2-2 6z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Pan => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M18 11V6a2 2 0 0 0-4 0v5M14 10V4a2 2 0 0 0-4 0v6M10 10.5V6a2 2 0 0 0-4 0v8"/>"#,
                r#"<path d="M6 14v2a6 6 0 0 0 12 0v-5a2 2 0 0 0-4 0"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::WireL => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::WireN => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="4" cy="12" r="2" fill="currentColor"/>"#,
                r#"<circle cx="20" cy="12" r="2" fill="currentColor"/>"#,
                r#"<path d="M6 12h12"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::WirePe => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M12 3v10M5 13h14M8 17h8M10 21h4"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::WireDc => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="4" cy="12" r="2" fill="currentColor"/>"#,
                r#"<circle cx="20" cy="12" r="2" fill="currentColor"/>"#,
                r#"<path d="M6 12h12M12 6v12"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Junction => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="12" cy="12" r="5" fill="currentColor"/>"#,
                r#"<path d="M12 2v5M12 17v5M2 12h5M17 12h5"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::DeleteWire => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M4 12h16M15 7l-6 10M9 7l6 10"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::ZoomIn => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="11" cy="11" r="7"/>"#,
                r#"<path d="m20 20-4-4M11 8v6M8 11h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::ZoomOut => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="11" cy="11" r="7"/>"#,
                r#"<path d="m20 20-4-4M8 11h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::ZoomReset => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="11" cy="11" r="7"/>"#,
                r#"<path d="m20 20-4-4M10 9v4M12 9v4"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::FitToPage => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Grid => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M4 4h16v16H4zM4 9.3h16M4 14.6h16M9.3 4v16M14.6 4v16"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Magnet => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M4 4v7a8 8 0 0 0 16 0V4h-4v7a4 4 0 0 1-8 0V4H4z"/>"#,
                r#"<path d="M4 8h4M16 8h4"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Text => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M5 6V4h14v2M12 4v16M9 20h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Theme => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="12" cy="12" r="9"/>"#,
                r#"<path d="M12 3v18"/>"#,
                r#"<path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Play => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">"#,
                r#"<path d="M7 4v16l13-8z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Pause => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M8 5v14M16 5v14"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Stop => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">"#,
                r#"<rect x="6" y="6" width="12" height="12" rx="1"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Step => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M6 5v14l10-7z" fill="currentColor"/>"#,
                r#"<path d="M19 5v14"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Reset => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M4 12a8 8 0 1 0 3-6.2M4 4v5h5"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::ChevronLeft => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m15 6-6 6 6 6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::ChevronRight => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m9 6 6 6-6 6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::AddBefore => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m11 6-6 6 6 6M17 9v6M14 12h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::AddAfter => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m13 6 6 6-6 6M7 9v6M4 12h6"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Search => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="11" cy="11" r="7"/>"#,
                r#"<path d="m20 20-4-4"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Pencil => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M17 3a2.8 2.8 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Folder => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Bolt => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Info => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<circle cx="12" cy="12" r="10"/>"#,
                r#"<line x1="12" y1="16" x2="12" y2="12"/>"#,
                r#"<line x1="12" y1="8" x2="12.01" y2="8"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Warning => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/>"#,
                r#"<line x1="12" y1="9" x2="12" y2="13"/>"#,
                r#"<line x1="12" y1="17" x2="12.01" y2="17"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Check => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<polyline points="20 6 9 17 4 12"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Group => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M3 7V5a2 2 0 0 1 2-2h2"/>"#,
                r#"<path d="M17 3h2a2 2 0 0 1 2 2v2"/>"#,
                r#"<path d="M21 17v2a2 2 0 0 1-2 2h-2"/>"#,
                r#"<path d="M7 21H5a2 2 0 0 1-2-2v-2"/>"#,
                r#"<rect x="7" y="7" width="6" height="6" rx="1"/>"#,
                r#"<rect x="11" y="11" width="6" height="6" rx="1"/>"#,
                r#"</svg>"#
            ).as_bytes(),

            Icon::Ungroup => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
                r#"<path d="M5 9V5h4"/>"#,
                r#"<path d="M5 15v4h4"/>"#,
                r#"<path d="M19 9V5h-4"/>"#,
                r#"<path d="M19 15v4h-4"/>"#,
                r#"<rect x="6" y="9" width="5" height="5" rx="1"/>"#,
                r#"<rect x="13" y="9" width="5" height="5" rx="1"/>"#,
                r#"</svg>"#
            ).as_bytes(),
        }
    }
}

/// Cria um elemento visual SVG vetorial em tamanho e cor personalizados.
pub fn view_icon<'a>(icon: Icon, size: u16, color: Color) -> iced::widget::Svg<'a, iced::Theme> {
    let handle = svg::Handle::from_memory(icon.bytes());
    svg(handle)
        .width(size)
        .height(size)
        .style(move |_theme, _status| svg::Style { color: Some(color) })
}
