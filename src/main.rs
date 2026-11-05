pub use cade_simu_rs::{
    canvas_view, icons, io, layout, model, qet, sim, svg_symbol, symbols, theme,
};

use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, mouse_area, pick_list,
    row, scrollable, stack, svg, text, text_input, tooltip, vertical_space, Space, Stack,
};
use iced::{Alignment, Border, Color, Element, Font, Length, Padding, Subscription, Theme, Vector};

use crate::canvas_view::{CanvasMessage, EditorTool, SchematicCanvas};
use crate::io::{build_dol_starter_preset, load_document, save_document};
use crate::layout::{Kind, Layout, Msg as LayoutMsg};
use crate::model::{SchematicDocument, SchematicItem};
use crate::sim::{press_component_on_sim, release_component_on_sim, step_simulation};
use crate::symbols::{
    classify_apparatus_role, detect_contact_kind, detect_current_sequence, extract_tag_prefix,
    find_duplicate_apparatus, find_occupied_sequences_for_tag, find_parent_apparatus, find_symbol,
    list_available_parent_tags, next_sequential_tag_for_role, symbol_catalog, SymbolCategory,
};
use crate::theme::{tokens, ThemeMode, MENU_H, MONO, R, STATUS_H, TOOL_H, UI};

/// Opção do menu dropdown sequencial de contatos (IEC 60947).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinSequenceOption {
    pub digit: u8,
    pub in_use: bool,
    pub is_suggested: bool,
    pub label: String,
}

impl std::fmt::Display for PinSequenceOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// Formato de arquivo para salvamento na janela do sistema ou seletor rápido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFormat {
    Cad,
    Json,
    Toml,
    Qet,
}

impl SaveFormat {
    pub const ALL: [SaveFormat; 4] = [
        SaveFormat::Cad,
        SaveFormat::Json,
        SaveFormat::Toml,
        SaveFormat::Qet,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            SaveFormat::Cad => "cad",
            SaveFormat::Json => "json",
            SaveFormat::Toml => "toml",
            SaveFormat::Qet => "qet",
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            SaveFormat::Cad => "CADe_SIMU Original (*.cad)",
            SaveFormat::Json => "Projeto JSON (*.json)",
            SaveFormat::Toml => "Projeto TOML (*.toml)",
            SaveFormat::Qet => "Projeto QElectroTech (*.qet)",
        }
    }

    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "cad" => Some(SaveFormat::Cad),
            "json" => Some(SaveFormat::Json),
            "toml" => Some(SaveFormat::Toml),
            "qet" => Some(SaveFormat::Qet),
            _ => None,
        }
    }
}

impl std::fmt::Display for SaveFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveFormat::Cad => write!(f, "Formato .CAD (Original)"),
            SaveFormat::Json => write!(f, "Formato .JSON"),
            SaveFormat::Toml => write!(f, "Formato .TOML"),
            SaveFormat::Qet => write!(f, "Formato .QET (QElectroTech)"),
        }
    }
}

pub fn main() -> iced::Result {
    iced::application("RustSimu", CadeSimuApp::update, CadeSimuApp::view)
        .subscription(CadeSimuApp::subscription)
        .theme(CadeSimuApp::theme)
        .window_size((1420.0, 880.0))
        .run_with(CadeSimuApp::new)
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(CanvasMessage),
    SelectCategory(SymbolCategory),
    SelectTool(EditorTool),
    ToggleSimulation,
    StopSimulation,
    SimTick,
    ToggleGrid(bool),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    RotateSelectedOrTool,
    MirrorSelectedOrTool,
    DeleteSelected,
    NewDocument,
    LoadPresetDol,
    OpenFileDialog,
    FileOpened(Option<PathBuf>),
    #[allow(dead_code)]
    SelectSaveFormat(SaveFormat),
    SaveFileDialog,
    FileSaved(Option<PathBuf>),
    FilePathChanged(String),
    LoadFromPath,
    SaveAsCad,
    SaveAsJson,
    SaveAsToml,
    EditTag(String),
    EditValue(String),
    EditPin(usize, String),
    ToggleShowTag(bool),
    ToggleShowValue(bool),
    EditTitleBlockTitle(String),
    EditTitleBlockSubtitle(String),
    EditTitleBlockAuthor(String),
    PrevFolio,
    NextFolio,
    SelectFolio(usize),
    CycleTheme,
    LayoutMsg(LayoutMsg),
    CollectionSearchChanged(String),
    SelectCollectionSymbol(i32),
    StartSymbolDrag(i32),
    InsertSelectedCollectionSymbol,
    CancelAction,
    GlobalMouseReleased,
    SelectPropTab(usize),
    CloseDocument,
    SelectPage(usize),
    AddPageBefore(usize),
    AddPageAfter(usize),
    DeletePage(usize),
    MovePage(usize, usize),
    StartEditingPageName(usize),
    UpdateEditingPageName(String),
    CommitEditingPageName,
    TogglePageContextMenu(usize),
    ClosePageContextMenu,
    ToggleMenu(Option<ActiveMenu>),
    CloseMenu,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    RotateClockwise,
    RotateCounterClockwise,
    MirrorHorizontal,
    MirrorVertical,
    AlignToGrid,
    FitToPage,
    ToggleShowTexts,
    ToggleAutoConnect,
    ToggleAutoDisconnect,
    SetGridDensity(crate::model::GridDensity),
    SaveAll,
    PrintDocument,
    ShowAbout,
    CloseAbout,
    ToggleLeftDockCollapse,
    ToggleRightDockCollapse,
    SelectLeftDockTab(usize),
    RecentProjectSelected(String),
    DeleteRecentProject(String),
    SetLampColor(i32),
    SetPulseDuration(String),
    ApplyIecNumberingPreset(&'static str),
    SelectPinSequence(u8),
    PasteAtOriginalPos,
    SelectAll,
    DeselectAll,
    InvertSelection,
    InsertLastElement,
    CopyElementInfo,
    PasteElementInfo,
    ZoomContent,
    ExportDialog,
    ToggleSnap,
    ColorElement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveMenu {
    Arquivo,
    Editar,
    Projeto,
    Exibicao,
    Configuracoes,
    Ajuda,
}

fn app_tooltip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    tip: impl Into<String>,
    position: tooltip::Position,
) -> Element<'a, M> {
    tooltip(content, text(tip.into()).size(12).font(UI), position)
        .style(theme::tooltip)
        .padding(6.0)
        .gap(4.0)
        .into()
}

pub struct CadeSimuApp {
    pub doc: SchematicDocument,
    pub pages: Vec<crate::model::ProjectPage>,
    pub active_page_index: usize,
    pub is_modified: bool,
    pub editing_page_name: Option<String>,
    pub page_context_menu: Option<usize>,
    pub qet_project: Option<crate::qet::QetProject>,
    pub active_category: SymbolCategory,
    pub tool: EditorTool,
    pub selected_id: Option<i32>,
    pub sim_running: bool,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub grid_density: crate::model::GridDensity,
    pub show_texts: bool,
    pub auto_connect: bool,
    pub auto_disconnect: bool,
    pub zoom: f32,
    pub pan: Vector,
    pub placement_orientation: i32,
    pub cursor_world: (i32, i32),
    pub file_path: String,
    pub current_file: Option<PathBuf>,
    pub recent_files: Vec<String>,
    pub last_dir: PathBuf,
    pub save_format: SaveFormat,
    pub status_msg: String,
    pub theme_mode: ThemeMode,
    pub system_is_dark: bool,
    pub layout: Layout,
    pub collection_search: String,
    pub selected_collection_sym: Option<i32>,
    pub dragging_symbol: Option<i32>,
    pub last_collection_click: Option<(i32, std::time::Instant)>,
    pub last_placed_sym: Option<i32>,
    pub prop_tab: usize,
    pub left_dock_tab: usize,
    pub left_dock_collapsed: bool,
    pub right_dock_collapsed: bool,
    pub open_menu: Option<ActiveMenu>,
    pub show_about_modal: bool,
    pub undo_stack: Vec<SchematicDocument>,
    pub redo_stack: Vec<SchematicDocument>,
    pub drag_undo_snapshot: Option<SchematicDocument>,
    pub clipboard: Option<SchematicItem>,
    pub copied_info: Option<(String, String, [String; 9])>,
}

fn load_recent_files(dir: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if matches!(ext, "cad" | "qet" | "json" | "toml") {
                    files.push(p.to_string_lossy().to_string());
                }
            }
        }
    }
    files.sort();
    files
}

fn default_projects_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p_exe = exe_dir.join("Projetos");
            if p_exe.is_dir() {
                return p_exe;
            }
        }
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let in_manifest = manifest_dir.join("Projetos");
    if in_manifest.is_dir() {
        return in_manifest;
    }
    let manifest_parent = manifest_dir
        .parent()
        .map(|p| p.join("Projetos"))
        .unwrap_or_else(|| PathBuf::from("Projetos"));
    if manifest_parent.is_dir() {
        return manifest_parent;
    }
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("Projetos");
        if candidate.is_dir() {
            return candidate;
        }
        return cwd;
    }
    PathBuf::from(".")
}

impl CadeSimuApp {
    fn new() -> (Self, iced::Task<Message>) {
        let doc = build_dol_starter_preset();
        let last_dir = default_projects_dir();
        let default_file = last_dir.join("partida_direta.cad");
        let layout = Layout::new(1420.0);
        let pages = vec![crate::model::ProjectPage::new(
            1,
            "Potência e Comando",
            doc.clone(),
        )];
        let recent_files = load_recent_files(&last_dir);
        (
            Self {
                doc,
                pages,
                active_page_index: 0,
                is_modified: false,
                editing_page_name: None,
                page_context_menu: None,
                qet_project: None,
                active_category: SymbolCategory::BreakersRelays,
                tool: EditorTool::Select,
                selected_id: None,
                sim_running: false,
                show_grid: true,
                snap_to_grid: true,
                grid_density: crate::model::GridDensity::Medium,
                show_texts: true,
                auto_connect: true,
                auto_disconnect: false,
                zoom: 3.2,
                pan: Vector::new(35.0, 20.0),
                placement_orientation: 0,
                cursor_world: (0, 0),
                file_path: default_file.to_string_lossy().to_string(),
                current_file: Some(default_file),
                recent_files,
                last_dir,
                save_format: SaveFormat::Json,
                status_msg: "Pronto — RustSimu ativo".to_string(),
                theme_mode: ThemeMode::Dark,
                system_is_dark: crate::theme::detect_system_dark(),
                layout,
                collection_search: String::new(),
                selected_collection_sym: Some(6003),
                dragging_symbol: None,
                last_collection_click: None,
                last_placed_sym: None,
                prop_tab: 0,
                left_dock_tab: 0,
                left_dock_collapsed: false,
                right_dock_collapsed: false,
                open_menu: None,
                show_about_modal: false,
                undo_stack: Vec::new(),
                redo_stack: Vec::new(),
                drag_undo_snapshot: None,
                clipboard: None,
                copied_info: None,
            },
            iced::Task::none(),
        )
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.doc.clone());
        self.redo_stack.clear();
    }

    /// Conclui a transação de arraste de um componente ou fio no Canvas.
    /// O snapshot do documento anterior ao início do movimento é registrado no histórico de desfazer (Undo)
    /// somente uma vez ao soltar o mouse, caso a posição final seja diferente da inicial.
    fn commit_drag_undo(&mut self) {
        if let Some(snapshot) = self.drag_undo_snapshot.take() {
            if self.doc != snapshot {
                if self.undo_stack.len() >= 50 {
                    self.undo_stack.remove(0);
                }
                self.undo_stack.push(snapshot);
                self.redo_stack.clear();
                self.is_modified = true;
                self.status_msg = "Elemento movido.".to_string();
            }
        }
    }

    pub fn is_dark(&self) -> bool {
        match self.theme_mode.resolve(self.system_is_dark) {
            crate::theme::AppTheme::Dark => true,
            crate::theme::AppTheme::Light => false,
        }
    }

    fn theme(&self) -> Theme {
        self.theme_mode.resolve(self.system_is_dark).theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();
        if self.sim_running {
            subs.push(iced::time::every(Duration::from_millis(80)).map(|_| Message::SimTick));
        }
        if self.layout.drag.is_some() {
            subs.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::LayoutMsg(crate::layout::Msg::DragMove(position.x)))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::LayoutMsg(crate::layout::Msg::DragEnd)),
                    _ => None,
                },
            ));
        }
        subs.push(
            iced::window::resize_events().map(|(_id, size)| {
                Message::LayoutMsg(crate::layout::Msg::WindowResized(size.width))
            }),
        );
        subs.push(iced::keyboard::on_key_press(|key, modifiers| {
            use iced::keyboard::key::Named;
            use iced::keyboard::Key;

            let ctrl = modifiers.command() || modifiers.control();
            let shift = modifiers.shift();
            let alt = modifiers.alt();

            match key {
                Key::Named(Named::Escape) => Some(Message::CancelAction),
                Key::Named(Named::Delete) => Some(Message::DeleteSelected),
                Key::Named(Named::Enter) if ctrl && shift => Some(Message::SelectPropTab(0)),
                Key::Named(Named::PageDown) if ctrl => Some(Message::NextFolio),
                Key::Named(Named::PageUp) if ctrl => Some(Message::PrevFolio),
                Key::Named(Named::Backspace) if ctrl && shift => {
                    Some(Message::DeletePage(usize::MAX))
                }
                Key::Character(ref s) => {
                    let lower = s.to_ascii_lowercase();
                    match lower.as_str() {
                        "n" if ctrl => Some(Message::NewDocument),
                        "o" if ctrl => Some(Message::OpenFileDialog),
                        "s" if ctrl && shift => Some(Message::SaveFileDialog),
                        "s" if ctrl && alt => Some(Message::SaveAll),
                        "s" if ctrl => Some(Message::SaveFileDialog),
                        "w" if ctrl => Some(Message::CloseDocument),
                        "p" if ctrl => Some(Message::PrintDocument),
                        "e" if ctrl && shift => Some(Message::ExportDialog),
                        "e" if ctrl => Some(Message::ExportDialog),
                        "z" if ctrl && shift => Some(Message::Redo),
                        "y" if ctrl => Some(Message::Redo),
                        "z" if ctrl => Some(Message::Undo),
                        "x" if ctrl => Some(Message::Cut),
                        "c" if ctrl && alt => Some(Message::CopyElementInfo),
                        "c" if ctrl => Some(Message::Copy),
                        "v" if ctrl && shift => Some(Message::PasteAtOriginalPos),
                        "v" if ctrl && alt => Some(Message::PasteElementInfo),
                        "v" if ctrl => Some(Message::Paste),
                        "d" if ctrl => Some(Message::Duplicate),
                        "a" if ctrl && shift => Some(Message::DeselectAll),
                        "a" if ctrl => Some(Message::SelectAll),
                        "a" if !ctrl => Some(Message::InsertLastElement),
                        "i" if ctrl => Some(Message::InvertSelection),
                        "t" if ctrl && shift => Some(Message::CycleTheme),
                        "t" if ctrl => Some(Message::AddPageAfter(usize::MAX)),
                        "g" if ctrl && shift => Some(Message::ToggleSnap),
                        "8" if ctrl => Some(Message::ZoomContent),
                        "9" if ctrl => Some(Message::FitToPage),
                        "0" if ctrl => Some(Message::ZoomReset),
                        "+" | "=" if ctrl => Some(Message::ZoomIn),
                        "-" if ctrl => Some(Message::ZoomOut),
                        "r" if shift && !ctrl => Some(Message::RotateCounterClockwise),
                        "r" if !ctrl => Some(Message::RotateClockwise),
                        "m" if !ctrl => Some(Message::MirrorHorizontal),
                        "f" if !ctrl => Some(Message::MirrorVertical),
                        _ => None,
                    }
                }
                _ => None,
            }
        }));
        subs.push(iced::event::listen_with(
            |event, status, _window| match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                    iced::mouse::Button::Left,
                )) if status == iced::event::Status::Ignored => Some(Message::GlobalMouseReleased),
                _ => None,
            },
        ));
        Subscription::batch(subs)
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        // Se um menu suspenso estiver aberto e o usuário disparar uma ação qualquer, fecha o menu automaticamente
        if !matches!(
            message,
            Message::ToggleMenu(_)
                | Message::CloseMenu
                | Message::SimTick
                | Message::Canvas(CanvasMessage::CursorMoved(_, _))
        ) {
            self.open_menu = None;
        }

        match message {
            Message::SelectCategory(cat) => {
                self.active_category = cat;
            }
            Message::SelectTool(tool) => {
                self.tool = tool;
                if self.sim_running && !matches!(tool, EditorTool::Select) {
                    self.sim_running = false;
                    self.doc.reset_simulation_state();
                    self.status_msg = "Simulação encerrada para edição.".to_string();
                }
            }
            Message::ToggleSimulation => {
                self.sim_running = !self.sim_running;
                if self.sim_running {
                    self.tool = EditorTool::Select;
                    self.selected_id = None;
                    step_simulation(&mut self.doc, 0);
                    self.status_msg =
                        "SIMULAÇÃO ATIVA: Clique em -Q1 (Disjuntor) e -S1 (Botão Liga) para partir o motor -M1!"
                            .to_string();
                } else {
                    self.status_msg = "Simulação pausada.".to_string();
                }
            }
            Message::StopSimulation => {
                self.sim_running = false;
                self.doc.reset_simulation_state();
                self.status_msg = "Simulação parada e estados reiniciados.".to_string();
            }
            Message::SimTick => {
                if self.sim_running {
                    step_simulation(&mut self.doc, 80);
                }
            }
            Message::ToggleGrid(v) => {
                self.show_grid = v;
            }
            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.15).clamp(0.4, 8.0);
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.15).clamp(0.4, 8.0);
            }
            Message::ZoomReset => {
                self.zoom = if self.qet_project.is_some() {
                    1.05
                } else {
                    3.2
                };
                self.pan = Vector::new(35.0, 20.0);
            }
            Message::RotateSelectedOrTool => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation = (item.orientation + 1) % 4;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Item {} rotacionado para orientação {}.",
                            item.tag, item.orientation
                        );
                        return iced::Task::none();
                    }
                }
                self.placement_orientation = (self.placement_orientation + 1) % 4;
                self.status_msg = format!(
                    "Orientação de inserção ajustada para {} ({}°).",
                    self.placement_orientation,
                    self.placement_orientation * 90
                );
            }
            Message::MirrorSelectedOrTool => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation = (item.orientation + 4) % 8;
                        self.is_modified = true;
                        return iced::Task::none();
                    }
                }
                self.placement_orientation = (self.placement_orientation + 4) % 8;
            }
            Message::DeleteSelected => {
                if let Some(id) = self.selected_id.take() {
                    self.doc.items.retain(|i| i.id != id);
                    self.is_modified = true;
                    self.status_msg = format!("Item #{id} excluído.");
                }
            }
            Message::NewDocument => {
                self.sim_running = false;
                self.doc = SchematicDocument::default();
                self.pages = vec![crate::model::ProjectPage::new(
                    1,
                    "Folha 1",
                    SchematicDocument::default(),
                )];
                self.active_page_index = 0;
                self.current_file = None;
                self.qet_project = None;
                self.selected_id = None;
                self.is_modified = false;
                self.editing_page_name = None;
                self.page_context_menu = None;
                self.status_msg = "Novo diagrama em branco criado.".to_string();
            }
            Message::LoadPresetDol => {
                self.sim_running = false;
                self.doc = build_dol_starter_preset();
                self.pages = vec![crate::model::ProjectPage::new(
                    1,
                    "Partida Direta Trifásica",
                    self.doc.clone(),
                )];
                self.active_page_index = 0;
                self.current_file = None;
                self.qet_project = None;
                self.selected_id = None;
                self.is_modified = false;
                self.editing_page_name = None;
                self.page_context_menu = None;
                self.status_msg =
                    "Circuito de Partida Direta Trifásica com Selo recarregado.".to_string();
            }
            Message::OpenFileDialog => {
                let start_dir = self.last_dir.clone();
                self.status_msg =
                    "Aguardando seleção de arquivo na janela do sistema...".to_string();
                return iced::Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title(
                                "Abrir Projeto CADe_SIMU / QElectroTech (.cad, .json, .toml, .qet)",
                            )
                            .set_directory(&start_dir)
                            .add_filter(
                                "Todos os Projetos (*.cad, *.json, *.toml, *.qet)",
                                &["cad", "CAD", "json", "JSON", "toml", "TOML", "qet", "QET"],
                            )
                            .add_filter("Projeto QElectroTech (*.qet)", &["qet", "QET"])
                            .add_filter("CADe_SIMU Original (*.cad)", &["cad", "CAD"])
                            .add_filter("Projeto JSON (*.json)", &["json", "JSON"])
                            .add_filter("Projeto TOML (*.toml)", &["toml", "TOML"])
                            .pick_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::FileOpened,
                );
            }
            Message::FileOpened(maybe_path) => {
                let Some(path) = maybe_path else {
                    self.status_msg = "Abertura de arquivo cancelada.".to_string();
                    return iced::Task::none();
                };
                if let Some(parent) = path.parent() {
                    self.last_dir = parent.to_path_buf();
                }
                if let Some(fmt) = SaveFormat::from_path(&path) {
                    self.save_format = fmt;
                }
                self.file_path = path.to_string_lossy().to_string();
                self.current_file = Some(path.clone());

                let is_qet = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("qet"))
                    .unwrap_or(false);

                if is_qet {
                    match crate::qet::load_qet_file(&path) {
                        Ok(project) => {
                            self.sim_running = false;
                            let mut pages = Vec::new();
                            for (idx, diag) in project.diagrams.iter().enumerate() {
                                let page_doc = crate::qet::diagram_to_document(&project, idx);
                                let title = if diag.title.trim().is_empty() {
                                    format!("Folha {}", idx + 1)
                                } else {
                                    diag.title.clone()
                                };
                                pages.push(crate::model::ProjectPage::new(
                                    idx + 1,
                                    title,
                                    page_doc,
                                ));
                            }
                            let active_idx = pages
                                .iter()
                                .position(|p| !p.doc.items.is_empty())
                                .unwrap_or(0);
                            self.doc = pages[active_idx].doc.clone();
                            self.pages = pages;
                            self.active_page_index = active_idx;
                            self.qet_project = Some(project);
                            self.selected_id = None;
                            self.is_modified = false;
                            self.editing_page_name = None;
                            self.page_context_menu = None;
                            let count = self.doc.items.len();
                            let total_pages = self.pages.len();
                            let page_title = self.pages[active_idx].name.clone();
                            self.status_msg = format!(
                                "Projeto QET '{}' aberto ({total_pages} folhas). Folha {}/{}: {} ({count} elementos).",
                                path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.display().to_string()),
                                active_idx + 1,
                                total_pages,
                                page_title
                            );
                        }
                        Err(err) => {
                            self.status_msg =
                                format!("Erro ao abrir QET '{}': {err}", path.display());
                        }
                    }
                } else {
                    self.qet_project = None;
                    match load_document(&path) {
                        Ok(loaded) => {
                            self.sim_running = false;
                            let count = loaded.items.len();
                            let title = if loaded.title_block.title.trim().is_empty() {
                                path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Esquema Principal")
                                    .to_string()
                            } else {
                                loaded.title_block.title.clone()
                            };
                            self.pages =
                                vec![crate::model::ProjectPage::new(1, title, loaded.clone())];
                            self.active_page_index = 0;
                            self.doc = loaded;
                            self.selected_id = None;
                            self.is_modified = false;
                            self.editing_page_name = None;
                            self.page_context_menu = None;
                            self.status_msg = format!(
                                "Projeto '{}' aberto com sucesso ({count} elementos).",
                                path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.display().to_string())
                            );
                        }
                        Err(err) => {
                            self.status_msg = format!("Erro ao abrir '{}': {err}", path.display());
                        }
                    }
                }
            }
            Message::SelectSaveFormat(fmt) => {
                self.save_format = fmt;
                if let Some(ref mut cur) = self.current_file {
                    cur.set_extension(fmt.extension());
                    self.file_path = cur.to_string_lossy().to_string();
                }
                self.status_msg = format!("Formato padrão de salvamento alterado para {}.", fmt);
            }
            Message::SaveFileDialog => {
                let start_dir = self.last_dir.clone();
                let preferred_fmt = self.save_format;
                let stem = self
                    .current_file
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("projeto_cadesimu");
                let default_name = format!("{}.{}", stem, preferred_fmt.extension());

                self.status_msg =
                    "Escolha onde salvar e o formato (.cad, .json, .toml ou .qet) na janela do sistema..."
                        .to_string();

                return iced::Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new()
                            .set_title("Salvar Projeto (.cad, .json, .toml, .qet)")
                            .set_directory(&start_dir)
                            .set_file_name(&default_name);

                        // Coloca o formato selecionado atualmente como filtro principal e lista os demais
                        let mut order = vec![preferred_fmt];
                        for fmt in SaveFormat::ALL {
                            if fmt != preferred_fmt {
                                order.push(fmt);
                            }
                        }
                        for fmt in order {
                            dialog = dialog.add_filter(fmt.filter_label(), &[fmt.extension()]);
                        }

                        dialog.save_file().await.map(|h| h.path().to_path_buf())
                    },
                    Message::FileSaved,
                );
            }
            Message::FileSaved(maybe_path) => {
                let Some(mut path) = maybe_path else {
                    self.status_msg = "Salvamento cancelado.".to_string();
                    return iced::Task::none();
                };
                if let Some(parent) = path.parent() {
                    self.last_dir = parent.to_path_buf();
                }
                // Se o caminho retornado pela janela do sistema não tiver extensão conhecida, aplica o formato escolhido
                if let Some(fmt) = SaveFormat::from_path(&path) {
                    self.save_format = fmt;
                } else {
                    path.set_extension(self.save_format.extension());
                }

                self.file_path = path.to_string_lossy().to_string();
                self.current_file = Some(path.clone());

                let res = if self.save_format == SaveFormat::Qet {
                    if let Some(ref mut qet_proj) = self.qet_project {
                        let active_idx = qet_proj.active_diagram_index;
                        if let Some(diag) = qet_proj.diagrams.get_mut(active_idx) {
                            crate::qet::update_diagram_from_document(diag, &self.doc);
                        }
                        crate::qet::save_qet_file(&path, qet_proj)
                    } else {
                        crate::qet::export_doc_to_qet(&self.doc, &path)
                    }
                } else {
                    save_document(&path, &self.doc)
                };

                match res {
                    Ok(()) => {
                        self.is_modified = false;
                        if self.active_page_index < self.pages.len() {
                            self.pages[self.active_page_index].doc = self.doc.clone();
                        }
                        self.status_msg = format!(
                            "Projeto salvo com sucesso em '{}' ({}).",
                            path.display(),
                            self.save_format
                        );
                    }
                    Err(err) => {
                        self.status_msg = format!("Erro ao salvar '{}': {err}", path.display());
                    }
                }
            }
            Message::FilePathChanged(p) => {
                self.file_path = p;
            }
            Message::LoadFromPath => {
                let path = PathBuf::from(self.file_path.trim());
                if let Some(parent) = path.parent() {
                    if parent.is_dir() {
                        self.last_dir = parent.to_path_buf();
                    }
                }
                if let Some(fmt) = SaveFormat::from_path(&path) {
                    self.save_format = fmt;
                }
                self.current_file = Some(path.clone());

                let is_qet = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("qet"))
                    .unwrap_or(false);

                if is_qet {
                    match crate::qet::load_qet_file(&path) {
                        Ok(project) => {
                            self.sim_running = false;
                            let active_idx = project
                                .diagrams
                                .iter()
                                .position(|d| !d.elements.is_empty())
                                .unwrap_or(0);
                            let doc = crate::qet::diagram_to_document(&project, active_idx);
                            let count = doc.items.len();
                            let total_pages = project.diagrams.len();
                            let page_title = project
                                .diagrams
                                .get(active_idx)
                                .map(|d| d.title.clone())
                                .unwrap_or_default();
                            self.doc = doc;
                            self.qet_project = Some(project);
                            self.selected_id = None;
                            self.status_msg = format!(
                                "Projeto QET '{}' aberto ({total_pages} folhas). Folha {}/{}: {} ({count} elementos).",
                                path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.display().to_string()),
                                active_idx + 1,
                                total_pages,
                                page_title
                            );
                        }
                        Err(err) => {
                            self.status_msg = format!("Erro ao abrir QET: {err}");
                        }
                    }
                } else {
                    self.qet_project = None;
                    match load_document(&path) {
                        Ok(loaded) => {
                            self.sim_running = false;
                            let count = loaded.items.len();
                            self.doc = loaded;
                            self.selected_id = None;
                            self.status_msg = format!(
                                "Arquivo '{}' carregado com sucesso ({count} elementos).",
                                path.display()
                            );
                        }
                        Err(err) => {
                            self.status_msg = format!("Erro ao abrir: {err}");
                        }
                    }
                }
            }
            Message::PrevFolio => {
                if let Some(ref mut qet_proj) = self.qet_project {
                    if qet_proj.active_diagram_index > 0 {
                        let cur_idx = qet_proj.active_diagram_index;
                        if let Some(diag) = qet_proj.diagrams.get_mut(cur_idx) {
                            crate::qet::update_diagram_from_document(diag, &self.doc);
                        }
                        qet_proj.active_diagram_index -= 1;
                        let new_idx = qet_proj.active_diagram_index;
                        self.doc = crate::qet::diagram_to_document(qet_proj, new_idx);
                        self.sim_running = false;
                        self.selected_id = None;
                        let title = qet_proj
                            .diagrams
                            .get(new_idx)
                            .map(|d| d.title.as_str())
                            .unwrap_or("");
                        let count = self.doc.items.len();
                        self.status_msg = format!(
                            "Folha {} / {}: {} ({count} elementos)",
                            new_idx + 1,
                            qet_proj.diagrams.len(),
                            title
                        );
                    }
                }
            }
            Message::NextFolio => {
                if let Some(ref mut qet_proj) = self.qet_project {
                    if qet_proj.active_diagram_index + 1 < qet_proj.diagrams.len() {
                        let cur_idx = qet_proj.active_diagram_index;
                        if let Some(diag) = qet_proj.diagrams.get_mut(cur_idx) {
                            crate::qet::update_diagram_from_document(diag, &self.doc);
                        }
                        qet_proj.active_diagram_index += 1;
                        let new_idx = qet_proj.active_diagram_index;
                        self.doc = crate::qet::diagram_to_document(qet_proj, new_idx);
                        self.sim_running = false;
                        self.selected_id = None;
                        let title = qet_proj
                            .diagrams
                            .get(new_idx)
                            .map(|d| d.title.as_str())
                            .unwrap_or("");
                        let count = self.doc.items.len();
                        self.status_msg = format!(
                            "Folha {} / {}: {} ({count} elementos)",
                            new_idx + 1,
                            qet_proj.diagrams.len(),
                            title
                        );
                    }
                }
            }
            Message::SelectFolio(new_idx) => {
                if let Some(ref mut qet_proj) = self.qet_project {
                    if new_idx < qet_proj.diagrams.len() {
                        let cur_idx = qet_proj.active_diagram_index;
                        if let Some(diag) = qet_proj.diagrams.get_mut(cur_idx) {
                            crate::qet::update_diagram_from_document(diag, &self.doc);
                        }
                        qet_proj.active_diagram_index = new_idx;
                        self.doc = crate::qet::diagram_to_document(qet_proj, new_idx);
                        self.sim_running = false;
                        self.selected_id = None;
                        let title = qet_proj
                            .diagrams
                            .get(new_idx)
                            .map(|d| d.title.as_str())
                            .unwrap_or("");
                        let count = self.doc.items.len();
                        self.status_msg = format!(
                            "Folha {} / {}: {} ({count} elementos)",
                            new_idx + 1,
                            qet_proj.diagrams.len(),
                            title
                        );
                    }
                }
            }
            Message::SaveAsCad => {
                let mut path = PathBuf::from(self.file_path.trim());
                path.set_extension("cad");
                self.file_path = path.to_string_lossy().to_string();
                match save_document(&path, &self.doc) {
                    Ok(()) => {
                        self.status_msg = format!(
                            "Salvo no formato legado CADe_SIMU (.cad): '{}'",
                            path.display()
                        );
                    }
                    Err(e) => self.status_msg = e,
                }
            }
            Message::SaveAsJson => {
                let mut path = PathBuf::from(self.file_path.trim());
                path.set_extension("json");
                self.file_path = path.to_string_lossy().to_string();
                match save_document(&path, &self.doc) {
                    Ok(()) => {
                        self.status_msg =
                            format!("Salvo em formato nativo JSON: '{}'", path.display());
                    }
                    Err(e) => self.status_msg = e,
                }
            }
            Message::SaveAsToml => {
                let mut path = PathBuf::from(self.file_path.trim());
                path.set_extension("toml");
                self.file_path = path.to_string_lossy().to_string();
                match save_document(&path, &self.doc) {
                    Ok(()) => {
                        self.status_msg =
                            format!("Salvo em formato nativo TOML: '{}'", path.display());
                    }
                    Err(e) => self.status_msg = e,
                }
            }
            Message::EditTag(new_tag) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.tag = new_tag;
                        self.is_modified = true;
                    }
                }
            }
            Message::EditValue(new_val) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.value = new_val;
                        self.is_modified = true;
                    }
                }
            }
            Message::EditPin(pin_idx, val) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        if pin_idx < item.pins.len() {
                            item.pins[pin_idx] = val;
                            self.is_modified = true;
                        }
                    }
                }
            }
            Message::ToggleShowTag(v) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.show_tag = if v { 1 } else { 0 };
                        self.is_modified = true;
                    }
                }
            }
            Message::ToggleShowValue(v) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.show_value = if v { 1 } else { 0 };
                        self.is_modified = true;
                    }
                }
            }
            Message::EditTitleBlockTitle(v) => {
                self.doc.title_block.title = v;
                self.is_modified = true;
            }
            Message::EditTitleBlockSubtitle(v) => {
                self.doc.title_block.subtitle = v;
                self.is_modified = true;
            }
            Message::EditTitleBlockAuthor(v) => {
                self.doc.title_block.drawn_by = v;
                self.is_modified = true;
            }
            Message::Canvas(cmsg) => match cmsg {
                CanvasMessage::SelectItem(sel) => {
                    self.selected_id = sel;
                }
                CanvasMessage::MoveItem { id, x1, y1 } => {
                    let old_pos = self
                        .doc
                        .items
                        .iter()
                        .find(|i| i.id == id)
                        .map(|i| (i.x1, i.y1));
                    if let Some((ox, oy)) = old_pos {
                        let dx = x1 - ox;
                        let dy = y1 - oy;
                        if dx != 0 || dy != 0 {
                            if self.drag_undo_snapshot.is_none() {
                                self.drag_undo_snapshot = Some(self.doc.clone());
                            }
                            if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                                item.x1 = x1;
                                item.y1 = y1;
                                item.x2 = x1;
                                item.y2 = y1;
                                self.is_modified = true;
                            }
                            if self.auto_disconnect {
                                // Remove fios conectados ao ponto anterior
                                self.doc.items.retain(|i| {
                                    if i.is_wire() {
                                        !((i.x1 == ox && i.y1 == oy) || (i.x2 == ox && i.y2 == oy))
                                    } else {
                                        true
                                    }
                                });
                            } else {
                                // Padrão: move pontas de cabos conectados para manter o circuito integrado
                                for w in self.doc.items.iter_mut() {
                                    if w.is_wire() && w.id != id {
                                        if w.x1 == ox && w.y1 == oy {
                                            w.x1 += dx;
                                            w.y1 += dy;
                                        }
                                        if w.x2 == ox && w.y2 == oy {
                                            w.x2 += dx;
                                            w.y2 += dy;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                CanvasMessage::EndMoveItem { id: _ } => {
                    self.commit_drag_undo();
                }
                CanvasMessage::PlaceComponent { tipo, x, y } => {
                    self.last_placed_sym = Some(tipo);
                    self.dragging_symbol = None;
                    self.tool = EditorTool::Select;
                    if let Some(sym) = find_symbol(tipo) {
                        self.push_undo();
                        let new_id = self.doc.next_id();
                        let mut item = sym.instantiate(new_id, x, y, self.placement_orientation);
                        let role = classify_apparatus_role(item.tipo);

                        if role.is_independent_apparatus() {
                            // Aparelhos primários (bobinas, disjuntores, botões, motores, sinaleiros...)
                            // recebem tag sequencial automática entre elementos do mesmo tipo: -KM1, -KM2, -S1, -S2, etc.
                            let prefix = if sym.default_tag.is_empty() {
                                "-K"
                            } else {
                                sym.default_tag
                            };
                            item.tag = next_sequential_tag_for_role(&self.doc.items, role, prefix);
                        } else if role.is_subordinate_contact() {
                            // Contatos subordinados (contatores de força, contatos auxiliares NA/NF, temporizados, térmicos)
                            // Devem se vincular ao aparelho pai correspondente existente no esquema, se houver
                            if let Some(parent_role) = role.required_parent_role() {
                                let parent_item = self.doc.items.iter().rfind(|i| {
                                    classify_apparatus_role(i.tipo) == parent_role
                                        && !i.tag.trim().is_empty()
                                });
                                if let Some(parent) = parent_item {
                                    item.tag = parent.tag.clone();
                                } else if !sym.default_tag.is_empty() {
                                    item.tag = format!("{}1", sym.default_tag.trim());
                                }
                            }

                            // Para contatos auxiliares NA/NF com numeração sequencial de bornes IEC:
                            if let Some(kind) = detect_contact_kind(item.tipo, &sym.pins) {
                                let occupied = find_occupied_sequences_for_tag(
                                    &self.doc.items,
                                    &item.tag,
                                    None,
                                );
                                if !occupied.is_empty() {
                                    let next_seq =
                                        (1..=9).find(|d| !occupied.contains(d)).unwrap_or(1);
                                    let new_pins = kind.generate_pins(next_seq);
                                    for (idx, p_val) in new_pins.into_iter().enumerate() {
                                        if idx < item.pins.len() {
                                            item.pins[idx] = p_val;
                                        }
                                    }
                                }
                            }
                        }

                        // Autoconectar terminais alinhados (padrão ativado)
                        if self.auto_connect {
                            let pins = sym.world_pins(&item);
                            for (_, _, (px, py)) in pins {
                                let hits_wire = self.doc.items.iter().any(|other| {
                                    if other.is_wire() {
                                        (other.x1 == px && other.y1 == py)
                                            || (other.x2 == px && other.y2 == py)
                                    } else {
                                        false
                                    }
                                });
                                if hits_wire {
                                    let junc_id = self.doc.next_id();
                                    self.doc
                                        .items
                                        .push(crate::model::SchematicItem::new_junction(
                                            junc_id, px, py,
                                        ));
                                }
                            }
                        }

                        self.doc.items.push(item);
                        self.selected_id = Some(new_id);
                        self.is_modified = true;
                        self.status_msg = format!("Inserido '{}' em ({x}, {y}).", sym.name);
                    }
                }
                CanvasMessage::AddWire {
                    tipo,
                    x1,
                    y1,
                    x2,
                    y2,
                } => {
                    self.push_undo();
                    let new_id = self.doc.next_id();
                    self.doc
                        .items
                        .push(SchematicItem::new_wire(new_id, tipo, x1, y1, x2, y2));
                    self.is_modified = true;
                    self.status_msg = format!("Cabo traçado de ({x1}, {y1}) a ({x2}, {y2}).");
                }
                CanvasMessage::AddJunction { x, y } => {
                    self.push_undo();
                    let new_id = self.doc.next_id();
                    self.doc
                        .items
                        .push(SchematicItem::new_junction(new_id, x, y));
                    self.is_modified = true;
                    self.status_msg = format!("Conexão (nó) adicionada em ({x}, {y}).");
                }
                CanvasMessage::DeleteItem(id) => {
                    self.push_undo();
                    self.doc.items.retain(|i| i.id != id);
                    self.is_modified = true;
                    if self.selected_id == Some(id) {
                        self.selected_id = None;
                    }
                }
                CanvasMessage::SimPressComponent(id) => {
                    press_component_on_sim(&mut self.doc, id);
                    step_simulation(&mut self.doc, 0);
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == id) {
                        let label = find_symbol(item.tipo)
                            .map(|s| s.short_label)
                            .or_else(|| {
                                item.qet_type
                                    .as_deref()
                                    .map(|s| s.split('/').next_back().unwrap_or(s))
                            })
                            .unwrap_or("Item");
                        if crate::sim::is_item_momentary(item) {
                            self.status_msg = format!(
                                "Simulação: '{}' ({label}) -> PRESSIONADO (solte o mouse para retornar)",
                                item.tag
                            );
                        } else {
                            self.status_msg = format!(
                                "Simulação: '{}' ({label}) -> {}",
                                item.tag,
                                if item.sim_fault {
                                    "DESARMADO POR CURTO-CIRCUITO (TRIP)"
                                } else if item.sim_manual_latched {
                                    "ACIONADO / FECHADO"
                                } else {
                                    "REARME / REPOUSO"
                                }
                            );
                        }
                    }
                }
                CanvasMessage::SimReleaseComponent(id) => {
                    let maybe_info = self.doc.items.iter().find(|i| i.id == id).and_then(|item| {
                        if crate::sim::is_item_momentary(item) {
                            let tag = item.tag.clone();
                            let label = find_symbol(item.tipo)
                                .map(|s| s.short_label.to_string())
                                .or_else(|| {
                                    item.qet_type
                                        .as_deref()
                                        .map(|s| s.split('/').next_back().unwrap_or(s).to_string())
                                })
                                .unwrap_or_else(|| "Botão".to_string());
                            Some((tag, label))
                        } else {
                            None
                        }
                    });
                    if let Some((tag, label)) = maybe_info {
                        release_component_on_sim(&mut self.doc, id);
                        step_simulation(&mut self.doc, 0);
                        self.status_msg = format!(
                            "Simulação: '{tag}' ({label}) -> SOLTO (retornou ao estado original)"
                        );
                    }
                }
                CanvasMessage::RotatePlacement => {
                    self.placement_orientation = (self.placement_orientation + 1) % 4;
                }
                CanvasMessage::PanBy(dx, dy) => {
                    self.pan.x += dx;
                    self.pan.y += dy;
                }
                CanvasMessage::ZoomAt { factor, cursor } => {
                    let old_zoom = self.zoom;
                    let new_zoom = (old_zoom * factor).clamp(0.8, 8.0);
                    let wx = (cursor.x - self.pan.x) / old_zoom;
                    let wy = (cursor.y - self.pan.y) / old_zoom;
                    self.zoom = new_zoom;
                    self.pan.x = cursor.x - wx * new_zoom;
                    self.pan.y = cursor.y - wy * new_zoom;
                }
                CanvasMessage::CursorMoved(wx, wy) => {
                    self.cursor_world = (wx, wy);
                }
                CanvasMessage::CancelAction => {
                    if let Some(snapshot) = self.drag_undo_snapshot.take() {
                        self.doc = snapshot;
                        self.status_msg = "Movimento cancelado.".to_string();
                    } else {
                        self.status_msg = "Ação cancelada. Modo de seleção ativado.".to_string();
                    }
                    self.tool = EditorTool::Select;
                    self.dragging_symbol = None;
                }
            },
            Message::CycleTheme => {
                self.theme_mode = self.theme_mode.next();
            }
            Message::LayoutMsg(msg) => {
                self.layout.update(msg);
            }
            Message::CollectionSearchChanged(query) => {
                self.collection_search = query;
            }
            Message::StartSymbolDrag(tipo) => {
                self.dragging_symbol = Some(tipo);
                self.selected_collection_sym = Some(tipo);
            }
            Message::CancelAction => {
                if let Some(snapshot) = self.drag_undo_snapshot.take() {
                    self.doc = snapshot;
                    self.status_msg = "Movimento cancelado.".to_string();
                } else {
                    self.status_msg = "Ação cancelada. Modo de seleção ativado.".to_string();
                }
                self.open_menu = None;
                self.tool = EditorTool::Select;
                self.dragging_symbol = None;
            }
            Message::GlobalMouseReleased => {
                self.dragging_symbol = None;
                self.commit_drag_undo();
            }
            Message::SelectCollectionSymbol(tipo) => {
                self.dragging_symbol = None;
                let now = std::time::Instant::now();
                let is_double_click = if let Some((last_tipo, last_time)) =
                    self.last_collection_click
                {
                    last_tipo == tipo && now.duration_since(last_time) < Duration::from_millis(400)
                } else {
                    false
                };
                self.last_collection_click = Some((tipo, now));
                self.selected_collection_sym = Some(tipo);

                if is_double_click {
                    self.tool = EditorTool::PlaceSymbol(tipo);
                    if let Some(sym) = find_symbol(tipo) {
                        self.status_msg = format!(
                            "Símbolo '{}' pronto no ponteiro — clique na folha para posicionar.",
                            sym.name
                        );
                    }
                } else {
                    // Clique simples: apenas seleciona o símbolo para pré-visualização, NÃO arma o cursor!
                    if let Some(sym) = find_symbol(tipo) {
                        self.status_msg = format!(
                            "Símbolo '{}' selecionado. Dê duplo clique, arraste ou clique em 'Inserir no Esquema'.",
                            sym.name
                        );
                    }
                }
            }
            Message::InsertSelectedCollectionSymbol => {
                if let Some(tipo) = self.selected_collection_sym {
                    self.tool = EditorTool::PlaceSymbol(tipo);
                    if let Some(sym) = find_symbol(tipo) {
                        self.status_msg = format!(
                            "Símbolo '{}' pronto no ponteiro — clique na folha para posicionar.",
                            sym.name
                        );
                    }
                }
            }
            Message::SelectPropTab(tab) => {
                self.prop_tab = tab;
            }
            Message::CloseDocument => {
                self.sim_running = false;
                self.doc = SchematicDocument::default();
                self.pages = vec![crate::model::ProjectPage::new(
                    1,
                    "Folha 1",
                    SchematicDocument::default(),
                )];
                self.active_page_index = 0;
                self.current_file = None;
                self.qet_project = None;
                self.selected_id = None;
                self.is_modified = false;
                self.editing_page_name = None;
                self.page_context_menu = None;
                self.status_msg = "Esquema fechado.".to_string();
            }
            Message::SelectPage(idx) => {
                if idx < self.pages.len() && idx != self.active_page_index {
                    self.pages[self.active_page_index].doc = self.doc.clone();
                    self.active_page_index = idx;
                    self.doc = self.pages[idx].doc.clone();
                    self.selected_id = None;
                    self.sim_running = false;
                    self.editing_page_name = None;
                    self.page_context_menu = None;
                    self.status_msg = format!("Folha {} ativa: {}", idx + 1, self.pages[idx].name);
                }
            }
            Message::AddPageBefore(idx) => {
                self.pages[self.active_page_index].doc = self.doc.clone();
                let insert_idx = idx.min(self.pages.len());
                let new_id = self.pages.len() + 1;
                let new_page = crate::model::ProjectPage::new(
                    new_id,
                    format!("Folha {}", new_id),
                    SchematicDocument::default(),
                );
                self.pages.insert(insert_idx, new_page);
                self.active_page_index = insert_idx;
                self.doc = self.pages[insert_idx].doc.clone();
                self.selected_id = None;
                self.sim_running = false;
                self.is_modified = true;
                self.page_context_menu = None;
                self.status_msg =
                    format!("Nova folha inserida antes da posição {}.", insert_idx + 1);
            }
            Message::AddPageAfter(raw_idx) => {
                let idx = if raw_idx == usize::MAX {
                    self.active_page_index
                } else {
                    raw_idx
                };
                self.pages[self.active_page_index].doc = self.doc.clone();
                let insert_idx = (idx + 1).min(self.pages.len());
                let new_id = self.pages.len() + 1;
                let new_page = crate::model::ProjectPage::new(
                    new_id,
                    format!("Folha {}", new_id),
                    SchematicDocument::default(),
                );
                self.pages.insert(insert_idx, new_page);
                self.active_page_index = insert_idx;
                self.doc = self.pages[insert_idx].doc.clone();
                self.selected_id = None;
                self.sim_running = false;
                self.is_modified = true;
                self.page_context_menu = None;
                self.status_msg = format!("Nova folha inserida na posição {}.", insert_idx + 1);
            }
            Message::DeletePage(raw_idx) => {
                let idx = if raw_idx == usize::MAX {
                    self.active_page_index
                } else {
                    raw_idx
                };
                if self.pages.len() <= 1 {
                    self.doc = SchematicDocument::default();
                    self.pages[0] =
                        crate::model::ProjectPage::new(1, "Folha 1", SchematicDocument::default());
                    self.active_page_index = 0;
                    self.is_modified = true;
                    self.page_context_menu = None;
                    self.status_msg = "Última folha limpa.".to_string();
                } else if idx < self.pages.len() {
                    self.pages.remove(idx);
                    if self.active_page_index >= self.pages.len() {
                        self.active_page_index = self.pages.len() - 1;
                    }
                    self.doc = self.pages[self.active_page_index].doc.clone();
                    self.selected_id = None;
                    self.sim_running = false;
                    self.is_modified = true;
                    self.page_context_menu = None;
                    self.status_msg = format!("Folha {} excluída.", idx + 1);
                }
            }
            Message::MovePage(from_idx, to_idx) => {
                if from_idx < self.pages.len() && to_idx < self.pages.len() && from_idx != to_idx {
                    self.pages[self.active_page_index].doc = self.doc.clone();
                    let page = self.pages.remove(from_idx);
                    self.pages.insert(to_idx, page);
                    self.active_page_index = to_idx;
                    self.doc = self.pages[to_idx].doc.clone();
                    self.is_modified = true;
                    self.page_context_menu = None;
                    self.status_msg = format!("Folha movida para a posição {}.", to_idx + 1);
                }
            }
            Message::StartEditingPageName(idx) => {
                if idx < self.pages.len() {
                    self.editing_page_name = Some(self.pages[idx].name.clone());
                    self.page_context_menu = None;
                }
            }
            Message::UpdateEditingPageName(new_name) => {
                self.editing_page_name = Some(new_name);
            }
            Message::CommitEditingPageName => {
                if let Some(new_name) = self.editing_page_name.take() {
                    let trimmed = new_name.trim();
                    if !trimmed.is_empty() && self.active_page_index < self.pages.len() {
                        self.pages[self.active_page_index].name = trimmed.to_string();
                        self.is_modified = true;
                    }
                }
            }
            Message::TogglePageContextMenu(idx) => {
                if self.page_context_menu == Some(idx) {
                    self.page_context_menu = None;
                } else {
                    self.page_context_menu = Some(idx);
                }
            }
            Message::ClosePageContextMenu => {
                self.page_context_menu = None;
            }
            Message::ToggleMenu(menu) => {
                if self.open_menu == menu {
                    self.open_menu = None;
                } else {
                    self.open_menu = menu;
                }
            }
            Message::CloseMenu => {
                self.open_menu = None;
                self.tool = EditorTool::Select;
            }
            Message::Undo => {
                self.drag_undo_snapshot = None;
                if let Some(prev) = self.undo_stack.pop() {
                    self.redo_stack.push(self.doc.clone());
                    self.doc = prev;
                    self.is_modified = true;
                    self.status_msg = "Desfazer executado.".to_string();
                } else {
                    self.status_msg = "Nenhuma ação para desfazer.".to_string();
                }
            }
            Message::Redo => {
                self.drag_undo_snapshot = None;
                if let Some(next) = self.redo_stack.pop() {
                    self.undo_stack.push(self.doc.clone());
                    self.doc = next;
                    self.is_modified = true;
                    self.status_msg = "Refazer executado.".to_string();
                } else {
                    self.status_msg = "Nenhuma ação para refazer.".to_string();
                }
            }
            Message::Cut => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == id) {
                        self.clipboard = Some(item.clone());
                    }
                    self.push_undo();
                    self.doc.items.retain(|i| i.id != id);
                    self.selected_id = None;
                    self.is_modified = true;
                    self.status_msg = "Recortado para área de transferência.".to_string();
                }
            }
            Message::Copy => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == id) {
                        self.clipboard = Some(item.clone());
                        self.status_msg = format!("Copiado: {}", item.tag);
                    }
                }
            }
            Message::Paste => {
                let clip = self.clipboard.clone();
                if let Some(item) = clip {
                    self.push_undo();
                    let mut new_item = item;
                    new_item.id = self.doc.next_id();
                    new_item.x1 += 12;
                    new_item.y1 += 12;
                    new_item.x2 += 12;
                    new_item.y2 += 12;
                    let new_id = new_item.id;
                    self.doc.items.push(new_item);
                    self.selected_id = Some(new_id);
                    self.is_modified = true;
                    self.status_msg = "Elemento colado.".to_string();
                }
            }
            Message::Duplicate => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == id).cloned() {
                        self.push_undo();
                        let mut new_item = item;
                        new_item.id = self.doc.next_id();
                        new_item.x1 += 12;
                        new_item.y1 += 12;
                        new_item.x2 += 12;
                        new_item.y2 += 12;

                        let role = classify_apparatus_role(new_item.tipo);
                        if role.is_independent_apparatus() {
                            let prefix = extract_tag_prefix(&new_item.tag);
                            new_item.tag =
                                next_sequential_tag_for_role(&self.doc.items, role, prefix);
                        } else if let Some(sym) = find_symbol(new_item.tipo) {
                            if let Some(kind) = detect_contact_kind(new_item.tipo, &sym.pins) {
                                let occupied = find_occupied_sequences_for_tag(
                                    &self.doc.items,
                                    &new_item.tag,
                                    None,
                                );
                                let next_seq = (1..=9).find(|d| !occupied.contains(d)).unwrap_or(1);
                                let new_pins = kind.generate_pins(next_seq);
                                for (idx, p_val) in new_pins.into_iter().enumerate() {
                                    if idx < new_item.pins.len() {
                                        new_item.pins[idx] = p_val;
                                    }
                                }
                            }
                        }

                        let new_id = new_item.id;
                        self.doc.items.push(new_item);
                        self.selected_id = Some(new_id);
                        self.is_modified = true;
                        self.status_msg = "Elemento duplicado.".to_string();
                    }
                }
            }
            Message::RotateClockwise => {
                self.placement_orientation = (self.placement_orientation + 1) % 8;
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation = (item.orientation + 1) % 8;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Elemento #{} girado para orientação {}.",
                            id, item.orientation
                        );
                    }
                }
            }
            Message::RotateCounterClockwise => {
                self.placement_orientation = (self.placement_orientation + 7) % 8;
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation = (item.orientation + 7) % 8;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Elemento #{} girado anti-horário para orientação {}.",
                            id, item.orientation
                        );
                    }
                }
            }
            Message::MirrorHorizontal => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation ^= 4;
                        self.is_modified = true;
                        self.status_msg = format!("Elemento #{} espelhado horizontalmente.", id);
                    }
                }
            }
            Message::MirrorVertical => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation ^= 2;
                        self.is_modified = true;
                        self.status_msg = format!("Elemento #{} espelhado verticalmente.", id);
                    }
                }
            }
            Message::AlignToGrid => {
                if let Some(id) = self.selected_id {
                    let step = self.grid_density.step();
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.x1 = ((item.x1 as f32 / step as f32).round() as i32) * step;
                        item.y1 = ((item.y1 as f32 / step as f32).round() as i32) * step;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Elemento #{} alinhado à grade ({}, {}).",
                            id, item.x1, item.y1
                        );
                    }
                }
            }
            Message::FitToPage => {
                self.zoom = 2.8;
                self.pan = Vector::new(20.0, 10.0);
                self.status_msg = "Visualização ajustada à página.".to_string();
            }
            Message::ToggleShowTexts => {
                self.show_texts = !self.show_texts;
                self.status_msg = if self.show_texts {
                    "Textos e nomes visíveis.".to_string()
                } else {
                    "Textos e nomes ocultos.".to_string()
                };
            }
            Message::ToggleAutoConnect => {
                self.auto_connect = !self.auto_connect;
                self.status_msg = format!(
                    "Autoconexão de terminais: {}",
                    if self.auto_connect {
                        "Ativada"
                    } else {
                        "Desativada"
                    }
                );
            }
            Message::ToggleAutoDisconnect => {
                self.auto_disconnect = !self.auto_disconnect;
                self.status_msg = format!(
                    "Desconectar ao mover: {}",
                    if self.auto_disconnect {
                        "Ativado"
                    } else {
                        "Desativado"
                    }
                );
            }
            Message::SetGridDensity(density) => {
                self.grid_density = density;
                self.status_msg = format!("Densidade da grade: {}.", density.label());
            }
            Message::SaveAll => {
                self.pages[self.active_page_index].doc = self.doc.clone();
                self.is_modified = false;
                self.status_msg = "Todas as folhas salvas com sucesso.".to_string();
            }
            Message::PrintDocument => {
                self.status_msg = "Preparando esquema para impressão...".to_string();
            }
            Message::ShowAbout => {
                self.show_about_modal = true;
                self.open_menu = None;
            }
            Message::CloseAbout => {
                self.show_about_modal = false;
            }
            Message::ToggleLeftDockCollapse => {
                self.left_dock_collapsed = !self.left_dock_collapsed;
            }
            Message::ToggleRightDockCollapse => {
                self.right_dock_collapsed = !self.right_dock_collapsed;
            }
            Message::SelectLeftDockTab(idx) => {
                self.left_dock_tab = idx;
            }
            Message::RecentProjectSelected(path_str) => {
                let p = PathBuf::from(&path_str);
                return iced::Task::done(Message::FileOpened(Some(p)));
            }
            Message::DeleteRecentProject(path_str) => {
                self.recent_files.retain(|p| p != &path_str);
                self.status_msg = "Projeto removido da lista recente.".to_string();
            }
            Message::SetLampColor(color_code) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.color = color_code;
                        self.is_modified = true;
                        self.status_msg =
                            format!("Cor da lâmpada alterada para código {}.", color_code);
                    }
                }
            }
            Message::SetPulseDuration(dur) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.value = dur;
                        self.is_modified = true;
                    }
                }
            }
            Message::ApplyIecNumberingPreset(preset) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        match preset {
                            "power_3p" => {
                                item.pins[0] = "1".to_string();
                                item.pins[1] = "3".to_string();
                                item.pins[2] = "5".to_string();
                                item.pins[3] = "2".to_string();
                                item.pins[4] = "4".to_string();
                                item.pins[5] = "6".to_string();
                            }
                            "aux_no" => {
                                item.pins[0] = "13".to_string();
                                item.pins[1] = "14".to_string();
                            }
                            "aux_nc" => {
                                item.pins[0] = "21".to_string();
                                item.pins[1] = "22".to_string();
                            }
                            "thermal" => {
                                item.pins[0] = "95".to_string();
                                item.pins[1] = "96".to_string();
                                item.pins[2] = "97".to_string();
                                item.pins[3] = "98".to_string();
                            }
                            "coil" => {
                                item.pins[0] = "A1".to_string();
                                item.pins[1] = "A2".to_string();
                            }
                            "lamp" => {
                                item.pins[0] = "X1".to_string();
                                item.pins[1] = "X2".to_string();
                            }
                            _ => {}
                        }
                        self.is_modified = true;
                        self.status_msg = format!("Padrão IEC '{}' aplicado.", preset);
                    }
                }
            }
            Message::SelectPinSequence(digit) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        if let Some(sym) = find_symbol(item.tipo) {
                            if let Some(kind) = detect_contact_kind(item.tipo, &sym.pins) {
                                let new_pins = kind.generate_pins(digit);
                                for (idx, p_val) in new_pins.into_iter().enumerate() {
                                    if idx < item.pins.len() {
                                        item.pins[idx] = p_val;
                                    }
                                }
                                self.is_modified = true;
                                self.status_msg = format!(
                                    "Sequência {} aplicada aos bornes do componente '{}' (Tag: {}).",
                                    digit, sym.name, item.tag
                                );
                            }
                        }
                    }
                }
            }
            Message::PasteAtOriginalPos => {
                let clip_opt = self.clipboard.clone();
                if let Some(mut new_item) = clip_opt {
                    self.push_undo();
                    let next_id = self.doc.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                    new_item.id = next_id;
                    self.doc.items.push(new_item);
                    self.selected_id = Some(next_id);
                    self.is_modified = true;
                    self.status_msg = "Elemento colado na posição original.".to_string();
                }
            }
            Message::SelectAll => {
                if let Some(first) = self.doc.items.first() {
                    self.selected_id = Some(first.id);
                    self.status_msg = format!(
                        "Todos os elementos selecionados (total: {}).",
                        self.doc.items.len()
                    );
                }
            }
            Message::DeselectAll => {
                self.selected_id = None;
                self.status_msg = "Seleção anulada.".to_string();
            }
            Message::InvertSelection => {
                if let Some(cur_id) = self.selected_id {
                    let next = self.doc.items.iter().find(|i| i.id != cur_id).map(|i| i.id);
                    self.selected_id = next;
                } else if let Some(first) = self.doc.items.first() {
                    self.selected_id = Some(first.id);
                }
                self.status_msg = "Seleção invertida.".to_string();
            }
            Message::InsertLastElement => {
                if let Some(sym) = self.last_placed_sym.or(self.selected_collection_sym) {
                    self.tool = EditorTool::PlaceSymbol(sym);
                    let name = find_symbol(sym).map(|s| s.name).unwrap_or("símbolo");
                    self.status_msg = format!("Armado no cursor o último elemento: {name}. Clique no canvas para posicionar.");
                }
            }
            Message::CopyElementInfo => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == id) {
                        self.copied_info =
                            Some((item.tag.clone(), item.value.clone(), item.pins.clone()));
                        self.status_msg = format!("Informações do elemento #{} copiadas.", id);
                    }
                }
            }
            Message::PasteElementInfo => {
                let info_opt = self.copied_info.clone();
                if let Some((tag, val, pins)) = info_opt {
                    if let Some(id) = self.selected_id {
                        self.push_undo();
                        if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                            item.tag = tag;
                            item.value = val;
                            item.pins = pins;
                            self.is_modified = true;
                            self.status_msg = format!("Informações coladas no elemento #{}.", id);
                        }
                    }
                }
            }
            Message::ZoomContent => {
                if self.doc.items.is_empty() {
                    self.zoom = 3.2;
                    self.pan = Vector::new(35.0, 20.0);
                } else {
                    let mut min_x = i32::MAX;
                    let mut max_x = i32::MIN;
                    let mut min_y = i32::MAX;
                    let mut max_y = i32::MIN;
                    for item in &self.doc.items {
                        min_x = min_x.min(item.x1).min(item.x2);
                        max_x = max_x.max(item.x1).max(item.x2);
                        min_y = min_y.min(item.y1).min(item.y2);
                        max_y = max_y.max(item.y1).max(item.y2);
                    }
                    let cx = (min_x + max_x) as f32 / 2.0;
                    let cy = (min_y + max_y) as f32 / 2.0;
                    self.pan = Vector::new(-cx * 2.5 + 400.0, -cy * 2.5 + 300.0);
                    self.zoom = 2.5;
                }
                self.status_msg = "Zoom ajustado ao conteúdo dos elementos.".to_string();
            }
            Message::ExportDialog => {
                self.status_msg =
                    "Exportação: Escolha o formato desejado (PDF, SVG, PNG, CAD).".to_string();
                return iced::Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Exportar Esquema Elétrico")
                            .add_filter("Documento PDF (*.pdf)", &["pdf"])
                            .add_filter("Gráfico Vetorial SVG (*.svg)", &["svg"])
                            .add_filter("CADe_SIMU (*.cad)", &["cad"])
                            .save_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::FileSaved,
                );
            }
            Message::ToggleSnap => {
                self.snap_to_grid = !self.snap_to_grid;
                self.status_msg = if self.snap_to_grid {
                    "Snap à grelha ativado.".to_string()
                } else {
                    "Snap à grelha desativado.".to_string()
                };
            }
            Message::ColorElement => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.color = (item.color + 1) % 5;
                        self.is_modified = true;
                        self.status_msg = format!("Cor do elemento #{} alterada.", id);
                    }
                }
            }
        }
        iced::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        let main_ui = column![
            self.view_menubar(),
            self.view_toolbar(),
            self.layout.view(
                self.left_dock_collapsed,
                self.right_dock_collapsed,
                Message::LayoutMsg,
                |kind| match kind {
                    Kind::Collections => self.view_collections_panel(),
                    Kind::Sheet => self.view_sheet_panel(),
                    Kind::Properties => self.view_properties_panel(),
                },
            ),
            self.view_statusbar(),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let mut layers: Vec<Element<'_, Message>> = vec![main_ui.into()];

        // Menu suspenso flutuante: renderizado em camada sobreposta sem empurrar a toolbar ou o canvas
        if let Some(active) = self.open_menu {
            let backdrop = column![
                Space::with_height(MENU_H),
                mouse_area(Space::new(Length::Fill, Length::Fill))
                    .on_press(Message::ToggleMenu(None)),
            ]
            .width(Length::Fill)
            .height(Length::Fill);

            let dropdown_content =
                column![Space::with_height(MENU_H), self.view_dropdown_menu(active),]
                    .width(Length::Fill);

            let menu_overlay = stack![backdrop, dropdown_content,]
                .width(Length::Fill)
                .height(Length::Fill);

            layers.push(menu_overlay.into());
        }

        // Modal Sobre o RustSimu: camada modal elevada com escurecimento de fundo
        if self.show_about_modal {
            let about_card = container(
                column![
                    row![
                        crate::icons::view_icon(crate::icons::Icon::Bolt, 24, t.primary),
                        column![
                            text("RustSimu")
                                .size(18)
                                .font(Font {
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
                    text("Recursos e Conformidade Técnica:")
                        .size(12)
                        .font(Font {
                            weight: iced::font::Weight::Semibold,
                            ..UI
                        }),
                    text("• Suporte completo a arquivos CADe_SIMU (.cad) e esquemas industriais multipáginas do QElectroTech (.qet).\n• Exportação e importação em formatos modernos abertos (.json, .toml).\n• Conformidade com normas de numeração IEC 60947-1 e identificação IEC 81346.\n• Motor de simulação física em tempo real para comando, potência trifásica, motores, inversores e softstarters.\n• Arquitetura 100% nativa em Rust com interface reativa acelerada por GPU.")
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
                .width(540)
            )
            .style(theme::dropdown_menu)
            .padding(Padding::from([16.0, 20.0]));

            let modal_backdrop = mouse_area(
                container(Space::new(Length::Fill, Length::Fill))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.45,
                        ))),
                        ..Default::default()
                    })
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(Message::CloseAbout);

            let modal_overlay = stack![
                modal_backdrop,
                container(
                    column![
                        vertical_space(),
                        row![horizontal_space(), about_card, horizontal_space()]
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
            .height(Length::Fill);

            layers.push(modal_overlay.into());
        }

        Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_menubar(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        let brand = row![
            crate::icons::view_icon(crate::icons::Icon::Bolt, 16, t.primary),
            text("RustSimu").size(13).font(Font {
                weight: iced::font::Weight::Bold,
                ..UI
            }),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let menu_btn = |menu: ActiveMenu, label: &'static str| {
            let is_open = self.open_menu == Some(menu);
            let s = if is_open {
                theme::tool_active
            } else {
                theme::menu_item
            };
            button(text(label).size(12).font(UI))
                .style(s)
                .padding(Padding::from([4.0, 8.0]))
                .on_press(Message::ToggleMenu(Some(menu)))
        };

        let menus = row![
            menu_btn(ActiveMenu::Arquivo, "Arquivo"),
            menu_btn(ActiveMenu::Editar, "Editar"),
            menu_btn(ActiveMenu::Projeto, "Projeto"),
            menu_btn(ActiveMenu::Exibicao, "Exibição"),
            menu_btn(ActiveMenu::Configuracoes, "Configurações"),
            menu_btn(ActiveMenu::Ajuda, "Ajuda"),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        let file_name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Novo Projeto");

        let file_badge = text(file_name).size(11).font(MONO).color(t.text2);

        let mode_text = if self.sim_running {
            "● Simulação"
        } else {
            "● Edição"
        };
        let mode_pill = container(text(mode_text).size(10.5).font(MONO))
            .style(theme::mode_badge(self.sim_running))
            .padding(Padding::from([2.0, 6.0]));

        let menubar_row = row![brand, menus, horizontal_space(), file_badge, mode_pill,]
            .spacing(10)
            .align_y(Alignment::Center);

        let top_bar = container(menubar_row)
            .style(theme::bar)
            .height(MENU_H)
            .padding(Padding::from([0.0, 12.0]))
            .width(Length::Fill);

        top_bar.into()
    }

    fn view_dropdown_menu(&self, menu: ActiveMenu) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        let menu_item_btn = |icon_opt: Option<crate::icons::Icon>,
                             label: &'static str,
                             shortcut: &'static str,
                             msg: Message| {
            let left_part = row![
                if let Some(ic) = icon_opt {
                    container(crate::icons::view_icon(ic, 14, t.text2))
                        .width(16)
                        .height(16)
                        .center_x(16)
                        .center_y(16)
                } else {
                    container(Space::with_width(16).height(16))
                },
                text(label).size(12).font(UI),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            button(
                row![
                    left_part,
                    horizontal_space(),
                    text(shortcut).size(11).font(MONO).color(t.text2),
                ]
                .width(Length::Fill)
                .align_y(Alignment::Center),
            )
            .style(theme::menu_item)
            .padding(Padding::from([5.0, 12.0]))
            .width(Length::Fill)
            .on_press(msg)
        };

        let items: Element<'_, Message> = match menu {
            ActiveMenu::Arquivo => column![
                menu_item_btn(
                    Some(crate::icons::Icon::New),
                    "Novo Esquema",
                    "Ctrl+N",
                    Message::NewDocument
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Open),
                    "Abrir Esquema...",
                    "Ctrl+O",
                    Message::OpenFileDialog
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Save),
                    "Salvar",
                    "Ctrl+S",
                    Message::SaveFileDialog
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::SaveAs),
                    "Salvar Como...",
                    "Ctrl+Shift+S",
                    Message::SaveFileDialog
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Duplicate),
                    "Salvar Cópia...",
                    "Ctrl+Alt+S",
                    Message::SaveAll
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Close),
                    "Fechar Esquema",
                    "Ctrl+W",
                    Message::CloseDocument
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::ExportPdf),
                    "Exportar (PDF, SVG, CAD)...",
                    "Ctrl+Shift+E",
                    Message::ExportDialog
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Print),
                    "Imprimir...",
                    "Ctrl+P",
                    Message::PrintDocument
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Close),
                    "Fechar Menu",
                    "ESC",
                    Message::CloseMenu
                ),
            ]
            .spacing(2)
            .width(280)
            .into(),
            ActiveMenu::Editar => column![
                menu_item_btn(
                    Some(crate::icons::Icon::Undo),
                    "Desfazer",
                    "Ctrl+Z",
                    Message::Undo
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Redo),
                    "Refazer",
                    "Ctrl+Shift+Z",
                    Message::Redo
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Cut),
                    "Cortar",
                    "Ctrl+X",
                    Message::Cut
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Copy),
                    "Copiar",
                    "Ctrl+C",
                    Message::Copy
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Paste),
                    "Colar",
                    "Ctrl+V",
                    Message::Paste
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Paste),
                    "Colar na Posição Original",
                    "Ctrl+Shift+V",
                    Message::PasteAtOriginalPos
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Duplicate),
                    "Duplicar",
                    "Ctrl+D",
                    Message::Duplicate
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Bolt),
                    "Inserir o Último Elemento",
                    "A",
                    Message::InsertLastElement
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Palette),
                    "Colorir Elemento...",
                    "",
                    Message::ColorElement
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Select),
                    "Selecionar Tudo",
                    "Ctrl+A",
                    Message::SelectAll
                ),
                menu_item_btn(
                    None,
                    "Anular a Seleção",
                    "Ctrl+Shift+A",
                    Message::DeselectAll
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::MirrorH),
                    "Inverter Seleção",
                    "Ctrl+I",
                    Message::InvertSelection
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Copy),
                    "Copiar Info do Elemento",
                    "Ctrl+Alt+C",
                    Message::CopyElementInfo
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Paste),
                    "Colar Info do Elemento",
                    "Ctrl+Alt+V",
                    Message::PasteElementInfo
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Delete),
                    "Deletar",
                    "Del",
                    Message::DeleteSelected
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::RotateCw),
                    "Rotacionar 90°",
                    "R",
                    Message::RotateClockwise
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::RotateCcw),
                    "Rotacionar como Grupo",
                    "Shift+R",
                    Message::RotateCounterClockwise
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::MirrorH),
                    "Espelhar Horizontal",
                    "M",
                    Message::MirrorHorizontal
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::MirrorV),
                    "Espelhar Vertical",
                    "F",
                    Message::MirrorVertical
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::New),
                    "Propriedades da Página",
                    "Ctrl+Shift+Enter",
                    Message::SelectPropTab(0)
                ),
            ]
            .spacing(2)
            .width(290)
            .into(),
            ActiveMenu::Projeto => {
                let autoconn_str = if self.auto_connect {
                    "[X] Criação Automática de Condutores"
                } else {
                    "[  ] Criação Automática de Condutores"
                };
                let autodisc_str = if self.auto_disconnect {
                    "[X] Desconexão Automática de Condutores"
                } else {
                    "[  ] Desconexão Automática de Condutores"
                };
                column![
                    menu_item_btn(
                        Some(crate::icons::Icon::Copy),
                        "Propriedades do Projeto",
                        "",
                        Message::SelectPropTab(0)
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::WireL),
                        autoconn_str,
                        "",
                        Message::ToggleAutoConnect
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::DeleteWire),
                        autodisc_str,
                        "",
                        Message::ToggleAutoDisconnect
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::AddAfter),
                        "Adicionar Página",
                        "Ctrl+T",
                        Message::AddPageAfter(self.active_page_index)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Delete),
                        "Deletar esta Página",
                        "Ctrl+Shift+Bksp",
                        Message::DeletePage(self.active_page_index)
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::ChevronRight),
                        "Próxima Folha",
                        "Ctrl+PgDn",
                        Message::SelectPage(self.active_page_index + 1)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::ChevronLeft),
                        "Folha Anterior",
                        "Ctrl+PgUp",
                        Message::SelectPage(self.active_page_index.saturating_sub(1))
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Pencil),
                        "Renomear Folha Atual",
                        "",
                        Message::StartEditingPageName(self.active_page_index)
                    ),
                ]
                .spacing(2)
                .width(310)
                .into()
            }
            ActiveMenu::Exibicao => {
                let snap_str = if self.snap_to_grid {
                    "[X] Ajustar à Grelha (Snap)"
                } else {
                    "[  ] Ajustar à Grelha (Snap)"
                };
                column![
                    menu_item_btn(
                        Some(crate::icons::Icon::Folder),
                        "Mostrar Projetos",
                        "",
                        Message::SelectLeftDockTab(1)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Select),
                        "Selecionar (Ponteiro)",
                        "",
                        Message::SelectTool(EditorTool::Select)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Pan),
                        "Mover (Pan)",
                        "",
                        Message::SelectTool(EditorTool::Pan)
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::Grid),
                        "Mostrar a Grelha",
                        "",
                        Message::ToggleGrid(!self.show_grid)
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Magnet),
                        snap_str,
                        "Ctrl+Shift+G",
                        Message::ToggleSnap
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Text),
                        "Visibilidade dos Textos",
                        "",
                        Message::ToggleShowTexts
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::Theme),
                        "Alternar Tema",
                        "Ctrl+Shift+T",
                        Message::CycleTheme
                    ),
                    horizontal_rule(1),
                    menu_item_btn(
                        Some(crate::icons::Icon::ZoomIn),
                        "Ampliar (Zoom In)",
                        "Ctrl++",
                        Message::ZoomIn
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::ZoomOut),
                        "Reduzir (Zoom Out)",
                        "Ctrl+-",
                        Message::ZoomOut
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::ZoomReset),
                        "Zoom do Conteúdo",
                        "Ctrl+8",
                        Message::ZoomContent
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::FitToPage),
                        "Ajustar à Vista",
                        "Ctrl+9",
                        Message::FitToPage
                    ),
                    menu_item_btn(
                        Some(crate::icons::Icon::ZoomReset),
                        "Repor Zoom (100%)",
                        "Ctrl+0",
                        Message::ZoomReset
                    ),
                ]
                .spacing(2)
                .width(290)
                .into()
            }
            ActiveMenu::Configuracoes => column![
                menu_item_btn(
                    Some(crate::icons::Icon::Grid),
                    "Grade Fina (10 pts/cm)",
                    "",
                    Message::SetGridDensity(crate::model::GridDensity::Fine)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Grid),
                    "Grade Média (5 pts/cm)",
                    "",
                    Message::SetGridDensity(crate::model::GridDensity::Medium)
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Grid),
                    "Grade Larga (2.5 pts/cm)",
                    "",
                    Message::SetGridDensity(crate::model::GridDensity::Coarse)
                ),
            ]
            .spacing(2)
            .width(270)
            .into(),
            ActiveMenu::Ajuda => column![
                menu_item_btn(
                    Some(crate::icons::Icon::Bolt),
                    "Sobre o RustSimu",
                    "",
                    Message::ShowAbout
                ),
                menu_item_btn(
                    Some(crate::icons::Icon::Open),
                    "Carregar Exemplo: Partida Direta",
                    "",
                    Message::LoadPresetDol
                ),
                horizontal_rule(1),
                menu_item_btn(
                    Some(crate::icons::Icon::Close),
                    "Fechar Menu",
                    "ESC",
                    Message::CloseMenu
                ),
            ]
            .spacing(2)
            .width(270)
            .into(),
        };

        let left_padding = match menu {
            ActiveMenu::Arquivo => 85.0,
            ActiveMenu::Editar => 145.0,
            ActiveMenu::Projeto => 195.0,
            ActiveMenu::Exibicao => 250.0,
            ActiveMenu::Configuracoes => 315.0,
            ActiveMenu::Ajuda => 415.0,
        };

        row![
            Space::with_width(left_padding),
            container(items)
                .style(theme::dropdown_menu)
                .padding(Padding::from([6.0, 8.0])),
            horizontal_space(),
        ]
        .width(Length::Fill)
        .into()
    }

    fn view_toolbar(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        let icon_btn = |icon: crate::icons::Icon, tip: &'static str, msg: Message| {
            app_tooltip(
                button(
                    container(crate::icons::view_icon(icon, 16, t.text))
                        .width(20)
                        .height(20)
                        .center_x(20)
                        .center_y(20),
                )
                .style(theme::tool)
                .padding(Padding::from([3.0, 5.0]))
                .on_press(msg),
                tip,
                tooltip::Position::Bottom,
            )
        };

        let icon_toggle =
            |icon: crate::icons::Icon, tip: &'static str, active: bool, msg: Message| {
                let s = if active {
                    theme::tool_active
                } else {
                    theme::tool
                };
                let col = if active { t.primary } else { t.text };
                app_tooltip(
                    button(
                        container(crate::icons::view_icon(icon, 16, col))
                            .width(20)
                            .height(20)
                            .center_x(20)
                            .center_y(20),
                    )
                    .style(s)
                    .padding(Padding::from([3.0, 5.0]))
                    .on_press(msg),
                    tip,
                    tooltip::Position::Bottom,
                )
            };

        let file_group = container(
            row![
                icon_btn(
                    crate::icons::Icon::New,
                    "Novo (Ctrl+N)",
                    Message::NewDocument
                ),
                icon_btn(
                    crate::icons::Icon::Open,
                    "Abrir (Ctrl+O)",
                    Message::OpenFileDialog
                ),
                icon_btn(
                    crate::icons::Icon::Save,
                    "Salvar (Ctrl+S)",
                    Message::SaveFileDialog
                ),
                icon_btn(
                    crate::icons::Icon::SaveAs,
                    "Salvar Como (Ctrl+Shift+S)",
                    Message::SaveFileDialog
                ),
                icon_btn(
                    crate::icons::Icon::Close,
                    "Fechar (Ctrl+W)",
                    Message::CloseDocument
                ),
                icon_btn(
                    crate::icons::Icon::Print,
                    "Imprimir (Ctrl+P)",
                    Message::PrintDocument
                ),
                icon_btn(
                    crate::icons::Icon::ExportPdf,
                    "Exportar em PDF (Ctrl+Shift+E)",
                    Message::ExportDialog
                ),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        )
        .style(theme::tool_group)
        .padding(Padding::from([2.0, 4.0]));

        let edit_group = container(
            row![
                icon_btn(crate::icons::Icon::Undo, "Desfazer (Ctrl+Z)", Message::Undo),
                icon_btn(
                    crate::icons::Icon::Redo,
                    "Refazer (Ctrl+Shift+Z)",
                    Message::Redo
                ),
                icon_btn(crate::icons::Icon::Cut, "Cortar (Ctrl+X)", Message::Cut),
                icon_btn(crate::icons::Icon::Copy, "Copiar (Ctrl+C)", Message::Copy),
                icon_btn(crate::icons::Icon::Paste, "Colar (Ctrl+V)", Message::Paste),
                icon_btn(
                    crate::icons::Icon::Duplicate,
                    "Duplicar (Ctrl+D)",
                    Message::Duplicate
                ),
                icon_btn(
                    crate::icons::Icon::Delete,
                    "Deletar (Del)",
                    Message::DeleteSelected
                ),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        )
        .style(theme::tool_group)
        .padding(Padding::from([2.0, 4.0]));

        let transform_group = container(
            row![
                icon_btn(
                    crate::icons::Icon::RotateCw,
                    "Rotacionar (R)",
                    Message::RotateClockwise
                ),
                icon_btn(
                    crate::icons::Icon::RotateCcw,
                    "Rotacionar como Grupo (Shift+R)",
                    Message::RotateCounterClockwise
                ),
                icon_btn(
                    crate::icons::Icon::MirrorH,
                    "Espelhar Horizontalmente (M)",
                    Message::MirrorHorizontal
                ),
                icon_btn(
                    crate::icons::Icon::MirrorV,
                    "Espelhar Verticalmente (F)",
                    Message::MirrorVertical
                ),
                icon_btn(
                    crate::icons::Icon::Palette,
                    "Colorir Elemento",
                    Message::ColorElement
                ),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        )
        .style(theme::tool_group)
        .padding(Padding::from([2.0, 4.0]));

        let tool_btn = |tool: EditorTool, icon: crate::icons::Icon, tip: &'static str| {
            let active = self.tool == tool;
            let s = if active {
                theme::tool_active
            } else {
                theme::tool
            };
            let col = if active { t.primary } else { t.text };
            app_tooltip(
                button(
                    container(crate::icons::view_icon(icon, 16, col))
                        .width(20)
                        .height(20)
                        .center_x(20)
                        .center_y(20),
                )
                .style(s)
                .padding(Padding::from([3.0, 5.0]))
                .on_press(Message::SelectTool(tool)),
                tip,
                tooltip::Position::Bottom,
            )
        };

        let draw_group = container(
            row![
                tool_btn(
                    EditorTool::Select,
                    crate::icons::Icon::Select,
                    "Selecionar (Ponteiro)"
                ),
                tool_btn(EditorTool::Pan, crate::icons::Icon::Pan, "Mover (Pan)"),
                tool_btn(
                    EditorTool::DrawWire(4000),
                    crate::icons::Icon::WireL,
                    "Cabo Fase L"
                ),
                tool_btn(
                    EditorTool::DrawWire(4009),
                    crate::icons::Icon::WireN,
                    "Cabo Neutro N"
                ),
                tool_btn(
                    EditorTool::DrawWire(4010),
                    crate::icons::Icon::WirePe,
                    "Cabo Terra PE"
                ),
                tool_btn(
                    EditorTool::DrawWire(4003),
                    crate::icons::Icon::WireDc,
                    "Cabo Contínuo CC+"
                ),
                tool_btn(
                    EditorTool::PlaceJunction,
                    crate::icons::Icon::Junction,
                    "Ponto de Junção"
                ),
                tool_btn(
                    EditorTool::Delete,
                    crate::icons::Icon::DeleteWire,
                    "Apagar Conexão"
                ),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        )
        .style(theme::tool_group)
        .padding(Padding::from([2.0, 4.0]));

        let view_group = container(
            row![
                icon_btn(
                    crate::icons::Icon::ZoomIn,
                    "Aproximar Zoom (Ctrl++)",
                    Message::ZoomIn
                ),
                icon_btn(
                    crate::icons::Icon::ZoomOut,
                    "Afastar Zoom (Ctrl+-)",
                    Message::ZoomOut
                ),
                icon_btn(
                    crate::icons::Icon::ZoomReset,
                    "Repor Zoom (Ctrl+0)",
                    Message::ZoomReset
                ),
                icon_btn(
                    crate::icons::Icon::FitToPage,
                    "Ajustar à Vista (Ctrl+9)",
                    Message::FitToPage
                ),
                icon_toggle(
                    crate::icons::Icon::Grid,
                    "Mostrar a Grelha",
                    self.show_grid,
                    Message::ToggleGrid(!self.show_grid)
                ),
                icon_toggle(
                    crate::icons::Icon::Magnet,
                    "Snap à Grelha",
                    self.snap_to_grid,
                    Message::ToggleSnap
                ),
                pick_list(
                    &crate::model::GridDensity::ALL[..],
                    Some(self.grid_density),
                    Message::SetGridDensity,
                )
                .text_size(11)
                .padding(Padding::from([3.0, 6.0])),
                icon_toggle(
                    crate::icons::Icon::Text,
                    "Textos (Ligado / Desligado)",
                    self.show_texts,
                    Message::ToggleShowTexts
                ),
            ]
            .spacing(3)
            .align_y(Alignment::Center),
        )
        .style(theme::tool_group)
        .padding(Padding::from([2.0, 4.0]));

        let theme_group = container(app_tooltip(
            button(
                container(crate::icons::view_icon(
                    crate::icons::Icon::Theme,
                    16,
                    t.text,
                ))
                .width(20)
                .height(20)
                .center_x(20)
                .center_y(20),
            )
            .style(theme::tool)
            .padding(Padding::from([3.0, 5.0]))
            .on_press(Message::CycleTheme),
            "Alternar Tema (Sistema / Claro / Escuro)",
            tooltip::Position::Bottom,
        ))
        .style(theme::tool_group)
        .padding(Padding::from([2.0, 4.0]));

        let transport_group = container(
            row![
                app_tooltip(
                    button(
                        container(crate::icons::view_icon(
                            crate::icons::Icon::Play,
                            14,
                            t.on_primary
                        ))
                        .width(20)
                        .height(20)
                        .center_x(20)
                        .center_y(20),
                    )
                    .style(theme::pill_play(self.sim_running))
                    .padding(Padding::from([3.0, 5.0]))
                    .on_press(Message::ToggleSimulation),
                    if self.sim_running {
                        "Pausar/Parar Simulação"
                    } else {
                        "Iniciar Simulação Elétrica"
                    },
                    tooltip::Position::Bottom,
                ),
                app_tooltip(
                    button(
                        container(crate::icons::view_icon(
                            crate::icons::Icon::Stop,
                            13,
                            t.on_danger
                        ))
                        .width(20)
                        .height(20)
                        .center_x(20)
                        .center_y(20),
                    )
                    .style(theme::pill_stop(self.sim_running))
                    .padding(Padding::from([3.0, 5.0]))
                    .on_press(Message::StopSimulation),
                    "Parar Simulação e Resetar",
                    tooltip::Position::Bottom,
                ),
                app_tooltip(
                    button(
                        container(crate::icons::view_icon(
                            crate::icons::Icon::Step,
                            15,
                            t.text
                        ))
                        .width(20)
                        .height(20)
                        .center_x(20)
                        .center_y(20),
                    )
                    .style(theme::pill_ghost)
                    .padding(Padding::from([3.0, 5.0]))
                    .on_press(Message::SimTick),
                    "Avançar Passo de Simulação",
                    tooltip::Position::Bottom,
                ),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .style(theme::transport)
        .padding(Padding::from([2.0, 6.0]));

        let left_tools = scrollable(
            row![
                file_group,
                edit_group,
                transform_group,
                draw_group,
                view_group,
                theme_group,
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new()
                .width(3.0)
                .scroller_width(3.0)
                .margin(0.0),
        ))
        .width(Length::Fill);

        container(
            row![left_tools, transport_group,]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .style(theme::bar)
        .height(TOOL_H)
        .padding(Padding::from([0.0, 10.0]))
        .width(Length::Fill)
        .into()
    }

    fn view_collections_panel(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        if self.left_dock_collapsed {
            return container(
                button(crate::icons::view_icon(
                    crate::icons::Icon::ChevronRight,
                    14,
                    t.text,
                ))
                .style(theme::tool)
                .padding(Padding::from([8.0, 5.0]))
                .on_press(Message::ToggleLeftDockCollapse),
            )
            .align_x(iced::alignment::Horizontal::Center)
            .style(theme::bar)
            .width(Length::Fixed(28.0))
            .height(Length::Fill)
            .into();
        }

        let header = container(
            row![
                text("Elementos e Projetos").size(12).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..UI
                }),
                horizontal_space(),
                button(crate::icons::view_icon(
                    crate::icons::Icon::ChevronLeft,
                    12,
                    t.text
                ))
                .style(theme::tool)
                .padding(Padding::from([3.0, 6.0]))
                .on_press(Message::ToggleLeftDockCollapse),
            ]
            .align_y(Alignment::Center),
        )
        .style(theme::bar)
        .padding(Padding::from([5.0, 10.0]))
        .width(Length::Fill);

        let dock_tabs = container(
            row![
                button(
                    row![
                        crate::icons::view_icon(
                            crate::icons::Icon::Bolt,
                            13,
                            if self.left_dock_tab == 0 {
                                t.primary
                            } else {
                                t.text
                            },
                        ),
                        text("Elementos").size(11).font(UI),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                )
                .style(if self.left_dock_tab == 0 {
                    theme::tool_active
                } else {
                    theme::tool
                })
                .padding(Padding::from([4.0, 8.0]))
                .width(Length::Fill)
                .on_press(Message::SelectLeftDockTab(0)),
                button(
                    row![
                        crate::icons::view_icon(
                            crate::icons::Icon::Folder,
                            13,
                            if self.left_dock_tab == 1 {
                                t.primary
                            } else {
                                t.text
                            },
                        ),
                        text("Projetos").size(11).font(UI),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                )
                .style(if self.left_dock_tab == 1 {
                    theme::tool_active
                } else {
                    theme::tool
                })
                .padding(Padding::from([4.0, 8.0]))
                .width(Length::Fill)
                .on_press(Message::SelectLeftDockTab(1)),
            ]
            .spacing(4)
            .width(Length::Fill),
        )
        .style(theme::bar)
        .padding(Padding::from([4.0, 8.0]))
        .width(Length::Fill);

        let content: Element<'_, Message> = match self.left_dock_tab {
            0 => {
                // Barra de busca
                let search_box = container(
                    text_input(
                        "Filtrar símbolos (ex: disjuntor, motor)...",
                        &self.collection_search,
                    )
                    .on_input(Message::CollectionSearchChanged)
                    .padding(Padding::from([6.0, 8.0]))
                    .size(12)
                    .style(theme::field),
                )
                .padding(Padding::from([8.0, 10.0]));

                // Lista de símbolos organizados por categoria
                let query = self.collection_search.trim().to_lowercase();
                let all_symbols = symbol_catalog();

                let mut tree_col = column![].spacing(6).padding(Padding::from([0.0, 10.0]));

                // Atalhos de cabeamento rápido sempre visíveis
                let wire_shortcuts = row![
                    button(text("+ Fase L").size(11))
                        .style(theme::tool)
                        .padding(Padding::from([2.0, 6.0]))
                        .on_press(Message::SelectTool(EditorTool::DrawWire(4000))),
                    button(text("+ Neutro N").size(11))
                        .style(theme::tool)
                        .padding(Padding::from([2.0, 6.0]))
                        .on_press(Message::SelectTool(EditorTool::DrawWire(4009))),
                    button(text("+ Terra").size(11))
                        .style(theme::tool)
                        .padding(Padding::from([2.0, 6.0]))
                        .on_press(Message::SelectTool(EditorTool::DrawWire(4010))),
                    button(text("+ Nó").size(11))
                        .style(theme::tool)
                        .padding(Padding::from([2.0, 6.0]))
                        .on_press(Message::SelectTool(EditorTool::PlaceJunction)),
                ]
                .spacing(4)
                .align_y(Alignment::Center);

                tree_col = tree_col.push(wire_shortcuts).push(horizontal_rule(1));

                for cat in SymbolCategory::ALL {
                    if cat == SymbolCategory::WiresConnections {
                        continue;
                    }
                    let cat_syms: Vec<_> = all_symbols
                        .iter()
                        .filter(|s| s.category == cat)
                        .filter(|s| {
                            if query.is_empty() {
                                true
                            } else {
                                s.name.to_lowercase().contains(&query)
                                    || s.default_tag.to_lowercase().contains(&query)
                                    || s.short_label.to_lowercase().contains(&query)
                                    || cat.title().to_lowercase().contains(&query)
                            }
                        })
                        .collect();

                    if cat_syms.is_empty() {
                        continue;
                    }

                    let cat_header = container(
                        text(cat.title())
                            .size(11)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            })
                            .color(t.primary),
                    )
                    .padding(Padding::from([4.0, 2.0]));

                    tree_col = tree_col.push(cat_header);

                    for sym in cat_syms {
                        let is_selected = self.selected_collection_sym == Some(sym.tipo);
                        let pin_count = sym.pins.len();

                        let item_row = container(
                            row![
                                container(
                                    text(format!("[{}]", sym.default_tag)).size(10.5).font(MONO)
                                )
                                .style(move |theme| {
                                    let tok = tokens(theme);
                                    container::Style {
                                        text_color: Some(tok.text),
                                        background: Some(iced::Background::Color(tok.s3)),
                                        border: iced::Border {
                                            color: tok.border,
                                            width: 1.0,
                                            radius: R.into(),
                                        },
                                        shadow: iced::Shadow::default(),
                                    }
                                })
                                .padding(Padding::from([1.0, 4.0])),
                                text(sym.name).size(12).font(UI).width(Length::Fill),
                                text(format!("{pin_count}b"))
                                    .size(10.5)
                                    .font(MONO)
                                    .color(t.text2),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                        )
                        .style(move |theme| {
                            let tok = tokens(theme);
                            if is_selected {
                                container::Style {
                                    background: Some(iced::Background::Color(tok.sel_bg)),
                                    text_color: Some(tok.sel_text),
                                    border: iced::Border {
                                        color: tok.sel_border,
                                        width: 1.0,
                                        radius: R.into(),
                                    },
                                    shadow: iced::Shadow::default(),
                                }
                            } else {
                                container::Style {
                                    background: Some(iced::Background::Color(Color::TRANSPARENT)),
                                    text_color: Some(tok.text),
                                    border: iced::Border {
                                        color: Color::TRANSPARENT,
                                        width: 1.0,
                                        radius: R.into(),
                                    },
                                    shadow: iced::Shadow::default(),
                                }
                            }
                        })
                        .padding(Padding::from([4.0, 6.0]))
                        .width(Length::Fill);

                        let item_area = mouse_area(item_row)
                            .interaction(iced::mouse::Interaction::Pointer)
                            .on_press(Message::StartSymbolDrag(sym.tipo))
                            .on_release(Message::SelectCollectionSymbol(sym.tipo));

                        tree_col = tree_col.push(item_area);
                    }
                }

                let tree_scroll = container(scrollable(tree_col)).height(Length::Fill);

                // Preview box na parte inferior do painel esquerdo
                let preview_content = if let Some(tipo) = self.selected_collection_sym {
                    if let Some(sym) = find_symbol(tipo) {
                        let is_dark = self.is_dark();
                        let svg_content =
                            crate::symbols::get_themed_symbol_svg_string(&sym, is_dark);
                        let svg_preview: Element<'_, Message> = container(
                            svg(svg::Handle::from_memory(svg_content.into_bytes()))
                                .width(Length::Fixed(80.0))
                                .height(Length::Fixed(60.0)),
                        )
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .into();

                        column![
                            row![
                                text(sym.name)
                                    .size(11.5)
                                    .font(Font {
                                        weight: iced::font::Weight::Semibold,
                                        ..UI
                                    })
                                    .width(Length::Fill),
                                text(format!("{} bornes", sym.pins.len()))
                                    .size(10.5)
                                    .font(MONO)
                                    .color(t.text2),
                            ]
                            .align_y(Alignment::Center),
                            svg_preview,
                            button(text("Inserir no Esquema").size(11.5).font(UI))
                                .style(theme::primary)
                                .padding(Padding::from([4.0, 10.0]))
                                .width(Length::Fill)
                                .on_press(Message::InsertSelectedCollectionSymbol),
                        ]
                        .spacing(4)
                        .align_x(Alignment::Center)
                    } else {
                        column![text("Símbolo não encontrado").size(12)]
                    }
                } else {
                    column![text("Selecione um símbolo acima").size(11.5).color(t.text2),]
                        .align_x(Alignment::Center)
                };

                let preview_dock = container(preview_content)
                    .style(theme::preview_box)
                    .padding(Padding::from([6.0, 10.0]))
                    .width(Length::Fill)
                    .height(130);

                column![search_box, tree_scroll, preview_dock]
                    .height(Length::Fill)
                    .into()
            }
            _ => {
                // Aba Projetos
                let mut proj_col = column![].spacing(6).padding(Padding::from([8.0, 10.0]));

                proj_col = proj_col.push(
                    text("Folha Ativa:")
                        .size(11.5)
                        .font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        })
                        .color(t.primary),
                );

                for (idx, p) in self.pages.iter().enumerate() {
                    let is_cur = idx == self.active_page_index;
                    let p_btn = button(
                        row![
                            text(format!("{}. {}", idx + 1, p.name)).size(11.5).font(UI),
                            horizontal_space(),
                            if is_cur {
                                text("● Ativa").size(10.5).color(t.primary)
                            } else {
                                text("").size(10.5)
                            },
                        ]
                        .align_y(Alignment::Center),
                    )
                    .style(if is_cur {
                        theme::tool_active
                    } else {
                        theme::tool
                    })
                    .padding(Padding::from([4.0, 8.0]))
                    .width(Length::Fill)
                    .on_press(Message::SelectPage(idx));
                    proj_col = proj_col.push(p_btn);
                }

                proj_col = proj_col.push(horizontal_rule(1));

                proj_col = proj_col.push(
                    row![
                        text("Projetos Recentes:")
                            .size(11.5)
                            .font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            })
                            .color(t.primary),
                        horizontal_space(),
                        text(format!("{} arquivos", self.recent_files.len()))
                            .size(10.5)
                            .color(t.text2),
                    ]
                    .align_y(Alignment::Center),
                );

                let mut files_col = column![].spacing(4);
                if self.recent_files.is_empty() {
                    files_col = files_col.push(
                        text("Nenhum arquivo recente na pasta Projetos.")
                            .size(11)
                            .color(t.text2),
                    );
                } else {
                    for path_str in &self.recent_files {
                        let p = std::path::Path::new(path_str);
                        let file_name = p
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path_str.clone());
                        let ext = p
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("cad")
                            .to_uppercase();

                        let open_btn = button(
                            row![
                                container(text(ext).size(9.5).font(MONO))
                                    .padding(Padding::from([1.0, 4.0]))
                                    .style(theme::card),
                                text(file_name).size(11).font(UI).width(Length::Fill),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                        )
                        .style(theme::tool)
                        .padding(Padding::from([3.0, 6.0]))
                        .width(Length::Fill)
                        .on_press(Message::RecentProjectSelected(path_str.clone()));

                        let del_btn = button(crate::icons::view_icon(
                            crate::icons::Icon::Close,
                            11,
                            t.text2,
                        ))
                        .style(theme::tool)
                        .padding(Padding::from([3.0, 5.0]))
                        .on_press(Message::DeleteRecentProject(path_str.clone()));

                        files_col = files_col.push(
                            row![open_btn, del_btn]
                                .spacing(2)
                                .align_y(Alignment::Center),
                        );
                    }
                }

                let files_scroll = container(scrollable(files_col)).height(Length::Fill);
                column![proj_col, files_scroll].height(Length::Fill).into()
            }
        };

        column![header, content, dock_tabs]
            .height(Length::Fill)
            .into()
    }

    fn view_sheet_panel(&self) -> Element<'_, Message> {
        let title_line = self.view_canvas_title_line();
        let pages_bar = self.view_canvas_pages_bar();

        let is_dark = self.is_dark();

        let canvas_el = SchematicCanvas {
            doc: &self.doc,
            tool: self.tool,
            selected_id: self.selected_id,
            sim_running: self.sim_running,
            show_grid: self.show_grid,
            grid_density: self.grid_density,
            show_texts: self.show_texts,
            is_dark,
            auto_connect: self.auto_connect,
            auto_disconnect: self.auto_disconnect,
            zoom: self.zoom,
            pan: self.pan,
            placement_orientation: self.placement_orientation,
            dragging_symbol: self.dragging_symbol,
            is_blocked: self.open_menu.is_some() || self.show_about_modal,
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

    fn view_properties_panel(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        if self.right_dock_collapsed {
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
            let active = self.prop_tab == idx;
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
            row![tab_btn(0, "Propriedades"), tab_btn(1, "Referência Cruzada"),]
                .spacing(4)
                .width(Length::Fill),
        )
        .style(theme::bar)
        .width(Length::Fill);

        // Corpo do painel de propriedades
        let panel_body: Element<'_, Message> = match self.prop_tab {
            0 => {
                // Aba Propriedades
                let mut body_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));

                if let Some(sel_id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == sel_id) {
                        let title =
                            find_symbol(item.tipo)
                                .map(|s| s.name)
                                .unwrap_or(if item.is_wire() {
                                    "Cabo Condutor"
                                } else {
                                    "Elemento"
                                });

                        body_col = body_col.push(
                            text(format!("#{sel_id} · {title}")).size(13).font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            }),
                        );

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
                                if let Some(parent) = find_parent_apparatus(&self.doc.items, item) {
                                    let parent_name = find_symbol(parent.tipo)
                                        .map(|s| s.name)
                                        .unwrap_or("Aparelho");
                                    body_col = body_col.push(
                                        text(format!(
                                            "✓ Vinculado a '{}' ({})",
                                            parent.tag, parent_name
                                        ))
                                        .size(10.5)
                                        .color(Color::from_rgb(0.2, 0.8, 0.4)),
                                    );
                                } else {
                                    let parent_display = role.parent_display_name();
                                    let mut alert_col = column![
                                        text(format!("⚠️ Contato sem bobina: Nenhuma {} com a tag '{}' no esquema.", parent_display, item.tag))
                                            .size(10.5)
                                            .color(Color::from_rgb(0.95, 0.7, 0.2)),
                                    ]
                                    .spacing(4);

                                    let available =
                                        list_available_parent_tags(&self.doc.items, role);
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
                                if find_duplicate_apparatus(&self.doc.items, item) {
                                    let prefix = extract_tag_prefix(&item.tag);
                                    let suggested_tag =
                                        next_sequential_tag_for_role(&self.doc.items, role, prefix);
                                    let suggested_tag_clone = suggested_tag.clone();
                                    body_col = body_col.push(
                                        container(
                                            column![
                                                text(format!("⚠️ Tag duplicada: já existe outro aparelho primário com a tag '{}'", item.tag))
                                                    .size(10.5)
                                                    .color(Color::from_rgb(0.95, 0.7, 0.2)),
                                                button(
                                                    text(format!("⚡ Renomear para {suggested_tag}"))
                                                        .size(10.5)
                                                        .font(UI)
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
                                    let sub_contacts_count = self
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
                                            text(format!(
                                                "✓ Contatos associados no esquema: {} elemento(s)",
                                                sub_contacts_count
                                            ))
                                            .size(10.5)
                                            .color(Color::from_rgb(0.2, 0.8, 0.4)),
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
                                            &self.doc.items,
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
                                                            text(format!(
                                                                "⚠️ Sequência {} já em uso por outro contato de '{}'",
                                                                cur_seq, item.tag
                                                            ))
                                                            .size(10.5)
                                                            .color(Color::from_rgb(0.95, 0.7, 0.2)),
                                                            button(
                                                                text(format!(
                                                                    "⚡ Usar Sequência {} recomendada ({})",
                                                                    suggested_digit,
                                                                    kind.generate_pins(suggested_digit).join("-")
                                                                ))
                                                                .size(10.5)
                                                                .font(UI)
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
                                                    text(format!(
                                                        "✓ Sequência {} conforme IEC 60947",
                                                        cur_seq
                                                    ))
                                                    .size(10.5)
                                                    .color(Color::from_rgb(0.2, 0.8, 0.4)),
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
                            }

                            // Controles para Sinalizadores Luminosos / Lâmpadas
                            let is_lamp = item.is_lamp_or_signaling();

                            if is_lamp {
                                body_col = body_col.push(horizontal_rule(1));
                                body_col = body_col.push(
                                    text("Cor e Pulso do Sinaleiro Luminoso:").size(11.5).font(
                                        Font {
                                            weight: iced::font::Weight::Bold,
                                            ..UI
                                        },
                                    ),
                                );

                                let color_btn =
                                    |code: i32, label: &'static str, color_val: Color| {
                                        let is_sel = item.color == code;
                                        button(
                                            row![
                                                container(Space::with_width(8).height(8)).style(
                                                    move |_| container::Style {
                                                        background: Some(iced::Background::Color(
                                                            color_val
                                                        )),
                                                        border: iced::Border {
                                                            color: Color::BLACK,
                                                            width: 1.0,
                                                            radius: 2.0.into()
                                                        },
                                                        ..container::Style::default()
                                                    }
                                                ),
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
                                    color_btn(1, "Verde", Color::from_rgb(0.1, 0.8, 0.2)),
                                    color_btn(2, "Vermelho", Color::from_rgb(0.9, 0.1, 0.1)),
                                    color_btn(3, "Amarelo", Color::from_rgb(0.9, 0.8, 0.1)),
                                    color_btn(4, "Azul", Color::from_rgb(0.1, 0.4, 0.9)),
                                    color_btn(0, "Branco", Color::from_rgb(0.95, 0.95, 0.95)),
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
                        }
                    }
                } else {
                    // Sem elemento selecionado -> Propriedades da Folha (Cajetín)
                    body_col =
                        body_col.push(text("Legenda da Folha (Cajetín)").size(13).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..UI
                        }));
                    body_col = body_col.push(
                        column![
                            text("Título do Projeto:").size(11).color(t.text2),
                            text_input("Título", &self.doc.title_block.title)
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
                            text_input("Subtítulo", &self.doc.title_block.subtitle)
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
                            text_input("Autor", &self.doc.title_block.drawn_by)
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
                            "Dica:
• Clique em qualquer símbolo na folha para editar suas propriedades.
• Arraste com botão direito ou scroll para navegar.",
                        )
                        .size(11.5)
                        .color(t.text2),
                    );
                }

                container(scrollable(body_col)).height(Length::Fill).into()
            }
            1 => {
                // Aba Referência Cruzada (XRef)
                let mut xref_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));

                xref_col =
                    xref_col.push(text("Referências Cruzadas no Projeto").size(13).font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }));

                // Se houver elemento selecionado, exibe a referência contextual específica em destaque
                if let Some(sel_id) = self.selected_id {
                    if let Some(sel_item) = self.doc.items.iter().find(|i| i.id == sel_id) {
                        let clean_sel_tag = sel_item.tag.trim();
                        if !clean_sel_tag.is_empty() {
                            let rel_items: Vec<_> = self
                                .doc
                                .items
                                .iter()
                                .filter(|i| i.tag.trim() == clean_sel_tag)
                                .collect();
                            let ctx_card = container(
                                column![
                                    text(format!(
                                        "🔍 Referência do Elemento Selecionado: #{} ({})",
                                        sel_id, clean_sel_tag
                                    ))
                                    .size(12)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..UI
                                    })
                                    .color(t.primary),
                                    text(format!(
                                        "Total de dispositivos vinculados ao tag '{}': {}",
                                        clean_sel_tag,
                                        rel_items.len()
                                    ))
                                    .size(10.5)
                                    .color(t.text2),
                                    column(rel_items.iter().map(|item| {
                                        let sname = find_symbol(item.tipo)
                                            .map(|s| s.name)
                                            .unwrap_or("Dispositivo");
                                        let p_list = item
                                            .pins
                                            .iter()
                                            .filter(|p| !p.is_empty())
                                            .cloned()
                                            .collect::<Vec<_>>()
                                            .join("-");
                                        let status = if item.sim_active {
                                            "⚡ Ativo"
                                        } else {
                                            "○ Desligado"
                                        };
                                        text(format!(
                                            "• #{} {}: [{}] ({})",
                                            item.id, sname, p_list, status
                                        ))
                                        .size(11)
                                        .font(MONO)
                                        .color(if item.id == sel_id { t.primary } else { t.text })
                                        .into()
                                    }))
                                    .spacing(2),
                                ]
                                .spacing(4),
                            )
                            .style(theme::card)
                            .padding(Padding::from([6.0, 8.0]))
                            .width(Length::Fill);

                            xref_col = xref_col.push(ctx_card).push(horizontal_rule(1));
                        }
                    }
                }

                // Agrupa itens por tag para visualização global do projeto
                let mut tags_seen: Vec<String> = Vec::new();
                for item in &self.doc.items {
                    let clean_tag = item.tag.trim();
                    if !clean_tag.is_empty() && !tags_seen.contains(&clean_tag.to_string()) {
                        tags_seen.push(clean_tag.to_string());
                    }
                }
                tags_seen.sort();

                if tags_seen.is_empty() {
                    xref_col = xref_col.push(
                        text("Nenhum elemento com etiqueta definido.")
                            .size(12)
                            .color(t.text2),
                    );
                } else {
                    for tag in tags_seen {
                        let matching: Vec<_> = self
                            .doc
                            .items
                            .iter()
                            .filter(|i| i.tag.trim() == tag)
                            .collect();

                        if matching.len() > 1
                            || matching
                                .iter()
                                .any(|i| matches!(i.tipo, 9000..=9008 | 6000..=6009 | 1000..=1002))
                        {
                            xref_col = xref_col.push(
                                container(
                                    column![
                                        row![
                                            text(format!("Etiqueta: {tag}"))
                                                .size(12)
                                                .font(Font {
                                                    weight: iced::font::Weight::Bold,
                                                    ..MONO
                                                })
                                                .color(t.primary),
                                            horizontal_space(),
                                            text(format!("{} instâncias", matching.len()))
                                                .size(10.5)
                                                .font(MONO)
                                                .color(t.text2),
                                        ]
                                        .align_y(Alignment::Center),
                                        column(matching.iter().map(|item| {
                                            let sname = find_symbol(item.tipo)
                                                .map(|s| s.short_label)
                                                .unwrap_or("Item");
                                            let bornes = if item.pins.is_empty() {
                                                String::new()
                                            } else {
                                                format!("· bornes: {}", item.pins.join("-"))
                                            };
                                            text(format!(
                                                "• #{}: {} (X:{}, Y:{}) {}",
                                                item.id, sname, item.x1, item.y1, bornes
                                            ))
                                            .size(11)
                                            .font(MONO)
                                            .color(t.text)
                                            .into()
                                        }))
                                        .spacing(2),
                                    ]
                                    .spacing(4),
                                )
                                .style(theme::card)
                                .padding(Padding::from([6.0, 8.0]))
                                .width(Length::Fill),
                            );
                        }
                    }
                }

                container(scrollable(xref_col)).height(Length::Fill).into()
            }
            _ => {
                // Aba Geral
                let mut geral_col = column![].spacing(8).padding(Padding::from([10.0, 12.0]));

                geral_col =
                    geral_col.push(text("Geometria & Propriedades Gerais").size(13).font(Font {
                        weight: iced::font::Weight::Bold,
                        ..UI
                    }));

                if let Some(sel_id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter().find(|i| i.id == sel_id) {
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
                                        Color::WHITE
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
                                    text(format!("{} itens", self.doc.items.len()))
                                        .size(11.5)
                                        .font(MONO),
                                ],
                                row![
                                    text("Snap de Grade:")
                                        .size(11)
                                        .color(t.text2)
                                        .width(Length::Fixed(110.0)),
                                    text(if self.show_grid {
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
        let sim_status_text = if self.sim_running {
            "Simulador ativo (80ms)"
        } else {
            "Simulador parado"
        };
        let led_color = if self.sim_running {
            t.primary
        } else {
            t.border_strong
        };
        let dock_foot = container(
            row![
                text("●").size(14).color(led_color),
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

    fn view_canvas_title_line(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        let doc_name = if !self.file_path.is_empty() {
            std::path::Path::new(&self.file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.file_path.clone())
        } else if let Some(ref qp) = self.qet_project {
            let cur = self.active_page_index;
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
            if self.is_modified {
                container(text("Modificado").size(10).color(t.danger))
                    .padding(Padding::from([2.0, 6.0]))
                    .style(theme::card)
            } else {
                container(text("Salvo").size(10).color(t.primary))
                    .padding(Padding::from([2.0, 6.0]))
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

    fn view_canvas_pages_bar(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());
        let total_pages = self.pages.len();

        let mut tabs_row = row![].spacing(4).align_y(Alignment::Center);

        for (idx, page) in self.pages.iter().enumerate() {
            let is_active = idx == self.active_page_index;
            let is_editing = is_active && self.editing_page_name.is_some();

            let tab_content: Element<'_, Message> = if is_editing {
                let current_input = self.editing_page_name.as_deref().unwrap_or(&page.name);
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
                .style(move |theme: &Theme| {
                    let mut s = container::transparent(theme);
                    if is_active {
                        s.border = Border {
                            color: t.primary,
                            width: 1.0,
                            radius: 4.0.into(),
                        };
                    }
                    s
                });

            tabs_row = tabs_row.push(tab_box);
        }

        // Atalhos de ponta: [◀+] e [+▶]
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
            .on_press(Message::AddPageBefore(self.active_page_index)),
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
            .on_press(Message::AddPageAfter(self.active_page_index)),
            "Inserir página depois da atual",
            tooltip::Position::Bottom,
        );

        let nav_controls = row![add_before_btn, add_after_btn]
            .spacing(3)
            .align_y(Alignment::Center);

        let context_banner: Option<Element<'_, Message>> =
            if let Some(target_idx) = self.page_context_menu {
                if target_idx < total_pages {
                    let p_name = &self.pages[target_idx].name;
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

    fn view_statusbar(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());

        let sep = || text("│").size(11).color(t.border);

        let snap_label = if self.show_grid {
            "snap ativo"
        } else {
            "snap desligado"
        };
        let zoom_pct = (self.zoom * 31.25).round() as i32;

        container(
            row![
                row![
                    text("Coordenadas:").size(11).color(t.text2),
                    text(format!(
                        "X: {:3}, Y: {:3}",
                        self.cursor_world.0, self.cursor_world.1
                    ))
                    .size(11)
                    .font(MONO)
                    .color(t.text),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
                sep(),
                row![
                    text("Grelha:").size(11).color(t.text2),
                    text(format!("5×5 ({snap_label})"))
                        .size(11)
                        .font(MONO)
                        .color(t.text),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
                sep(),
                text("Folha: DIN A4 (1040 × 720 px)")
                    .size(11)
                    .color(t.text2),
                sep(),
                text(format!("● {}", self.status_msg))
                    .size(11)
                    .color(if self.sim_running { t.primary } else { t.text2 })
                    .width(Length::Fill),
                sep(),
                row![
                    text("Zoom:").size(11).color(t.text2),
                    text(format!("{zoom_pct}%"))
                        .size(11)
                        .font(MONO)
                        .color(t.text),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
                sep(),
                text(self.save_format.extension().to_uppercase())
                    .size(11)
                    .font(MONO)
                    .color(t.text2),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .style(theme::bar)
        .height(STATUS_H)
        .padding(Padding::from([0.0, 10.0]))
        .width(Length::Fill)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{parse_cad_str, serialize_cad_str};
    use crate::model::Potential;

    #[test]
    fn test_cad_roundtrip_dol_starter() {
        let original = build_dol_starter_preset();
        let cad_text = serialize_cad_str(&original);
        assert!(cad_text.starts_with("CADe_SIMU*"));
        assert!(cad_text.contains("$$$"));
        assert!(cad_text.ends_with("***$$$&&&***$$$&&&"));

        let parsed = parse_cad_str(&cad_text).expect("Falha ao fazer parse do .cad gerado");
        assert_eq!(parsed.items.len(), original.items.len());
        assert_eq!(parsed.items[0].tipo, 3005);
        assert_eq!(parsed.items[1].tag, "-Q1");
        assert_eq!(parsed.items[4].tipo, 1000);
        assert_eq!(parsed.items[4].pins[0], "U1");
        assert_eq!(parsed.title_block.title, "Partida Direta Trifasica");
    }

    #[test]
    fn test_dol_starter_simulation_self_holding_and_thermal_trip() {
        let mut doc = build_dol_starter_preset();
        step_simulation(&mut doc, 50);

        // Inicialmente o motor -M1 está parado
        let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(!m1.sim_active);
        assert_eq!(m1.sim_motor_dir, 0);

        // 1) Fecha o disjuntor tripolar -Q1 (bi-estável)
        let q1_id = doc.items.iter().find(|i| i.tag == "-Q1").unwrap().id;
        press_component_on_sim(&mut doc, q1_id);
        release_component_on_sim(&mut doc, q1_id);
        step_simulation(&mut doc, 50);

        // Motor ainda parado porque -KM1 está aberto
        let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(!m1.sim_active);

        // 2) Pressiona o botão de partida -S1 (NA, 8000)
        let s1_id = doc.items.iter().find(|i| i.tag == "-S1").unwrap().id;
        press_component_on_sim(&mut doc, s1_id);
        step_simulation(&mut doc, 50);

        // Bobina -KM1 energiza, fecha contato de selo -KM1 (13-14), fecha contator -KM1 e gira -M1 no sentido horário!
        let km1_coil = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "-KM1")
            .unwrap();
        assert!(km1_coil.sim_active, "Bobina -KM1 deveria estar energizada");

        let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(m1.sim_active, "Motor -M1 deveria estar girando");
        assert_eq!(m1.sim_motor_dir, 1);

        // 3) Solta o botão pulsador -S1 -> -S1 retorna automaticamente ao estado original (aberto),
        //    e o motor DEVE permanecer ligado pelo contato de selo 13-14 de -KM1!
        release_component_on_sim(&mut doc, s1_id);
        step_simulation(&mut doc, 50);
        let s1 = doc.items.iter().find(|i| i.id == s1_id).unwrap();
        assert!(
            !s1.sim_manual_latched && !s1.sim_active,
            "Botão -S1 deve retornar ao estado original quando solto"
        );

        let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(
            m1.sim_active,
            "Motor -M1 deve permanecer ligado via contato de selo -KM1 (13-14)"
        );

        // 4) Atua o relé térmico -F1 -> Abre o contato 95-96 e desliga a bobina -KM1 e o motor -M1!
        let f1_id = doc
            .items
            .iter()
            .find(|i| i.tipo == 6007 && i.tag == "-F1")
            .unwrap()
            .id;
        press_component_on_sim(&mut doc, f1_id);
        release_component_on_sim(&mut doc, f1_id);
        step_simulation(&mut doc, 50);

        let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(
            !m1.sim_active,
            "Motor -M1 deve desligar imediatamente após atuação do relé térmico -F1"
        );
    }

    #[test]
    fn test_momentary_buttons_press_and_release_all_4_types() {
        let mut doc = SchematicDocument::default();
        // Instancia os 4 tipos de botões pulsadores: 8000 (NO), 8001 (NC), 8002 (NO-NC), 8003 (CON)
        // mais uma chave seletora retentiva 8004 (Switch) para comparar.
        for (idx, tipo) in [8000, 8001, 8002, 8003, 8008].into_iter().enumerate() {
            let sym = find_symbol(tipo).unwrap_or_else(|| panic!("Símbolo {tipo} deve existir"));
            let mut item = sym.instantiate((idx + 1) as i32, (idx as i32) * 30, 30, 0);
            item.tag = format!("-S{}", idx + 1);
            doc.items.push(item);
        }

        // Ao pressionar e soltar os 4 botões (8000, 8001, 8002, 8003), todos voltam para false!
        for id in 1..=4 {
            press_component_on_sim(&mut doc, id);
            step_simulation(&mut doc, 10);
            let it = doc.items.iter().find(|i| i.id == id).unwrap();
            assert!(
                it.sim_manual_latched && it.sim_active,
                "Botão tipo {} deve ativar enquanto pressionado",
                it.tipo
            );

            release_component_on_sim(&mut doc, id);
            step_simulation(&mut doc, 10);
            let it = doc.items.iter().find(|i| i.id == id).unwrap();
            assert!(
                !it.sim_manual_latched && !it.sim_active,
                "Botão tipo {} deve voltar ao estado original quando solto",
                it.tipo
            );
        }

        // Já a chave interruptor 8008 (id=5, DJ) é bi-estável: permanece ligada ao soltar o mouse,
        // e só desliga no próximo clique.
        press_component_on_sim(&mut doc, 5);
        release_component_on_sim(&mut doc, 5);
        step_simulation(&mut doc, 10);
        let sw = doc.items.iter().find(|i| i.id == 5).unwrap();
        assert!(sw.sim_manual_latched && sw.sim_active);

        press_component_on_sim(&mut doc, 5);
        release_component_on_sim(&mut doc, 5);
        step_simulation(&mut doc, 10);
        let sw = doc.items.iter().find(|i| i.id == 5).unwrap();
        assert!(!sw.sim_manual_latched && !sw.sim_active);
    }

    fn verify_doc_equivalence(a: &SchematicDocument, b: &SchematicDocument, ctx: &str) {
        assert_eq!(a.items.len(), b.items.len(), "{ctx}: items.len mismatch");
        assert_eq!(
            a.title_block.title, b.title_block.title,
            "{ctx}: title mismatch"
        );
        for (ia, ib) in a.items.iter().zip(b.items.iter()) {
            assert_eq!(ia.tipo, ib.tipo, "{ctx}: tipo mismatch");
            assert_eq!(ia.tag, ib.tag, "{ctx}: tag mismatch");
            assert_eq!(ia.value, ib.value, "{ctx}: value mismatch");
            assert_eq!(ia.pins, ib.pins, "{ctx}: pins mismatch");
            assert_eq!(
                (ia.x1, ia.y1, ia.x2, ia.y2),
                (ib.x1, ib.y1, ib.x2, ib.y2),
                "{ctx}: coords mismatch"
            );
            assert_eq!(
                ia.orientation, ib.orientation,
                "{ctx}: orientation mismatch"
            );
            assert_eq!(
                ia.trailing_str, ib.trailing_str,
                "{ctx}: trailing_str mismatch"
            );
        }
    }

    fn simulate_enchimento_de_tanque_fluido(mut doc: SchematicDocument, label: &str) {
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);
        let k1 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "K1")
            .unwrap();
        assert!(!k1.sim_active, "{label}: K1 deve iniciar desligado");

        // Pulsa E1 -> liga K1 (selo) e solenoide S1
        set_manual_by_tag(&mut doc, "E1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "E1", false);
        step_simulation(&mut doc, 50);

        let k1 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "K1")
            .unwrap();
        let s1 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "S1")
            .unwrap();
        assert!(
            k1.sim_active,
            "{label}: K1 deve ficar retido após pulsar E1"
        );
        assert!(
            s1.sim_active,
            "{label}: S1 deve estar energizada durante enchimento"
        );

        // Nível alto E2 atua (tanque cheio mantém E2 fechado) -> desliga K1/S1, liga FS e temporizador T1 (2s)
        set_manual_by_tag(&mut doc, "E2", true);
        step_simulation(&mut doc, 50);

        let k1 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "K1")
            .unwrap();
        let fs = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "FS")
            .unwrap();
        let t1 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9004 && i.tag == "T1")
            .unwrap();
        assert!(!k1.sim_active, "{label}: K1 deve desligar ao atingir E2");
        assert!(fs.sim_active, "{label}: FS deve ligar após E2");
        assert!(t1.sim_active, "{label}: T1 deve iniciar contagem de 2s");

        // Avança 2.1s -> T1 dispara, liga K2, S2 e T2 (3s)
        step_simulation(&mut doc, 2100);
        let k2 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "K2")
            .unwrap();
        let s2 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "S2")
            .unwrap();
        let t2 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9004 && i.tag == "T2")
            .unwrap();
        assert!(k2.sim_active, "{label}: K2 deve ligar após 2s de T1");
        assert!(s2.sim_active, "{label}: S2 deve abrir esvaziamento após T1");
        assert!(t2.sim_active, "{label}: T2 deve iniciar contagem de 3s");

        // Avança 3.1s -> T2 dispara e abre contato NF T2 desligando S2
        step_simulation(&mut doc, 3100);
        let s2 = doc
            .items
            .iter()
            .find(|i| i.tipo == 9000 && i.tag == "S2")
            .unwrap();
        assert!(!s2.sim_active, "{label}: S2 deve desligar após 3s de T2");
    }

    fn simulate_inversao_rotacao_motor_trifasico(mut doc: SchematicDocument, label: &str) {
        use crate::model::MotorMode;
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);

        // Pulsa B1 (sentido horário via K1)
        set_manual_by_tag(&mut doc, "B1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B1", false);
        step_simulation(&mut doc, 50);

        let m = doc
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "-M")
            .unwrap();
        assert!(m.sim_active, "{label}: Motor -M deve ligar com B1");
        assert_eq!(
            m.sim_motor_dir, 1,
            "{label}: Motor -M deve girar CW (1) com K1"
        );
        assert_eq!(m.sim_motor_mode, MotorMode::Direct);

        // Pulsa B0 (parada)
        set_manual_by_tag(&mut doc, "B0", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B0", false);
        step_simulation(&mut doc, 50);

        let m = doc
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "-M")
            .unwrap();
        assert!(!m.sim_active, "{label}: Motor -M deve parar com B0");

        // Pulsa B2 (sentido anti-horário via K2)
        set_manual_by_tag(&mut doc, "B2", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B2", false);
        step_simulation(&mut doc, 50);

        let m = doc
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "-M")
            .unwrap();
        assert!(m.sim_active, "{label}: Motor -M deve ligar com B2");
        assert_eq!(
            m.sim_motor_dir, -1,
            "{label}: Motor -M deve girar CCW (-1) com K2"
        );
    }

    fn simulate_estrela_triangulo_sem_inversao(mut doc: SchematicDocument, label: &str) {
        use crate::model::MotorMode;
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);
        // Pulsa S1 -> Partida em Estrela (K1 + K3 + K6=5s)
        set_manual_by_tag(&mut doc, "S1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "S1", false);
        step_simulation(&mut doc, 50);

        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert!(m.sim_active, "{label}: Motor deve partir ao pulsar S1");
        assert_eq!(m.sim_motor_dir, 1, "{label}: Motor deve girar CW");
        assert_eq!(
            m.sim_motor_mode,
            MotorMode::Star,
            "{label}: Motor deve iniciar em Estrela (Y)"
        );

        // Avança 5.1s -> Comuta para Triângulo (K1 + K2)
        step_simulation(&mut doc, 5100);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert!(m.sim_active, "{label}: Motor deve continuar ligado após 5s");
        assert_eq!(m.sim_motor_dir, 1, "{label}: Motor deve manter sentido CW");
        assert_eq!(
            m.sim_motor_mode,
            MotorMode::Delta,
            "{label}: Motor deve comutar para Triângulo (Δ) após 5s"
        );
    }

    fn simulate_estrela_triangulo_com_inversao(mut doc: SchematicDocument, label: &str) {
        use crate::model::MotorMode;
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);

        // 1) Sentido horário (B1): Estrela -> Triângulo
        set_manual_by_tag(&mut doc, "B1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B1", false);
        step_simulation(&mut doc, 50);

        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, 1, MotorMode::Star)
        );

        step_simulation(&mut doc, 5100);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, 1, MotorMode::Delta)
        );

        // 2) Parada (B0)
        set_manual_by_tag(&mut doc, "B0", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B0", false);
        step_simulation(&mut doc, 50);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert!(!m.sim_active, "{label}: Motor deve parar com B0");

        // 3) Sentido anti-horário (B2): Estrela -> Triângulo
        set_manual_by_tag(&mut doc, "B2", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B2", false);
        step_simulation(&mut doc, 50);

        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, -1, MotorMode::Star)
        );

        step_simulation(&mut doc, 5100);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, -1, MotorMode::Delta)
        );
    }

    fn step_secs(doc: &mut SchematicDocument, secs: f64) {
        let steps = ((secs * 10.0).round() as usize).max(1);
        for _ in 0..steps {
            step_simulation(doc, 100);
        }
    }

    fn simulate_estrela_triangulo_com_inversao_automatico(mut doc: SchematicDocument, label: &str) {
        use crate::model::MotorMode;
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);

        // Pulsa B1: inicia ciclo automático
        set_manual_by_tag(&mut doc, "B1", true);
        step_simulation(&mut doc, 100);
        set_manual_by_tag(&mut doc, "B1", false);
        step_simulation(&mut doc, 100);

        // t = 0..4s: Horário em Estrela (K1 + K4)
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, 1, MotorMode::Star),
            "{label}: t=0s deve estar CW Star"
        );

        // Avança 5.5s -> Horário em Triângulo (K1 + K3)
        step_secs(&mut doc, 5.5);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, 1, MotorMode::Delta),
            "{label}: t=5.5s deve estar CW Delta"
        );

        // Avança +5.5s (t=11s) -> Pausa de frenagem/inércia antes de inverter (K5 TOF contando 5s)
        step_secs(&mut doc, 5.5);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (false, 0, MotorMode::Off),
            "{label}: t=11s deve estar em pausa antes de inverter"
        );

        // Avança +5.5s (t=16.5s) -> Inversão automática para Anti-horário em Estrela (K2 + K4)
        step_secs(&mut doc, 5.5);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, -1, MotorMode::Star),
            "{label}: t=16.5s deve estar CCW Star"
        );

        // Avança +5.5s (t=22s) -> Anti-horário em Triângulo (K2 + K3)
        step_secs(&mut doc, 5.5);
        let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
        assert_eq!(
            (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
            (true, -1, MotorMode::Delta),
            "{label}: t=22s deve estar CCW Delta"
        );
    }

    fn simulate_ligacao_tres_motores(mut doc: SchematicDocument, label: &str) {
        use crate::model::MotorMode;
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);
        // Pulsa S1: cascata sequencial de 3 motores estrela-triângulo (3s por estágio)
        set_manual_by_tag(&mut doc, "S1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "S1", false);
        step_simulation(&mut doc, 50);

        let motor_modes = |d: &SchematicDocument| -> Vec<(String, MotorMode)> {
            d.items
                .iter()
                .filter(|i| i.tipo == 1001)
                .map(|i| (i.tag.clone(), i.sim_motor_mode))
                .collect()
        };

        // t = 0s: M1 em Estrela, M2 Off, M3 Off
        assert_eq!(
            motor_modes(&doc),
            vec![
                ("M1".to_string(), MotorMode::Star),
                ("M2".to_string(), MotorMode::Off),
                ("M3".to_string(), MotorMode::Off),
            ],
            "{label}: estágio 1 (t=0s)"
        );

        // t = 3.1s: M1 em Triângulo, M2 Off, M3 Off
        step_simulation(&mut doc, 3100);
        assert_eq!(
            motor_modes(&doc),
            vec![
                ("M1".to_string(), MotorMode::Delta),
                ("M2".to_string(), MotorMode::Off),
                ("M3".to_string(), MotorMode::Off),
            ],
            "{label}: estágio 2 (t=3.1s)"
        );

        // t = 6.2s: M1 em Triângulo, M2 em Estrela, M3 Off
        step_simulation(&mut doc, 3100);
        assert_eq!(
            motor_modes(&doc),
            vec![
                ("M1".to_string(), MotorMode::Delta),
                ("M2".to_string(), MotorMode::Star),
                ("M3".to_string(), MotorMode::Off),
            ],
            "{label}: estágio 3 (t=6.2s)"
        );

        // t = 9.3s: M1 em Triângulo, M2 em Triângulo, M3 Off
        step_simulation(&mut doc, 3100);
        assert_eq!(
            motor_modes(&doc),
            vec![
                ("M1".to_string(), MotorMode::Delta),
                ("M2".to_string(), MotorMode::Delta),
                ("M3".to_string(), MotorMode::Off),
            ],
            "{label}: estágio 4 (t=9.3s)"
        );

        // t = 12.4s: M1 em Triângulo, M2 em Triângulo, M3 em Estrela
        step_simulation(&mut doc, 3100);
        assert_eq!(
            motor_modes(&doc),
            vec![
                ("M1".to_string(), MotorMode::Delta),
                ("M2".to_string(), MotorMode::Delta),
                ("M3".to_string(), MotorMode::Star),
            ],
            "{label}: estágio 5 (t=12.4s)"
        );

        // t = 15.5s: Todos os 3 motores M1, M2, M3 em Triângulo!
        step_simulation(&mut doc, 3100);
        assert_eq!(
            motor_modes(&doc),
            vec![
                ("M1".to_string(), MotorMode::Delta),
                ("M2".to_string(), MotorMode::Delta),
                ("M3".to_string(), MotorMode::Delta),
            ],
            "{label}: estágio 6 final (t=15.5s)"
        );
    }

    fn simulate_partida_direta(mut doc: SchematicDocument, label: &str) {
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "DJ1", true);
        set_manual_by_tag(&mut doc, "DJ2", true);
        step_simulation(&mut doc, 50);

        let mo = doc
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "MO")
            .unwrap();
        assert!(!mo.sim_active, "{label}: Motor MO deve iniciar desligado");

        // Pulsa B1 -> liga -KM1 (selo) e gira motor MO
        set_manual_by_tag(&mut doc, "B1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B1", false);
        step_simulation(&mut doc, 50);

        let mo = doc
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "MO")
            .unwrap();
        assert!(mo.sim_active, "{label}: Motor MO deve girar após pulsar B1");
        assert_eq!(mo.sim_motor_dir, 1);

        // Pulsa B0 -> desliga -KM1 e para motor MO
        set_manual_by_tag(&mut doc, "B0", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B0", false);
        step_simulation(&mut doc, 50);

        let mo = doc
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "MO")
            .unwrap();
        assert!(
            !mo.sim_active,
            "{label}: Motor MO deve parar após pulsar B0"
        );
    }

    fn simulate_exemplo_desconexao(mut doc: SchematicDocument, label: &str) {
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "DJ1", true);
        set_manual_by_tag(&mut doc, "DJ2", true);
        step_simulation(&mut doc, 50);

        // Pulsa B1 -> liga K1 (motor 1 id=6)
        set_manual_by_tag(&mut doc, "B1", true);
        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "B1", false);
        step_simulation(&mut doc, 50);

        let m1 = doc.items.iter().find(|i| i.id == 6).unwrap();
        let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
        assert!(m1.sim_active, "{label}: Motor 1 (K1) deve ligar com B1");
        assert!(!m2.sim_active, "{label}: Motor 2 (K2) ainda desligado");

        // Pulsa B0 -> desliga K1 (motor 1) e dispara temporizador Off-Delay TP (30s) ligando K2 (motor 2)
        set_manual_by_tag(&mut doc, "B0", true);
        step_simulation(&mut doc, 100);
        set_manual_by_tag(&mut doc, "B0", false);
        step_simulation(&mut doc, 100);

        let m1 = doc.items.iter().find(|i| i.id == 6).unwrap();
        let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
        assert!(
            !m1.sim_active,
            "{label}: Motor 1 deve desligar ao pulsar B0"
        );
        assert!(
            m2.sim_active,
            "{label}: Motor 2 (K2) deve ficar ligado durante os 30s do Off-Delay TP"
        );

        // Avança 30.5s -> TP expira e desliga K2 (motor 2)
        step_secs(&mut doc, 30.5);
        let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
        assert!(
            !m2.sim_active,
            "{label}: Motor 2 (K2) deve desligar após os 30s do Off-Delay TP"
        );
    }

    fn simulate_partida_motores_escalonada(mut doc: SchematicDocument, label: &str) {
        use crate::sim::set_manual_by_tag;

        step_simulation(&mut doc, 50);
        set_manual_by_tag(&mut doc, "DJ", true);
        step_simulation(&mut doc, 50);

        // Pulsa B1 -> Liga K1/M1 imediatamente e inicia T1 (7s)
        set_manual_by_tag(&mut doc, "B1", true);
        step_simulation(&mut doc, 100);
        set_manual_by_tag(&mut doc, "B1", false);
        step_simulation(&mut doc, 100);

        let m_states = |d: &SchematicDocument| -> (bool, bool, bool, bool) {
            let m1 = d
                .items
                .iter()
                .find(|i| i.tipo == 1000 && i.tag == "M1")
                .unwrap()
                .sim_active;
            let m2 = d
                .items
                .iter()
                .find(|i| i.tipo == 1000 && i.tag == "M2")
                .unwrap()
                .sim_active;
            let m3 = d
                .items
                .iter()
                .find(|i| i.tipo == 1000 && i.tag == "M3")
                .unwrap()
                .sim_active;
            let ok = d
                .items
                .iter()
                .find(|i| i.tipo == 9008 && i.tag == "OK")
                .unwrap()
                .sim_active;
            (m1, m2, m3, ok)
        };

        assert_eq!(
            m_states(&doc),
            (true, false, false, false),
            "{label}: t=0s (M1 ligado)"
        );

        // Avança 7.2s -> T1 (7s) dispara e liga K2/M2 + inicia T2 (7s)
        step_secs(&mut doc, 7.2);
        assert_eq!(
            m_states(&doc),
            (true, true, false, false),
            "{label}: t=7.2s (M1 e M2 ligados)"
        );

        // Avança +7.2s (t=14.4s) -> T2 (7s) dispara e liga K3/M3 + inicia T3 (3s)
        step_secs(&mut doc, 7.2);
        assert_eq!(
            m_states(&doc),
            (true, true, true, false),
            "{label}: t=14.4s (M1, M2, M3 ligados)"
        );

        // Avança +3.2s (t=17.6s) -> T3 (3s) dispara e acende sinaleiro OK
        step_secs(&mut doc, 3.2);
        assert_eq!(
            m_states(&doc),
            (true, true, true, true),
            "{label}: t=17.6s (M1, M2, M3 e sinaleiro OK ligados)"
        );
    }

    #[test]
    fn test_all_6_projetos_open_simulate_and_roundtrip_json_toml() {
        use std::path::Path;

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let projetos_dir = if manifest.join("Projetos").is_dir() {
            manifest.join("Projetos")
        } else {
            manifest.parent().unwrap().join("Projetos")
        };

        #[allow(clippy::type_complexity)]
        let test_cases: Vec<(&str, fn(SchematicDocument, &str))> = vec![
            (
                "EnchimentodeTanqueFluido",
                simulate_enchimento_de_tanque_fluido,
            ),
            (
                "Inversão de rotação motor trifásico",
                simulate_inversao_rotacao_motor_trifasico,
            ),
            (
                "Estrela triângulo sem inversão de rotação",
                simulate_estrela_triangulo_sem_inversao,
            ),
            (
                "Estrela triângulo com inversão de rotação",
                simulate_estrela_triangulo_com_inversao,
            ),
            (
                "Estrela triângulo com inversão de rotação automático",
                simulate_estrela_triangulo_com_inversao_automatico,
            ),
            ("Ligação três motores", simulate_ligacao_tres_motores),
            ("Partida Direta", simulate_partida_direta),
            ("Exemplo Desconexão", simulate_exemplo_desconexao),
            (
                "Partida de Motores Escalonada",
                simulate_partida_motores_escalonada,
            ),
        ];

        for (stem, sim_fn) in test_cases {
            let cad_path = projetos_dir.join(format!("{stem}.cad"));
            let json_path = projetos_dir.join(format!("{stem}.json"));
            let toml_path = projetos_dir.join(format!("{stem}.toml"));

            // 1) Abre o projeto .cad original
            let doc_cad = load_document(&cad_path)
                .unwrap_or_else(|e| panic!("Erro ao abrir {:?}: {}", cad_path, e));
            assert!(
                !doc_cad.items.is_empty(),
                "Projeto {stem}.cad não deve estar vazio"
            );

            // 2) Salva como .json e .toml na pasta Projetos/
            save_document(&json_path, &doc_cad)
                .unwrap_or_else(|e| panic!("Erro ao salvar {:?}: {}", json_path, e));
            save_document(&toml_path, &doc_cad)
                .unwrap_or_else(|e| panic!("Erro ao salvar {:?}: {}", toml_path, e));

            // 3) Reabre os arquivos .json e .toml (e testa roundtrip .cad em memória)
            let doc_json = load_document(&json_path)
                .unwrap_or_else(|e| panic!("Erro ao reabrir {:?}: {}", json_path, e));
            let doc_toml = load_document(&toml_path)
                .unwrap_or_else(|e| panic!("Erro ao reabrir {:?}: {}", toml_path, e));
            let cad_roundtrip = parse_cad_str(&serialize_cad_str(&doc_cad))
                .unwrap_or_else(|e| panic!("Erro no roundtrip .cad de {stem}: {}", e));

            // 4) Verifica equivalência estrutural completa
            verify_doc_equivalence(&doc_cad, &doc_json, &format!("{stem} (.cad vs .json)"));
            verify_doc_equivalence(&doc_cad, &doc_toml, &format!("{stem} (.cad vs .toml)"));
            verify_doc_equivalence(
                &doc_cad,
                &cad_roundtrip,
                &format!("{stem} (.cad vs .cad roundtrip)"),
            );

            // 5) Executa a simulação elétrica completa no .cad, no .json reaberto e no .toml reaberto
            sim_fn(doc_cad, &format!("{stem}.cad"));
            sim_fn(doc_json, &format!("{stem}.json"));
            sim_fn(doc_toml, &format!("{stem}.toml"));

            println!(
                "[OK] Projeto '{}' aberto (.cad), salvo e reaberto (.json e .toml) e simulado nas 3 versões com sucesso!",
                stem
            );
        }
    }

    #[test]
    fn test_svg_push_button_rendering_and_animation() {
        use crate::svg_symbol::{get_parsed_svg, SvgShape};

        let sym_no = find_symbol(8000).expect("Botão NA 8000 deve existir no catálogo");
        let (no_idle_path, no_active_path) = sym_no
            .svg_paths()
            .expect("Botão NA 8000 deve ter caminhos SVG definidos");

        let no_idle = get_parsed_svg(no_idle_path)
            .unwrap_or_else(|| panic!("Falha ao carregar SVG NO idle: {no_idle_path}"));
        let no_active = get_parsed_svg(no_active_path)
            .unwrap_or_else(|| panic!("Falha ao carregar SVG NO active: {no_active_path}"));

        assert_eq!(no_idle.term_top_y, -21.0);
        assert_eq!(no_idle.term_bottom_y, 21.0);

        // Mapeamento de terminais para escala de grade do CADe (distância 12 unidades)
        let (t0_x, t0_y) = no_idle.map_point(0.0, -21.0, 12.0);
        let (t1_x, t1_y) = no_idle.map_point(0.0, 21.0, 12.0);
        assert!((t0_x - 0.0).abs() < 1e-4 && (t0_y - 0.0).abs() < 1e-4);
        assert!((t1_x - 0.0).abs() < 1e-4 && (t1_y - 12.0).abs() < 1e-4);

        // Verifica que no idle a lâmina está aberta e no ativo fecha em x=0
        let has_open_blade = no_idle.shapes.iter().any(|s| match s {
            SvgShape::Polyline { points } => points.iter().any(|(x, _)| *x < -1.0),
            _ => false,
        });
        assert!(
            has_open_blade,
            "Lâmina do botão NA deve estar aberta em repouso"
        );

        let has_closed_blade = no_active.shapes.iter().any(|s| match s {
            SvgShape::Polyline { points } => points.iter().all(|(x, _)| x.abs() < 0.1),
            _ => false,
        });
        assert!(
            has_closed_blade,
            "Lâmina do botão NA deve fechar (x=0) quando ativo/pressionado"
        );

        // Verifica botão NF (8001)
        let sym_nc = find_symbol(8001).expect("Botão NF 8001 deve existir no catálogo");
        let (nc_idle_path, nc_active_path) = sym_nc
            .svg_paths()
            .expect("Botão NF 8001 deve ter caminhos SVG definidos");

        let nc_idle = get_parsed_svg(nc_idle_path)
            .unwrap_or_else(|| panic!("Falha ao carregar SVG NC idle: {nc_idle_path}"));
        let nc_active = get_parsed_svg(nc_active_path)
            .unwrap_or_else(|| panic!("Falha ao carregar SVG NC active: {nc_active_path}"));

        assert_eq!(nc_idle.term_top_y, -21.0);
        assert_eq!(nc_idle.term_bottom_y, 21.0);

        // No ativo do NF, a lâmina é empurrada para a direita (x >= 8) abrindo o circuito
        let has_pushed_blade = nc_active.shapes.iter().any(|s| match s {
            SvgShape::Polyline { points } => points.iter().any(|(x, _)| *x >= 8.0),
            _ => false,
        });
        assert!(
            has_pushed_blade,
            "Lâmina do botão NF deve ser empurrada para abrir quando pressionada"
        );

        // Verifica que o Motor 1000 possui corpo (Ellipse) e texto descritivo
        let sym_motor = find_symbol(1000).expect("Motor 1000 deve existir");
        let (m_idle_path, m_act_path) = sym_motor.svg_paths().expect("Motor deve ter SVGs");
        let m_idle = get_parsed_svg(m_idle_path).expect("Motor idle deve carregar");
        let m_act = get_parsed_svg(m_act_path).expect("Motor active deve carregar");
        assert!(
            m_idle
                .shapes
                .iter()
                .any(|s| matches!(s, SvgShape::Ellipse { .. })),
            "Motor deve conter elipse/círculo principal de carcaça"
        );
        assert!(
            m_idle
                .shapes
                .iter()
                .any(|s| matches!(s, SvgShape::Text { .. })),
            "Motor deve conter textos informativos (ex: M, 3~)"
        );
        assert!(m_act
            .shapes
            .iter()
            .any(|s| matches!(s, SvgShape::Ellipse { .. })));

        // Verifica que o Relé Térmico Tripolar 6007 (QElectroTech thermal_relay_4) mapeia perfeitamente os 3 polos (0, 6, 12) e tem linha pontilhada
        let sym_thermal = find_symbol(6007).expect("Relé térmico 6007 deve existir");
        let (th_idle_path, th_act_path) =
            sym_thermal.svg_paths().expect("Relé térmico deve ter SVGs");
        assert!(th_idle_path.contains("thermal_relay_4.svg"));
        assert!(th_act_path.contains("thermal_relay_4_active.svg"));
        let th_idle = get_parsed_svg(th_idle_path).expect("Relé térmico QET idle deve carregar");
        let _th_act = get_parsed_svg(th_act_path).expect("Relé térmico QET active deve carregar");
        assert_eq!(th_idle.max_term_x, 40.0);
        assert!(
            th_idle
                .shapes
                .iter()
                .any(|s| matches!(s, SvgShape::Rect { dashed: true, .. })),
            "Relé térmico 4 deve conter linha pontilhada em volta (rect dashed)"
        );
        let (p1x, _) = th_idle.map_point(0.0, -21.0, 12.0);
        let (p2x, _) = th_idle.map_point(20.0, -21.0, 12.0);
        let (p3x, _) = th_idle.map_point(40.0, -21.0, 12.0);
        assert!(
            (p1x - 0.0).abs() < 1e-4,
            "Pólo 1 do relé térmico deve estar em x=0"
        );
        assert!(
            (p2x - 6.0).abs() < 1e-4,
            "Pólo 2 do relé térmico deve estar em x=6"
        );
        assert!(
            (p3x - 12.0).abs() < 1e-4,
            "Pólo 3 do relé térmico deve estar em x=12"
        );

        // Verifica que os contatos auxiliares 8016 e 8017 do relé térmico usam os SVGs QET
        let sym_th_nc = find_symbol(8017).expect("Contato NF relé térmico deve existir");
        let (nc_path, nc_act) = sym_th_nc.svg_paths().expect("8017 deve ter SVGs");
        assert!(nc_path.contains("contact_thermal_relay__nc.svg"));
        assert!(nc_act.contains("contact_thermal_relay__nc_active.svg"));

        // Verifica os Disjuntores Termomagnéticos (2P, 3P e 4P) com os SVGs oficiais solicitados
        // 1. Disjuntor 2P (com 2 pontos)
        let sym_brk_2p = find_symbol(6008).expect("Disjuntor 2P deve existir");
        let (b2_idle_path, b2_act_path) = sym_brk_2p.svg_paths().expect("6008 deve ter SVGs");
        assert!(b2_idle_path.contains("magneto-thermal_circuit_breaker_gv_2p.svg"));
        assert!(b2_act_path.contains("magneto-thermal_circuit_breaker_gv_2p_active.svg"));
        let b2_idle = get_parsed_svg(b2_idle_path).expect("SVG 2P idle deve carregar");
        let _b2_act = get_parsed_svg(b2_act_path).expect("SVG 2P active deve carregar");
        assert_eq!(b2_idle.min_term_x, -20.0);
        assert_eq!(b2_idle.max_term_x, 0.0);
        let (p1, p1_y) = b2_idle.map_point(-20.0, -40.0, 21.0);
        let (p2, p2_y) = b2_idle.map_point(0.0, 40.0, 21.0);
        assert!(
            (p1 - 0.0).abs() < 1e-4,
            "Terminal 1 do disjuntor 2P deve estar em x=0"
        );
        assert!(
            (p1_y - 0.0).abs() < 1e-4,
            "Terminal 1 do disjuntor 2P deve estar em y=0"
        );
        assert!(
            (p2 - 6.0).abs() < 1e-4,
            "Terminal 4 do disjuntor 2P deve estar em x=6"
        );
        assert!(
            (p2_y - 21.0).abs() < 1e-4,
            "Terminal 4 do disjuntor 2P deve estar em y=21"
        );

        // 2. Disjuntor 3P (com 3 pontos) - mesma altura do 2P (target_span=21.0)
        let sym_brk_3p = find_symbol(6003).expect("Disjuntor 3P deve existir");
        let (b3_idle_path, b3_act_path) = sym_brk_3p.svg_paths().expect("6003 deve ter SVGs");
        assert!(b3_idle_path.contains("magneto-thermal_circuit_breaker_gv_4.svg"));
        assert!(b3_act_path.contains("magneto-thermal_circuit_breaker_gv_4_active.svg"));
        let b3_idle = get_parsed_svg(b3_idle_path).expect("SVG 3P idle deve carregar");
        let _b3_act = get_parsed_svg(b3_act_path).expect("SVG 3P active deve carregar");
        assert_eq!(b3_idle.min_term_x, -20.0);
        assert_eq!(b3_idle.max_term_x, 20.0);
        let (b3_p1, _) = b3_idle.map_point(-20.0, -40.0, 21.0);
        let (b3_p2, _) = b3_idle.map_point(0.0, -40.0, 21.0);
        let (b3_p3, b3_p3_y) = b3_idle.map_point(20.0, 40.0, 21.0);
        assert!(
            (b3_p1 - 0.0).abs() < 1e-4,
            "Pólo 1 do disjuntor 3P deve estar em x=0"
        );
        assert!(
            (b3_p2 - 6.0).abs() < 1e-4,
            "Pólo 2 do disjuntor 3P deve estar em x=6"
        );
        assert!(
            (b3_p3 - 12.0).abs() < 1e-4,
            "Pólo 3 do disjuntor 3P deve estar em x=12"
        );
        assert!(
            (b3_p3_y - 21.0).abs() < 1e-4,
            "Bornes inferiores do disjuntor 3P devem estar em y=21 (altura igual ao 2P)"
        );

        // 3. Disjuntor 4P (com 4 pontos) - mesma altura do 2P (target_span=21.0)
        let sym_brk_4p = find_symbol(6004).expect("Disjuntor 4P deve existir");
        let (b4_idle_path, b4_act_path) = sym_brk_4p.svg_paths().expect("6004 deve ter SVGs");
        assert!(b4_idle_path.contains("magneto-thermal_circuit_breaker.svg"));
        assert!(b4_act_path.contains("magneto-thermal_circuit_breaker_active.svg"));
        let b4_idle = get_parsed_svg(b4_idle_path).expect("SVG 4P idle deve carregar");
        let _b4_act = get_parsed_svg(b4_act_path).expect("SVG 4P active deve carregar");
        assert_eq!(b4_idle.min_term_x, -20.0);
        assert_eq!(b4_idle.max_term_x, 40.0);
        let (b4_pn, _) = b4_idle.map_point(-20.0, -40.0, 21.0);
        let (b4_p1, _) = b4_idle.map_point(0.0, -40.0, 21.0);
        let (b4_p2, _) = b4_idle.map_point(20.0, -40.0, 21.0);
        let (b4_p3, b4_p3_y) = b4_idle.map_point(40.0, 40.0, 21.0);
        assert!(
            (b4_pn - 0.0).abs() < 1e-4,
            "Pólo N do disjuntor 4P deve estar em x=0"
        );
        assert!(
            (b4_p1 - 6.0).abs() < 1e-4,
            "Pólo 1 do disjuntor 4P deve estar em x=6"
        );
        assert!(
            (b4_p2 - 12.0).abs() < 1e-4,
            "Pólo 2 do disjuntor 4P deve estar em x=12"
        );
        assert!(
            (b4_p3 - 18.0).abs() < 1e-4,
            "Pólo 3 do disjuntor 4P deve estar em x=18"
        );
        assert!(
            (b4_p3_y - 21.0).abs() < 1e-4,
            "Bornes inferiores do disjuntor 4P devem estar em y=21 (altura igual ao 2P)"
        );
    }

    #[test]
    fn test_qet_kaefer_open_simulate_and_roundtrip() {
        use std::path::Path;

        let qet_path = Path::new("/home/caiojordao84/Downloads/Kaefer_1303_20260204-2.qet");
        if !qet_path.exists() {
            println!("Arquivo QET de teste não encontrado no caminho padrão, ignorando teste.");
            return;
        }

        // 1. Carrega o projeto completo com 23 folhas e 300+ definições de biblioteca
        let project = crate::qet::load_qet_file(qet_path)
            .expect("Deve carregar o arquivo Kaefer_1303_20260204-2.qet com sucesso");

        assert_eq!(project.title, "Kaefer_1303_20251206");
        assert_eq!(
            project.diagrams.len(),
            23,
            "Projeto do Fusca deve conter 23 diagramas"
        );
        assert!(
            project.element_definitions.len() >= 100,
            "Coleção de símbolos embutidos deve ter mais de 100 elementos definidos"
        );

        // 2. Converte o Diagrama 2 (Bateria, chave de ignição e motor de partida) em SchematicDocument
        let d2_idx = project
            .diagrams
            .iter()
            .position(|d| d.order == 2)
            .expect("Diagrama de ordem 2 deve existir");

        let mut doc = crate::qet::diagram_to_document(&project, d2_idx);
        assert!(
            doc.items.len() >= 30,
            "Diagrama 2 deve ter elementos, condutores e textos importados (total: {})",
            doc.items.len()
        );

        // Verifica a presença dos componentes principais do circuito de partida
        let has_battery = doc.items.iter().any(|i| {
            i.qet_type
                .as_deref()
                .map(|t| t.to_ascii_lowercase().contains("battery"))
                .unwrap_or(false)
        });
        assert!(
            has_battery,
            "Diagrama 2 deve conter o componente da Bateria 12V"
        );

        let starter_idx = doc
            .items
            .iter()
            .position(|i| {
                i.qet_type
                    .as_deref()
                    .map(|t| t.to_ascii_lowercase().ends_with("starter_001.elmt"))
                    .unwrap_or(false)
            })
            .expect("Diagrama 2 deve conter o motor de partida (starter_001.elmt)");

        // Verifica que os símbolos QET possuem textos dinâmicos e labels de bornes compilados
        let starter_qet_type = doc.items[starter_idx].qet_type.as_ref().unwrap();
        let parsed_starter = crate::svg_symbol::get_parsed_svg(starter_qet_type)
            .expect("Símbolo do motor de partida deve estar no cache de SVGs compilados");
        let has_text_shapes = parsed_starter
            .shapes
            .iter()
            .any(|s| matches!(s, crate::svg_symbol::SvgShape::Text { .. }));
        assert!(
            has_text_shapes,
            "Símbolo do motor de partida deve conter textos/labels dinâmicos compilados com rotação!"
        );

        let switch_idx = doc
            .items
            .iter()
            .position(|i| {
                i.qet_type
                    .as_deref()
                    .map(|t| {
                        t.to_ascii_lowercase()
                            .contains("ignition_starter_switch_with_park")
                    })
                    .unwrap_or(false)
            })
            .expect("Diagrama 2 deve conter o comutador de ignição");

        // 3. Testa a simulação elétrica do circuito de partida do Fusca
        // Estado inicial de repouso: motor de partida desligado
        step_simulation(&mut doc, 0);
        assert!(
            !doc.items[starter_idx].sim_active,
            "Motor de partida deve estar desativado com a chave de ignição aberta"
        );

        // Aciona a chave de ignição (partida)
        doc.items[switch_idx].sim_manual_latched = true;
        step_simulation(&mut doc, 0);

        // O motor de partida deve ser energizado pela bateria via comutador
        assert!(
            doc.items[starter_idx].sim_active,
            "Motor de partida deve estar ativo quando a chave de ignição for ligada!"
        );
        assert_eq!(
            doc.items[starter_idx].sim_motor_dir, 1,
            "Motor de partida deve indicar rotação ativa (dir = 1)"
        );

        // Ao desligar a chave de ignição, o motor de partida desliga
        doc.items[switch_idx].sim_manual_latched = false;
        step_simulation(&mut doc, 0);
        assert!(
            !doc.items[starter_idx].sim_active,
            "Motor de partida deve desligar ao soltar a chave de ignição"
        );

        // 3b. Testa a simulação da Folha 12 ("Horn, fuel gauge, fan", índice 11)
        let mut doc_horn = crate::qet::diagram_to_document(&project, 11);
        let horn_idx = doc_horn
            .items
            .iter()
            .position(|i| {
                i.qet_type
                    .as_deref()
                    .map(|t| t.to_ascii_lowercase().ends_with("horn_001.elmt"))
                    .unwrap_or(false)
            })
            .expect("Diagrama 12 deve conter a buzina (horn_001.elmt)");
        let horn_sw_idx = doc_horn
            .items
            .iter()
            .position(|i| {
                i.qet_type
                    .as_deref()
                    .map(|t| t.to_ascii_lowercase().contains("horn_switch"))
                    .unwrap_or(false)
            })
            .expect("Diagrama 12 deve conter o botão da buzina (horn_switch_001.elmt)");

        step_simulation(&mut doc_horn, 0);
        assert!(
            !doc_horn.items[horn_idx].sim_active,
            "Buzina desligada em repouso"
        );

        doc_horn.items[horn_sw_idx].sim_manual_latched = true;
        step_simulation(&mut doc_horn, 0);
        assert!(
            doc_horn.items[horn_idx].sim_active,
            "Buzina deve tocar ao acionar o botão horn_switch!"
        );

        // 4. Testa roundtrip de gravação e recarregamento em XML .qet
        let tmp_out = Path::new("/tmp/test_kaefer_roundtrip.qet");
        crate::qet::save_qet_file(tmp_out, &project)
            .expect("Gravação de projeto .qet deve ser concluída sem erros");

        let reloaded = crate::qet::load_qet_file(tmp_out)
            .expect("Deve recarregar com sucesso o arquivo .qet salvo");

        assert_eq!(reloaded.title, project.title);
        assert_eq!(reloaded.diagrams.len(), 23);
        assert!(reloaded.element_definitions.len() >= 100);

        let _ = std::fs::remove_file(tmp_out);
    }

    #[test]
    fn test_new_ui_layout_and_theme_collections() {
        let (mut app, _) = CadeSimuApp::new();

        // Verifica estado inicial do novo layout e tema
        assert_eq!(app.theme_mode, ThemeMode::Dark);
        assert_eq!(app.prop_tab, 0);
        assert_eq!(app.selected_collection_sym, Some(6003));
        assert!(app.collection_search.is_empty());

        // Verifica ciclo de tema: Dark -> System -> Light -> Dark
        assert_eq!(app.theme_mode.next(), ThemeMode::System);
        assert_eq!(app.theme_mode.next().next(), ThemeMode::Light);
        assert_eq!(app.theme_mode.next().next().next(), ThemeMode::Dark);

        // Testa comportamento da aba Elementos e Projetos:
        // 1) Clique simples apenas seleciona o símbolo para pré-visualização, mantendo a ferramenta Select
        let _ = app.update(Message::SelectCollectionSymbol(6008));
        assert_eq!(app.selected_collection_sym, Some(6008));
        assert_eq!(app.tool, EditorTool::Select);

        // 2) Duplo clique arma o símbolo no ponteiro do mouse
        let _ = app.update(Message::SelectCollectionSymbol(6008));
        assert_eq!(app.tool, EditorTool::PlaceSymbol(6008));

        // 3) Cancelamento com tecla ESC retorna imediatamente ao modo de seleção
        let _ = app.update(Message::CancelAction);
        assert_eq!(app.tool, EditorTool::Select);

        // 4) Botão "Inserir no Esquema" arma o símbolo
        let _ = app.update(Message::InsertSelectedCollectionSymbol);
        assert_eq!(app.tool, EditorTool::PlaceSymbol(6008));

        // 5) Cancelamento com botão direito no Canvas retorna ao modo de seleção
        let _ = app.update(Message::Canvas(CanvasMessage::CancelAction));
        assert_eq!(app.tool, EditorTool::Select);

        // 6) Arraste do símbolo solto no Canvas coloca o elemento diretamente na posição
        let items_before = app.doc.items.len();
        let _ = app.update(Message::StartSymbolDrag(6008));
        assert_eq!(app.dragging_symbol, Some(6008));
        let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
            tipo: 6008,
            x: 100,
            y: 150,
        }));
        assert_eq!(app.doc.items.len(), items_before + 1);
        assert_eq!(app.dragging_symbol, None);
        assert_eq!(app.tool, EditorTool::Select);

        // Testa troca de abas de propriedades
        let _ = app.update(Message::SelectPropTab(1));
        assert_eq!(app.prop_tab, 1);
        let _ = app.update(Message::SelectPropTab(2));
        assert_eq!(app.prop_tab, 2);

        // Testa busca de coleções
        let _ = app.update(Message::CollectionSearchChanged("motor".to_string()));
        assert_eq!(app.collection_search, "motor");

        // Testa renderização completa da UI
        let _view_element = app.view();
    }

    #[test]
    fn test_multipage_menus_iec_undo_redo_toolbar() {
        let (mut app, _) = CadeSimuApp::new();

        // 1) Multi-páginas: inicial tem 1 página
        assert_eq!(app.pages.len(), 1);
        assert_eq!(app.active_page_index, 0);

        // Adiciona página depois
        let _ = app.update(Message::AddPageAfter(0));
        assert_eq!(app.pages.len(), 2);
        assert_eq!(app.active_page_index, 1);
        assert!(app.is_modified);

        // Adiciona página antes
        let _ = app.update(Message::AddPageBefore(1));
        assert_eq!(app.pages.len(), 3);
        assert_eq!(app.active_page_index, 1);

        // Renomeia folha ativa
        let _ = app.update(Message::StartEditingPageName(1));
        assert!(app.editing_page_name.is_some());
        let _ = app.update(Message::UpdateEditingPageName(
            "Diagrama de Comando".to_string(),
        ));
        let _ = app.update(Message::CommitEditingPageName);
        assert_eq!(app.pages[1].name, "Diagrama de Comando");

        // Reordena páginas
        let _ = app.update(Message::MovePage(1, 0));
        assert_eq!(app.pages[0].name, "Diagrama de Comando");
        assert_eq!(app.active_page_index, 0);

        // Exclui página
        let _ = app.update(Message::DeletePage(2));
        assert_eq!(app.pages.len(), 2);

        // 2) Menus Dropdown Flutuantes em Camada Superior (Stack / Overlay)
        assert_eq!(app.open_menu, None);
        let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Arquivo)));
        assert_eq!(app.open_menu, Some(ActiveMenu::Arquivo));
        {
            let _ = app.view();
        }
        // Alterna diretamente para outro menu (Exibição)
        let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Exibicao)));
        assert_eq!(app.open_menu, Some(ActiveMenu::Exibicao));
        {
            let _ = app.view();
        }
        // Uma ação como ZoomIn executa e fecha o menu dropdown automaticamente
        let _ = app.update(Message::ZoomIn);
        assert_eq!(app.open_menu, None);

        // Fecha explicitamente com CloseMenu
        let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Arquivo)));
        assert_eq!(app.open_menu, Some(ActiveMenu::Arquivo));
        let _ = app.update(Message::CloseMenu);
        assert_eq!(app.open_menu, None);

        // 3) Configurações de Grade, Textos e Autoconexão
        assert_eq!(app.grid_density.step(), 6);
        let _ = app.update(Message::SetGridDensity(crate::model::GridDensity::Fine));
        assert_eq!(app.grid_density.step(), 3);
        let _ = app.update(Message::SetGridDensity(crate::model::GridDensity::Coarse));
        assert_eq!(app.grid_density.step(), 12);

        assert!(app.show_texts);
        let _ = app.update(Message::ToggleShowTexts);
        assert!(!app.show_texts);

        assert!(app.auto_connect);
        let _ = app.update(Message::ToggleAutoConnect);
        assert!(!app.auto_connect);

        assert!(!app.auto_disconnect);
        let _ = app.update(Message::ToggleAutoDisconnect);
        assert!(app.auto_disconnect);

        // 4) IEC Presets e Edição de Propriedades
        let original_page_idx = app
            .pages
            .iter()
            .position(|p| !p.doc.items.is_empty())
            .unwrap_or(0);
        let _ = app.update(Message::SelectPage(original_page_idx));
        assert!(!app.doc.items.is_empty());

        let initial_items_count = app.doc.items.len();
        let first_id = app.doc.items[0].id;
        app.selected_id = Some(first_id);

        let _ = app.update(Message::ApplyIecNumberingPreset("power_3p"));
        let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
        assert_eq!(item.pins[0], "1");
        assert_eq!(item.pins[1], "3");
        assert_eq!(item.pins[2], "5");
        assert_eq!(item.pins[3], "2");
        assert_eq!(item.pins[4], "4");
        assert_eq!(item.pins[5], "6");

        let _ = app.update(Message::ApplyIecNumberingPreset("aux_no"));
        let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
        assert_eq!(item.pins[0], "13");
        assert_eq!(item.pins[1], "14");

        let _ = app.update(Message::ApplyIecNumberingPreset("aux_nc"));
        let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
        assert_eq!(item.pins[0], "21");
        assert_eq!(item.pins[1], "22");

        let _ = app.update(Message::ApplyIecNumberingPreset("coil"));
        let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
        assert_eq!(item.pins[0], "A1");
        assert_eq!(item.pins[1], "A2");

        // 5) Cores de lâmpada e pulso
        let _ = app.update(Message::SetLampColor(2)); // Vermelho
        let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
        assert_eq!(item.color, 2);

        let _ = app.update(Message::SetPulseDuration("0.5".to_string()));
        let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
        assert_eq!(item.value, "0.5");

        // 6) Área de transferência: Copiar, Colar, Duplicar
        let _ = app.update(Message::Copy);
        assert!(app.clipboard.is_some());

        let _ = app.update(Message::Paste);
        assert_eq!(app.doc.items.len(), initial_items_count + 1);

        let _ = app.update(Message::Duplicate);
        assert_eq!(app.doc.items.len(), initial_items_count + 2);

        // 7) Undo e Redo
        let _ = app.update(Message::Undo);
        assert_eq!(app.doc.items.len(), initial_items_count + 1);
        let _ = app.update(Message::Undo);
        assert_eq!(app.doc.items.len(), initial_items_count);

        let _ = app.update(Message::Redo);
        assert_eq!(app.doc.items.len(), initial_items_count + 1);

        // 8) Dock Esquerdo Recolhível e Abas
        assert!(!app.left_dock_collapsed);
        let _ = app.update(Message::ToggleLeftDockCollapse);
        assert!(app.left_dock_collapsed);
        {
            let _ = app.view();
        }
        let _ = app.update(Message::ToggleLeftDockCollapse);
        assert!(!app.left_dock_collapsed);

        let _ = app.update(Message::SelectLeftDockTab(1)); // Projetos
        assert_eq!(app.left_dock_tab, 1);
        {
            let _ = app.view();
        }

        // 8b) Dock Direito Recolhível
        assert!(!app.right_dock_collapsed);
        let _ = app.update(Message::ToggleRightDockCollapse);
        assert!(app.right_dock_collapsed);
        {
            let _ = app.view();
        }
        let _ = app.update(Message::ToggleRightDockCollapse);
        assert!(!app.right_dock_collapsed);
        {
            let _ = app.view();
        }

        // 9) Modal Sobre
        let _ = app.update(Message::ShowAbout);
        assert!(app.show_about_modal);
        {
            let _ = app.view();
        }
        let _ = app.update(Message::CloseAbout);
        assert!(!app.show_about_modal);

        // 10) Seleção, Cópia de Info e Snap à Grelha
        let _ = app.update(Message::SelectAll);
        assert!(app.selected_id.is_some());

        let _ = app.update(Message::CopyElementInfo);
        assert!(app.copied_info.is_some());

        let _ = app.update(Message::DeselectAll);
        assert!(app.selected_id.is_none());

        assert!(app.snap_to_grid);
        let _ = app.update(Message::ToggleSnap);
        assert!(!app.snap_to_grid);

        let _ = app.update(Message::PasteAtOriginalPos);
        assert_eq!(app.doc.items.len(), initial_items_count + 2);

        // 11) Tooltip seguindo o tema com fundo e estilo
        let tt_style_dark = theme::tooltip(&iced::Theme::Dark);
        assert!(tt_style_dark.background.is_some());
        let tt_style_light = theme::tooltip(&iced::Theme::Light);
        assert!(tt_style_light.background.is_some());
    }

    #[test]
    fn test_sinaleiro_color_and_aux_contacts_distinction() {
        let (mut app, _) = CadeSimuApp::new();

        // 1) Contatos Auxiliares NA (7000) e NF (7001) não devem ser sinalizadores/lâmpadas
        let sym_na = find_symbol(7000).expect("Contato NA deve existir");
        let item_na = sym_na.instantiate(101, 10, 10, 0);
        assert_eq!(item_na.tipo, 7000);
        assert!(
            !item_na.is_lamp_or_signaling(),
            "Contato NA (7000) NÃO deve ser considerado lâmpada"
        );

        let sym_nf = find_symbol(7001).expect("Contato NF deve existir");
        let item_nf = sym_nf.instantiate(102, 20, 20, 0);
        assert_eq!(item_nf.tipo, 7001);
        assert!(
            !item_nf.is_lamp_or_signaling(),
            "Contato NF (7001) NÃO deve ser considerado lâmpada"
        );

        // 2) Sinaleiro Luminoso (Piloto, 9008) DEVE ser sinaleiro/lâmpada
        let sym_lamp = find_symbol(9008).expect("Sinaleiro 9008 deve existir");
        let item_lamp = sym_lamp.instantiate(103, 30, 30, 0);
        assert_eq!(item_lamp.tipo, 9008);
        assert!(
            item_lamp.is_lamp_or_signaling(),
            "Sinaleiro (9008) DEVE ser considerado lâmpada"
        );

        // 3) Ao inserir o sinaleiro, ele deve ter cor verde por padrão (1) e permitir alterar
        assert_eq!(item_lamp.color, 1); // Verde padrão
        app.doc.items.push(item_lamp);
        app.selected_id = Some(103);

        // Altera cor para Vermelho (2)
        let _ = app.update(Message::SetLampColor(2));
        let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
        assert_eq!(item.color, 2);

        // Altera cor para Amarelo (3)
        let _ = app.update(Message::SetLampColor(3));
        let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
        assert_eq!(item.color, 3);

        // Altera cor para Azul (4)
        let _ = app.update(Message::SetLampColor(4));
        let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
        assert_eq!(item.color, 4);

        // Aplica preset de bornes Piloto X1-X2
        let _ = app.update(Message::ApplyIecNumberingPreset("lamp"));
        let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
        assert_eq!(item.pins[0], "X1");
        assert_eq!(item.pins[1], "X2");

        // Renderiza a view garantindo que o painel de propriedades renderiza sem pânico
        let _ = app.view();
    }

    #[test]
    fn test_all_icons_render_valid_svg() {
        use crate::icons::{view_icon, Icon};
        let all_icons = [
            Icon::New,
            Icon::Open,
            Icon::Save,
            Icon::SaveAs,
            Icon::Close,
            Icon::Print,
            Icon::ExportPdf,
            Icon::Undo,
            Icon::Redo,
            Icon::Cut,
            Icon::Copy,
            Icon::Paste,
            Icon::Duplicate,
            Icon::Delete,
            Icon::RotateCw,
            Icon::RotateCcw,
            Icon::MirrorH,
            Icon::MirrorV,
            Icon::Palette,
            Icon::Select,
            Icon::Pan,
            Icon::WireL,
            Icon::WireN,
            Icon::WirePe,
            Icon::WireDc,
            Icon::Junction,
            Icon::DeleteWire,
            Icon::ZoomIn,
            Icon::ZoomOut,
            Icon::ZoomReset,
            Icon::FitToPage,
            Icon::Grid,
            Icon::Magnet,
            Icon::Text,
            Icon::Theme,
            Icon::Play,
            Icon::Pause,
            Icon::Stop,
            Icon::Step,
            Icon::Reset,
            Icon::ChevronLeft,
            Icon::ChevronRight,
            Icon::AddBefore,
            Icon::AddAfter,
            Icon::Search,
            Icon::Pencil,
            Icon::Folder,
            Icon::Bolt,
        ];
        for icon in all_icons {
            let bytes = icon.bytes();
            assert!(!bytes.is_empty());
            let _widget = view_icon(icon, 16, iced::Color::BLACK);
        }
    }

    #[test]
    fn test_canvas_theme_adaptation_light_dark_system() {
        let (mut app, _) = CadeSimuApp::new();

        // 1. Testa resolução de temas (LIGHT, DARK, SYSTEM)
        app.theme_mode = ThemeMode::Light;
        assert!(!app.is_dark());

        app.theme_mode = ThemeMode::Dark;
        assert!(app.is_dark());

        app.theme_mode = ThemeMode::System;
        app.system_is_dark = true;
        assert!(app.is_dark());

        app.system_is_dark = false;
        assert!(!app.is_dark());

        // 2. Testa comportamento do wire_color no Canvas
        let mut wire_phase = SchematicItem::new_wire(1, 4000, 10, 10, 20, 10);
        let wire_neutral = SchematicItem::new_wire(2, 4009, 10, 20, 20, 20);
        let wire_dneg = SchematicItem::new_wire(3, 4004, 10, 30, 20, 30);
        let wire_generic = SchematicItem::new_wire(4, 4001, 10, 40, 20, 40);

        let canvas_light = SchematicCanvas {
            doc: &app.doc,
            tool: app.tool,
            selected_id: None,
            sim_running: false,
            show_grid: true,
            grid_density: app.grid_density,
            show_texts: true,
            is_dark: false,
            auto_connect: true,
            auto_disconnect: true,
            zoom: 1.0,
            pan: iced::Vector::default(),
            placement_orientation: 0,
            dragging_symbol: None,
            is_blocked: false,
        };

        let canvas_dark = SchematicCanvas {
            doc: &app.doc,
            tool: app.tool,
            selected_id: None,
            sim_running: false,
            show_grid: true,
            grid_density: app.grid_density,
            show_texts: true,
            is_dark: true,
            auto_connect: true,
            auto_disconnect: true,
            zoom: 1.0,
            pan: iced::Vector::default(),
            placement_orientation: 0,
            dragging_symbol: None,
            is_blocked: false,
        };

        // No tema LIGHT, itens e conectores são escuros (inalterados)
        let (col_l_phase, _) = canvas_light.wire_color(&wire_phase);
        assert_eq!(col_l_phase, Color::from_rgb(0.55, 0.18, 0.12));

        let (col_l_neutral, _) = canvas_light.wire_color(&wire_neutral);
        assert_eq!(col_l_neutral, Color::from_rgb(0.08, 0.38, 0.82));

        let (col_l_dneg, _) = canvas_light.wire_color(&wire_dneg);
        assert_eq!(col_l_dneg, Color::from_rgb(0.15, 0.15, 0.20));

        let (col_l_generic, _) = canvas_light.wire_color(&wire_generic);
        assert_eq!(col_l_generic, Color::from_rgb(0.15, 0.18, 0.24));

        // No tema DARK, conectores sem cor personalizada tornam-se claros
        let (col_d_phase, _) = canvas_dark.wire_color(&wire_phase);
        assert_eq!(col_d_phase, Color::from_rgb(0.94, 0.58, 0.50));

        let (col_d_neutral, _) = canvas_dark.wire_color(&wire_neutral);
        assert_eq!(col_d_neutral, Color::from_rgb(0.45, 0.75, 1.0));

        let (col_d_dneg, _) = canvas_dark.wire_color(&wire_dneg);
        assert_eq!(col_d_dneg, Color::from_rgb(0.85, 0.88, 0.95));

        let (col_d_generic, _) = canvas_dark.wire_color(&wire_generic);
        assert_eq!(col_d_generic, Color::from_rgb(0.88, 0.92, 0.98));

        // 3. Conectores com cor personalizada definida pelo usuário (item.color != 0)
        // NUNCA são sobrescritos pelo tema
        let custom_color_rgb = 0x00FF88; // verde-água vibrante
        wire_phase.color = custom_color_rgb;
        let (col_cust_light, _) = canvas_light.wire_color(&wire_phase);
        let (col_cust_dark, _) = canvas_dark.wire_color(&wire_phase);
        assert_eq!(col_cust_light, col_cust_dark);
        assert_eq!(
            col_cust_light,
            Color::from_rgb(0.0, 1.0, 0x88 as f32 / 255.0)
        );

        // 4. Renderiza view em modo claro e escuro sem erros
        app.theme_mode = ThemeMode::Light;
        let _ = app.view();
        app.theme_mode = ThemeMode::Dark;
        let _ = app.view();

        // 5. Testa adaptação da miniatura de preview (preview_dock) ao tema claro e escuro
        app.selected_collection_sym = Some(8008); // Chave interruptor DJ
        let sym_dj = find_symbol(8008).unwrap();
        let svg_dj_light = crate::symbols::get_themed_symbol_svg_string(&sym_dj, false);
        let svg_dj_dark = crate::symbols::get_themed_symbol_svg_string(&sym_dj, true);

        assert!(
            svg_dj_light.contains("stroke=\"#111827\"")
                || svg_dj_light.contains("stroke=\"black\""),
            "No tema claro, o traço da miniatura deve ser escuro"
        );
        assert!(
            svg_dj_dark.contains("stroke=\"#E2E8F0\""),
            "No tema escuro, o traço da miniatura deve ser claro (#E2E8F0)"
        );
        assert!(
            !svg_dj_dark.contains("stroke=\"#111827\"")
                && !svg_dj_dark.contains("stroke=\"black\""),
            "No tema escuro, traços pretos devem ter sido substituídos por claros"
        );

        // Testa também para botão de emergência com trava (8030)
        let sym_emg = find_symbol(8030).unwrap();
        let svg_emg_dark = crate::symbols::get_themed_symbol_svg_string(&sym_emg, true);
        assert!(
            svg_emg_dark.contains("stroke=\"#E2E8F0\""),
            "Emergência 8030 deve ter traços claros em dark mode"
        );
        assert!(
            !svg_emg_dark.contains("stroke=\"black\""),
            "Emergência 8030 não pode ter stroke='black' em dark mode"
        );

        // Testa para componente gerado dinamicamente (ex: motor 1000)
        let sym_motor = find_symbol(1000).unwrap();
        let svg_motor_dark = crate::symbols::get_themed_symbol_svg_string(&sym_motor, true);
        assert!(
            svg_motor_dark.contains("stroke=\"#E2E8F0\""),
            "Motor 1000 deve ter traços claros em dark mode"
        );

        // Renderiza app.view() com o símbolo selecionado em ambos os temas para garantir integração total
        app.theme_mode = ThemeMode::Light;
        let _ = app.view();
        app.theme_mode = ThemeMode::Dark;
        let _ = app.view();
    }

    #[test]
    fn test_strict_and_series_conduction_command_and_power() {
        use crate::sim::set_manual_by_tag;

        // ==================== 1. CIRCUITO DE COMANDO EM SÉRIE (AND / &&) ====================
        // Monta: L1 (3000) -> S1 (NA, 8000) -> S2 (NA, 8000) -> Bobina KM1 (9000) -> Neutro N (3001)
        let mut doc = SchematicDocument::default();
        let mut id_counter = 1;
        let mut next_id = || {
            let cur = id_counter;
            id_counter += 1;
            cur
        };

        // Alimentação Fase L em (72, 10)
        let sym_l = find_symbol(3000).unwrap();
        let mut it_l = sym_l.instantiate(next_id(), 72, 10, 0);
        it_l.tag = "-L1".to_string();
        doc.items.push(it_l);

        // Chave S1 (NA, 8000) em (72, 20): pin 0 em (72, 20), pin 1 em (72, 32)
        let sym_s1 = find_symbol(8000).unwrap();
        let mut it_s1 = sym_s1.instantiate(next_id(), 72, 20, 0);
        it_s1.tag = "-S1".to_string();
        doc.items.push(it_s1);

        // Chave S2 (NA, 8000) em (72, 40): pin 0 em (72, 40), pin 1 em (72, 52)
        let sym_s2 = find_symbol(8000).unwrap();
        let mut it_s2 = sym_s2.instantiate(next_id(), 72, 40, 0);
        it_s2.tag = "-S2".to_string();
        doc.items.push(it_s2);

        // Bobina KM1 (9000) em (72, 60): pin 0 (A1) em (72, 60), pin 1 (A2) em (72, 72)
        let sym_km = find_symbol(9000).unwrap();
        let mut it_km = sym_km.instantiate(next_id(), 72, 60, 0);
        it_km.tag = "-KM1".to_string();
        doc.items.push(it_km);

        // Alimentação Neutro N (3001) em (72, 80, rot 2)
        let sym_n = find_symbol(3001).unwrap();
        let mut it_n = sym_n.instantiate(next_id(), 72, 80, 2);
        it_n.tag = "-N".to_string();
        doc.items.push(it_n);

        // Cabos ligando em série:
        // Cabo 1: L -> S1 (72, 10) -> (72, 20)
        let w_l_s1 = next_id();
        doc.items
            .push(SchematicItem::new_wire(w_l_s1, 4000, 72, 10, 72, 20));
        // Cabo 2: S1 -> S2 (72, 32) -> (72, 40)
        let w_s1_s2 = next_id();
        doc.items
            .push(SchematicItem::new_wire(w_s1_s2, 4000, 72, 32, 72, 40));
        // Cabo 3: S2 -> KM1 (72, 52) -> (72, 60)
        let w_s2_km = next_id();
        doc.items
            .push(SchematicItem::new_wire(w_s2_km, 4000, 72, 52, 72, 60));
        // Cabo 4: KM1 -> N (72, 72) -> (72, 80)
        let w_km_n = next_id();
        doc.items
            .push(SchematicItem::new_wire(w_km_n, 4009, 72, 72, 72, 80));

        // Caso 1: (S1=0, S2=0) -> Ambos abertos
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(!km.sim_active, "AND (0,0): Bobina deve estar DESLIGADA");
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s1_s2)
                .unwrap()
                .sim_potential,
            Potential::None
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s2_km)
                .unwrap()
                .sim_potential,
            Potential::None
        );

        // Caso 2: (S1=1, S2=0) -> S1 fechado, S2 aberto
        // A eletricidade passa por S1 (cabo w_s1_s2 recebe L1), mas NÃO passa por S2 (w_s2_km é None)!
        set_manual_by_tag(&mut doc, "-S1", true);
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(
            !km.sim_active,
            "AND (1,0): Bobina deve estar DESLIGADA se S2 estiver aberto"
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s1_s2)
                .unwrap()
                .sim_potential,
            Potential::L1
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s2_km)
                .unwrap()
                .sim_potential,
            Potential::None
        );

        // Caso 3: (S1=0, S2=1) -> S1 aberto, S2 fechado
        // A eletricidade é barrada em S1!
        set_manual_by_tag(&mut doc, "-S1", false);
        set_manual_by_tag(&mut doc, "-S2", true);
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(
            !km.sim_active,
            "AND (0,1): Bobina deve estar DESLIGADA se S1 estiver aberto"
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s1_s2)
                .unwrap()
                .sim_potential,
            Potential::None
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s2_km)
                .unwrap()
                .sim_potential,
            Potential::None
        );

        // Caso 4: (S1=1, S2=1) -> Ambos fechados!
        // Condição AND satisfeita: eletricidade passa por S1 E por S2, energizando KM1!
        set_manual_by_tag(&mut doc, "-S1", true);
        set_manual_by_tag(&mut doc, "-S2", true);
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(
            km.sim_active,
            "AND (1,1): Bobina DEVE LIGAR quando ambos estiverem fechados"
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s1_s2)
                .unwrap()
                .sim_potential,
            Potential::L1
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_s2_km)
                .unwrap()
                .sim_potential,
            Potential::L1
        );

        // ==================== 2. CIRCUITO DE POTÊNCIA EM SÉRIE (AND / &&) ====================
        // Monta: Alimentação 3F (3004) -> Disjuntor 3P -Q1 (6003) -> Contator 3P -KM2 (2001) -> Motor 3~ -M1 (1000)
        let mut doc_pwr = SchematicDocument::default();
        let sym_3f = find_symbol(3004).unwrap();
        doc_pwr.items.push(sym_3f.instantiate(next_id(), 24, 18, 0));

        let sym_q1 = find_symbol(6003).unwrap();
        let mut it_q1 = sym_q1.instantiate(next_id(), 24, 24, 0);
        it_q1.tag = "-Q1".to_string();
        doc_pwr.items.push(it_q1);

        let sym_km2 = find_symbol(2001).unwrap();
        let mut it_km2 = sym_km2.instantiate(next_id(), 24, 51, 0);
        it_km2.tag = "-KM2".to_string();
        doc_pwr.items.push(it_km2);

        // Circuito de comando para acionar a bobina KM2:
        // L -> S_KM2 (8000) -> Bobina KM2 (9000) -> N
        let sym_l_cmd = find_symbol(3000).unwrap();
        doc_pwr
            .items
            .push(sym_l_cmd.instantiate(next_id(), 72, 45, 0));

        let sym_s_km2 = find_symbol(8000).unwrap();
        let mut it_s_km2 = sym_s_km2.instantiate(next_id(), 72, 50, 0);
        it_s_km2.tag = "-SKM2".to_string();
        doc_pwr.items.push(it_s_km2);

        let mut it_km2_coil = sym_km.instantiate(next_id(), 72, 68, 0);
        it_km2_coil.tag = "-KM2".to_string();
        doc_pwr.items.push(it_km2_coil);

        let sym_n_cmd = find_symbol(3001).unwrap();
        doc_pwr
            .items
            .push(sym_n_cmd.instantiate(next_id(), 72, 85, 2));

        // Cabos de comando da bobina KM2:
        doc_pwr
            .items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 45, 72, 50));
        doc_pwr
            .items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 62, 72, 68));
        doc_pwr
            .items
            .push(SchematicItem::new_wire(next_id(), 4009, 72, 80, 72, 85));

        let sym_m1 = find_symbol(1000).unwrap();
        let mut it_m1 = sym_m1.instantiate(next_id(), 24, 69, 0);
        it_m1.tag = "-M1".to_string();
        doc_pwr.items.push(it_m1);

        // Cabos das 3 fases L1, L2, L3 interligando os elementos em série
        let mut wires_mid = Vec::new();
        let mut wires_bot = Vec::new();
        for dx in [0, 6, 12] {
            let x = 24 + dx;
            doc_pwr
                .items
                .push(SchematicItem::new_wire(next_id(), 4000, x, 18, x, 24));
            let w_m = next_id();
            doc_pwr
                .items
                .push(SchematicItem::new_wire(w_m, 4000, x, 45, x, 51));
            wires_mid.push(w_m);
            let w_b = next_id();
            doc_pwr
                .items
                .push(SchematicItem::new_wire(w_b, 4000, x, 63, x, 69));
            wires_bot.push(w_b);
        }

        // Q1 aberto, KM2 aberto -> Motor Parado
        step_simulation(&mut doc_pwr, 50);
        let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(
            !m1.sim_active,
            "Potência: Motor desligado quando Q1 e KM2 abertos"
        );

        // Q1 fechado, KM2 aberto -> Eletricidade chega até a entrada de KM2, mas NÃO passa para o motor!
        set_manual_by_tag(&mut doc_pwr, "-Q1", true);
        step_simulation(&mut doc_pwr, 50);
        let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(
            !m1.sim_active,
            "Potência: Motor desligado se contator KM2 estiver aberto"
        );
        for &w in &wires_mid {
            assert!(doc_pwr
                .items
                .iter()
                .find(|i| i.id == w)
                .unwrap()
                .sim_potential
                .is_energized());
        }
        for &w in &wires_bot {
            assert_eq!(
                doc_pwr
                    .items
                    .iter()
                    .find(|i| i.id == w)
                    .unwrap()
                    .sim_potential,
                Potential::None
            );
        }

        // Q1 aberto, KM2 fechado -> Eletricidade NÃO passa por Q1, motor parado!
        set_manual_by_tag(&mut doc_pwr, "-Q1", false);
        set_manual_by_tag(&mut doc_pwr, "-SKM2", true);
        step_simulation(&mut doc_pwr, 50);
        let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(
            !m1.sim_active,
            "Potência: Motor desligado se disjuntor Q1 estiver aberto"
        );

        // Q1 fechado E KM2 fechado -> Eletricidade passa por ambos e motor gira!
        set_manual_by_tag(&mut doc_pwr, "-Q1", true);
        set_manual_by_tag(&mut doc_pwr, "-SKM2", true);
        step_simulation(&mut doc_pwr, 50);
        let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
        assert!(
            m1.sim_active,
            "Potência: Motor DEVE LIGAR quando Q1 E KM2 estiverem fechados"
        );
        assert_eq!(m1.sim_motor_dir, 1);
        for &w in &wires_bot {
            assert!(doc_pwr
                .items
                .iter()
                .find(|i| i.id == w)
                .unwrap()
                .sim_potential
                .is_energized());
        }
    }

    #[test]
    fn test_strict_or_parallel_conduction_command_and_power() {
        use crate::sim::set_manual_by_tag;

        // ==================== 1. CIRCUITO DE COMANDO EM PARALELO (OR / ||) ====================
        // Monta: L1 -> Nó J1
        //               |--> Ramo A: S1 (8000) -->|
        //               |--> Ramo B: S2 (8000) -->|--> Nó J2 -> Bobina KM1 -> Neutro N
        let mut doc = SchematicDocument::default();
        let mut id_counter = 1;
        let mut next_id = || {
            let cur = id_counter;
            id_counter += 1;
            cur
        };

        // Alimentação Fase L em (72, 10)
        let sym_l = find_symbol(3000).unwrap();
        let mut it_l = sym_l.instantiate(next_id(), 72, 10, 0);
        it_l.tag = "-L1".to_string();
        doc.items.push(it_l);

        // Chave S1 (NA) no ramo A em (72, 25): pin 0 em (72, 25), pin 1 em (72, 37)
        let sym_s1 = find_symbol(8000).unwrap();
        let mut it_s1 = sym_s1.instantiate(next_id(), 72, 25, 0);
        it_s1.tag = "-S1".to_string();
        doc.items.push(it_s1);

        // Chave S2 (NA) no ramo B em (84, 25): pin 0 em (84, 25), pin 1 em (84, 37)
        let sym_s2 = find_symbol(8000).unwrap();
        let mut it_s2 = sym_s2.instantiate(next_id(), 84, 25, 0);
        it_s2.tag = "-S2".to_string();
        doc.items.push(it_s2);

        // Bobina KM1 em (72, 50): pin 0 em (72, 50), pin 1 em (72, 62)
        let sym_km = find_symbol(9000).unwrap();
        let mut it_km = sym_km.instantiate(next_id(), 72, 50, 0);
        it_km.tag = "-KM1".to_string();
        doc.items.push(it_km);

        // Alimentação Neutro N em (72, 70, rot 2)
        let sym_n = find_symbol(3001).unwrap();
        let mut it_n = sym_n.instantiate(next_id(), 72, 70, 2);
        it_n.tag = "-N".to_string();
        doc.items.push(it_n);

        // Cabo Fase até o Nó J1 (72, 20)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 10, 72, 20));
        doc.items
            .push(SchematicItem::new_junction(next_id(), 72, 20));

        // Ramo A: (72, 20) -> (72, 25)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 20, 72, 25));
        // Saída Ramo A: (72, 37) -> (72, 45)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 37, 72, 45));

        // Ramo B: (72, 20) -> (84, 20) -> (84, 25)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 20, 84, 20));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 84, 20, 84, 25));
        // Saída Ramo B: (84, 37) -> (84, 45) -> (72, 45)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 84, 37, 84, 45));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 84, 45, 72, 45));
        doc.items
            .push(SchematicItem::new_junction(next_id(), 72, 45));

        // Cabo de J2 (72, 45) até Bobina A1 (72, 50)
        let w_out = next_id();
        doc.items
            .push(SchematicItem::new_wire(w_out, 4000, 72, 45, 72, 50));
        // Cabo Bobina A2 (72, 62) até Neutro (72, 70)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4009, 72, 62, 72, 70));

        // Caso 1: (S1=0, S2=0) -> Ambos abertos
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(!km.sim_active, "OR (0,0): Bobina deve estar DESLIGADA");
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_out)
                .unwrap()
                .sim_potential,
            Potential::None
        );

        // Caso 2: (S1=1, S2=0) -> S1 fechado, S2 aberto
        // A corrente flui pelo ramo A, alcança o nó J2 e liga a bobina!
        set_manual_by_tag(&mut doc, "-S1", true);
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(
            km.sim_active,
            "OR (1,0): Bobina DEVE LIGAR pelo ramo A fechado"
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_out)
                .unwrap()
                .sim_potential,
            Potential::L1
        );

        // Caso 3: (S1=0, S2=1) -> S1 aberto, S2 fechado
        // A corrente flui pelo ramo B, alcança o nó J2 e liga a bobina!
        set_manual_by_tag(&mut doc, "-S1", false);
        set_manual_by_tag(&mut doc, "-S2", true);
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(
            km.sim_active,
            "OR (0,1): Bobina DEVE LIGAR pelo ramo B fechado"
        );
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_out)
                .unwrap()
                .sim_potential,
            Potential::L1
        );

        // Caso 4: (S1=1, S2=1) -> Ambos fechados
        set_manual_by_tag(&mut doc, "-S1", true);
        set_manual_by_tag(&mut doc, "-S2", true);
        step_simulation(&mut doc, 50);
        let km = doc
            .items
            .iter()
            .find(|i| i.tag == "-KM1" && i.tipo == 9000)
            .unwrap();
        assert!(km.sim_active, "OR (1,1): Bobina DEVE LIGAR");
        assert_eq!(
            doc.items
                .iter()
                .find(|i| i.id == w_out)
                .unwrap()
                .sim_potential,
            Potential::L1
        );
    }

    #[test]
    fn test_ladder_and_fluid_power_boolean_equivalence() {
        use crate::sim::set_manual_by_tag;

        // Monta a lógica clássica: Saída Y = (A || B) && !C
        // Representa:
        // - Ladder: Rung com dois contatos NA em paralelo (A || B) em série com um contato NF (!C) acionando bobina Y
        // - Eletropneumática: Válvula alternadora (OR) alimentando válvula direcional normalmente aberta/fechada e atuando solenoide Y
        // - Eletrohidráulica: Bloqueio de emergência (C) com duplo comando de partida (A ou B)
        let mut doc = SchematicDocument::default();
        let mut id_counter = 1;
        let mut next_id = || {
            let cur = id_counter;
            id_counter += 1;
            cur
        };

        let sym_l = find_symbol(3000).unwrap();
        doc.items.push(sym_l.instantiate(next_id(), 72, 10, 0));

        // Botão de Parada de Emergência C (NF, 8001) em (72, 15): pin 0 em (72, 15), pin 1 em (72, 27)
        let sym_c = find_symbol(8001).unwrap();
        let mut it_c = sym_c.instantiate(next_id(), 72, 15, 0);
        it_c.tag = "-C".to_string();
        doc.items.push(it_c);

        // Bloco paralelo (A || B) abaixo de C:
        // Botão A (NA, 8000) em (72, 35): pin 0 em (72, 35), pin 1 em (72, 47)
        let sym_a = find_symbol(8000).unwrap();
        let mut it_a = sym_a.instantiate(next_id(), 72, 35, 0);
        it_a.tag = "-A".to_string();
        doc.items.push(it_a);

        // Botão B (NA, 8000) em (84, 35): pin 0 em (84, 35), pin 1 em (84, 47)
        let sym_b = find_symbol(8000).unwrap();
        let mut it_b = sym_b.instantiate(next_id(), 84, 35, 0);
        it_b.tag = "-B".to_string();
        doc.items.push(it_b);

        // Bobina Solenoide / Saída Y (9000) em (72, 60): pin 0 em (72, 60), pin 1 em (72, 72)
        let sym_y = find_symbol(9000).unwrap();
        let mut it_y = sym_y.instantiate(next_id(), 72, 60, 0);
        it_y.tag = "-Y".to_string();
        doc.items.push(it_y);

        let sym_n = find_symbol(3001).unwrap();
        doc.items.push(sym_n.instantiate(next_id(), 72, 80, 2));

        // Fiação:
        // L -> C (72, 10) -> (72, 15)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 10, 72, 15));
        // C saída (72, 27) -> Nó J1 (72, 30)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 27, 72, 30));
        doc.items
            .push(SchematicItem::new_junction(next_id(), 72, 30));

        // Ramo A: (72, 30) -> (72, 35) e saída (72, 47) -> (72, 55)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 30, 72, 35));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 47, 72, 55));

        // Ramo B: (72, 30) -> (84, 30) -> (84, 35) e saída (84, 47) -> (84, 55) -> (72, 55)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 30, 84, 30));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 84, 30, 84, 35));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 84, 47, 84, 55));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 84, 55, 72, 55));
        doc.items
            .push(SchematicItem::new_junction(next_id(), 72, 55));

        // J2 (72, 55) -> Y (72, 60) e Y (72, 72) -> N (72, 80)
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, 72, 55, 72, 60));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4009, 72, 72, 72, 80));

        // Valida todas as 8 combinações da tabela verdade: Y = (A || B) && !C_pressed
        for (a_val, b_val, c_pressed) in [
            (false, false, false), // 0, 0, !0 -> Y=0
            (false, true, false),  // 0, 1, !0 -> Y=1
            (true, false, false),  // 1, 0, !0 -> Y=1
            (true, true, false),   // 1, 1, !0 -> Y=1
            (false, false, true),  // 0, 0, !1 -> Y=0
            (false, true, true),   // 0, 1, !1 -> Y=0 (Emergência bloqueia mesmo com B ligado!)
            (true, false, true),   // 1, 0, !1 -> Y=0 (Emergência bloqueia mesmo com A ligado!)
            (true, true, true),    // 1, 1, !1 -> Y=0 (Emergência bloqueia mesmo com A e B ligados!)
        ] {
            set_manual_by_tag(&mut doc, "-A", a_val);
            set_manual_by_tag(&mut doc, "-B", b_val);
            set_manual_by_tag(&mut doc, "-C", c_pressed);
            step_simulation(&mut doc, 50);

            let expected = (a_val || b_val) && !c_pressed;
            let y_active = doc
                .items
                .iter()
                .find(|i| i.tag == "-Y" && i.tipo == 9000)
                .unwrap()
                .sim_active;
            assert_eq!(
                y_active, expected,
                "Falha na tabela verdade para A={}, B={}, C_pressed={}: esperado {}, obtido {}",
                a_val, b_val, c_pressed, expected, y_active
            );
        }
    }

    #[test]
    fn test_smart_sequential_pin_dropdown_and_context_reading() {
        use crate::symbols::ContactKind;
        let (mut app, _) = CadeSimuApp::new();
        app.doc.items.clear();

        // Valida ContactKind e geração dos bornes conforme a norma
        assert_eq!(ContactKind::NO.generate_pins(1), vec!["13", "14"]);
        assert_eq!(ContactKind::NO.generate_pins(2), vec!["23", "24"]);
        assert_eq!(ContactKind::NC.generate_pins(1), vec!["11", "12"]);
        assert_eq!(ContactKind::NC.generate_pins(2), vec!["21", "22"]);

        // 1. Testa seleção sequencial em Botão NA (tipo 8000)
        let sym_btn_na = find_symbol(8000).expect("Botão NA deve existir no catálogo");
        let id_btn1 = 1001;
        let mut btn1 = sym_btn_na.instantiate(id_btn1, 100, 100, 0);
        btn1.tag = "-S1".to_string();
        app.doc.items.push(btn1);
        app.selected_id = Some(id_btn1);

        // Por padrão: 13 e 14
        let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
        assert_eq!(item.pins[0], "13");
        assert_eq!(item.pins[1], "14");

        // Seleciona Sequência 2 -> 23 e 24
        let _ = app.update(Message::SelectPinSequence(2));
        let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
        assert_eq!(
            item.pins[0], "23",
            "Borne 1 deve atualizar para 23 na sequência 2"
        );
        assert_eq!(
            item.pins[1], "24",
            "Borne 2 deve atualizar para 24 na sequência 2"
        );

        // Seleciona Sequência 3 -> 33 e 34
        let _ = app.update(Message::SelectPinSequence(3));
        let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
        assert_eq!(item.pins[0], "33");
        assert_eq!(item.pins[1], "34");

        // Seleciona Sequência 1 -> volta para 13 e 14
        let _ = app.update(Message::SelectPinSequence(1));
        let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
        assert_eq!(item.pins[0], "13");
        assert_eq!(item.pins[1], "14");

        // 2. Testa seleção sequencial em Botão NF (tipo 8001)
        let sym_btn_nf = find_symbol(8001).expect("Botão NF deve existir no catálogo");
        let id_btn2 = 1002;
        let mut btn2 = sym_btn_nf.instantiate(id_btn2, 120, 100, 0);
        btn2.tag = "-S1".to_string();
        app.doc.items.push(btn2);
        app.selected_id = Some(id_btn2);

        // Seleciona Sequência 2 em NF -> 21 e 22
        let _ = app.update(Message::SelectPinSequence(2));
        let item = app.doc.items.iter().find(|i| i.id == id_btn2).unwrap();
        assert_eq!(
            item.pins[0], "21",
            "Borne 1 de NF na sequência 2 deve ser 21"
        );
        assert_eq!(
            item.pins[1], "22",
            "Borne 2 de NF na sequência 2 deve ser 22"
        );

        // 3. Testa edição livre de texto nos bornes (inputs manuais continuam livres)
        let _ = app.update(Message::EditPin(0, "COM".to_string()));
        let _ = app.update(Message::EditPin(1, "NO".to_string()));
        let item = app.doc.items.iter().find(|i| i.id == id_btn2).unwrap();
        assert_eq!(item.pins[0], "COM");
        assert_eq!(item.pins[1], "NO");

        // 4. Testa Leitura Inteligente do Contexto para Contatos Auxiliares (-KM1)
        let sym_cont_na = find_symbol(7000).expect("Contato NA deve existir");
        let sym_cont_nf = find_symbol(7001).expect("Contato NF deve existir");

        let id_km_na1 = 2001;
        let mut cont_na1 = sym_cont_na.instantiate(id_km_na1, 200, 100, 0);
        cont_na1.tag = "-KM1".to_string();
        cont_na1.pins[0] = "13".to_string();
        cont_na1.pins[1] = "14".to_string(); // Sequência 1 ocupada
        app.doc.items.push(cont_na1);

        let id_km_nf1 = 2002;
        let mut cont_nf1 = sym_cont_nf.instantiate(id_km_nf1, 250, 100, 0);
        cont_nf1.tag = "-KM1".to_string();
        // Inicialmente também na sequência 1 (conflito)
        cont_nf1.pins[0] = "11".to_string();
        cont_nf1.pins[1] = "12".to_string();
        app.doc.items.push(cont_nf1);

        // Contexto: quais sequências de -KM1 estão ocupadas excluindo o próprio cont_nf1?
        let occupied = find_occupied_sequences_for_tag(&app.doc.items, "-KM1", Some(id_km_nf1));
        assert!(
            occupied.contains(&1),
            "Sequência 1 deve ser detectada como ocupada pelo cont_na1"
        );
        assert!(!occupied.contains(&2), "Sequência 2 deve estar livre");

        // Próxima sequência sugerida deve ser 2
        let suggested = (1..=9).find(|d| !occupied.contains(d)).unwrap();
        assert_eq!(
            suggested, 2,
            "A próxima sequência sugerida para -KM1 deve ser 2"
        );

        // Seleciona o contato NF com conflito e aplica a sequência sugerida 2
        app.selected_id = Some(id_km_nf1);
        let _ = app.update(Message::SelectPinSequence(suggested));
        let item = app.doc.items.iter().find(|i| i.id == id_km_nf1).unwrap();
        assert_eq!(item.pins[0], "21");
        assert_eq!(item.pins[1], "22");

        // Agora tanto 1 quanto 2 estão ocupados em -KM1
        let occupied_all = find_occupied_sequences_for_tag(&app.doc.items, "-KM1", None);
        assert!(occupied_all.contains(&1));
        assert!(occupied_all.contains(&2));
        let next_suggested = (1..=9).find(|d| !occupied_all.contains(d)).unwrap();
        assert_eq!(
            next_suggested, 3,
            "Novo contato de -KM1 deve sugerir sequência 3"
        );

        // 5. Testa Bobinas: bornes SEMPRE A1-A2 e sequenciamento de tags (-KM1, -KM2)
        let sym_coil = find_symbol(9000).expect("Bobina deve existir");
        // Bobina NUNCA deve ser classificada como contato com sequência aberta/fechada
        assert!(
            detect_contact_kind(9000, &sym_coil.pins).is_none(),
            "Bobina não deve ter ContactKind"
        );

        // Insere primeira bobina no Canvas -> deve ser -KM1 e bornes A1-A2
        let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
            tipo: 9000,
            x: 400,
            y: 100,
        }));
        let coil1 = app.doc.items.last().unwrap();
        assert_eq!(coil1.tag, "-KM1", "Primeira bobina deve receber tag -KM1");
        assert_eq!(coil1.pins[0], "A1", "Borne 1 da bobina DEVE ser A1");
        assert_eq!(coil1.pins[1], "A2", "Borne 2 da bobina DEVE ser A2");

        // Insere segunda bobina no Canvas -> deve ser -KM2 e bornes A1-A2
        let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
            tipo: 9000,
            x: 450,
            y: 100,
        }));
        let (coil2_id, coil2_tag, coil2_p0, coil2_p1) = {
            let c2 = app.doc.items.last().unwrap();
            (
                c2.id,
                c2.tag.clone(),
                c2.pins[0].clone(),
                c2.pins[1].clone(),
            )
        };
        assert_eq!(
            coil2_tag, "-KM2",
            "Segunda bobina deve receber tag sequencial -KM2"
        );
        assert_eq!(coil2_p0, "A1", "Borne 1 da segunda bobina DEVE ser A1");
        assert_eq!(coil2_p1, "A2", "Borne 2 da segunda bobina DEVE ser A2");

        // Insere botões/interruptores no Canvas -> -S1, -S2...
        let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
            tipo: 8000,
            x: 500,
            y: 100,
        }));
        let btn_s = app.doc.items.last().unwrap();
        assert!(
            btn_s.tag.starts_with("-S"),
            "Botão deve iniciar com prefixo -S"
        );

        // 6. Testa verificação de contato com e sem bobina
        let mut orphan_contact = sym_cont_na.instantiate(3001, 600, 100, 0);
        orphan_contact.tag = "-KM99".to_string(); // Bobina inexistente no esquema
        assert!(
            find_parent_apparatus(&app.doc.items, &orphan_contact).is_none(),
            "Contato -KM99 deve ser identificado como sem bobina"
        );

        let mut linked_contact = sym_cont_na.instantiate(3002, 650, 100, 0);
        linked_contact.tag = "-KM1".to_string(); // Bobina existente
        assert!(
            find_parent_apparatus(&app.doc.items, &linked_contact).is_some(),
            "Contato -KM1 deve encontrar a bobina -KM1"
        );

        // 7. Testa duplicação de aparelhos (Ctrl+D)
        app.selected_id = Some(coil2_id); // seleciona -KM2
        let _ = app.update(Message::Duplicate);
        let duplicated_coil = app.doc.items.last().unwrap();
        assert_eq!(
            duplicated_coil.tag, "-KM3",
            "Duplicar bobina -KM2 deve gerar -KM3"
        );
        assert_eq!(duplicated_coil.pins[0], "A1");
        assert_eq!(duplicated_coil.pins[1], "A2");

        // 8. Renderiza a view garantindo que o pick_list e os avisos são gerados sem pânico
        let _ = app.view();
    }

    #[test]
    fn test_drag_move_single_undo_step() {
        let (mut app, _) = CadeSimuApp::new();
        app.doc.items.clear();

        // 1. Posiciona componente inicial em (100, 100)
        let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
            tipo: 9000,
            x: 100,
            y: 100,
        }));
        let item_id = app.doc.items[0].id;
        assert_eq!(app.doc.items[0].x1, 100);
        assert_eq!(app.doc.items[0].y1, 100);
        let undo_len_after_place = app.undo_stack.len();

        // 2. Simula o arraste suave do componente passando por múltiplos pontos da grade até (180, 100)
        let intermediate_steps = [110, 120, 130, 140, 150, 160, 170, 180];
        for &step_x in &intermediate_steps {
            let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
                id: item_id,
                x1: step_x,
                y1: 100,
            }));
            // Durante o arraste contínuo, a pilha de undo NÃO deve acumular passos intermediários!
            assert_eq!(
                app.undo_stack.len(),
                undo_len_after_place,
                "Passos intermediários de arraste não podem poluir a pilha de Ctrl+Z"
            );
            assert_eq!(app.doc.items[0].x1, step_x);
        }

        // 3. Usuário solta o botão do mouse -> EndMoveItem
        let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: item_id }));

        // Apenas UM snapshot único deve ser adicionado ao histórico
        assert_eq!(
            app.undo_stack.len(),
            undo_len_after_place + 1,
            "Apenas uma transação de movimento deve ser registrada ao soltar o mouse"
        );
        assert_eq!(app.doc.items[0].x1, 180);

        // 4. Um ÚNICO Ctrl+Z (Undo) deve retornar o elemento diretamente da posição final (180, 100) para a inicial (100, 100)
        let _ = app.update(Message::Undo);
        assert_eq!(
            app.doc.items[0].x1, 100,
            "Um único Ctrl+Z deve retornar o elemento diretamente para o ponto inicial (100, 100)"
        );
        assert_eq!(app.doc.items[0].y1, 100);

        // 5. Um ÚNICO Ctrl+Y (Redo) deve mover de volta para a posição final (180, 100)
        let _ = app.update(Message::Redo);
        assert_eq!(
            app.doc.items[0].x1, 180,
            "Um único Ctrl+Y deve avançar o elemento para a posição final (180, 100)"
        );
        assert_eq!(app.doc.items[0].y1, 100);
    }

    #[test]
    fn test_drag_move_cancel_and_wire_tracking() {
        let (mut app, _) = CadeSimuApp::new();
        app.doc.items.clear();

        // Insere componente e fio conectado
        let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
            tipo: 9000,
            x: 100,
            y: 100,
        }));
        let comp_id = app.doc.items[0].id;
        let _ = app.update(Message::Canvas(CanvasMessage::AddWire {
            tipo: 4000,
            x1: 100,
            y1: 100,
            x2: 100,
            y2: 50,
        }));
        let wire_id = app.doc.items.iter().find(|i| i.is_wire()).unwrap().id;
        let initial_undo_len = app.undo_stack.len();

        // 1. Arraste cancelado via ESC / Botão Direito (CancelAction)
        let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
            id: comp_id,
            x1: 140,
            y1: 100,
        }));
        assert_eq!(
            app.doc.items.iter().find(|i| i.id == comp_id).unwrap().x1,
            140
        );
        let _ = app.update(Message::Canvas(CanvasMessage::CancelAction));
        assert_eq!(
            app.doc.items.iter().find(|i| i.id == comp_id).unwrap().x1,
            100,
            "Cancelar deve reverter o elemento à posição de origem imediatamente"
        );
        assert_eq!(
            app.undo_stack.len(),
            initial_undo_len,
            "Cancelar arraste não deve criar entrada na pilha de undo"
        );

        // 2. Arraste concluído com fio conectado
        let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
            id: comp_id,
            x1: 130,
            y1: 100,
        }));
        let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: comp_id }));
        assert_eq!(app.undo_stack.len(), initial_undo_len + 1);

        // Fio deve ter acompanhado o componente para x1 = 130
        let wire_moved = app.doc.items.iter().find(|i| i.id == wire_id).unwrap();
        assert_eq!(wire_moved.x1, 130);

        // Desfazer (Ctrl+Z) restaura ambos para x1 = 100 em um passo
        let _ = app.update(Message::Undo);
        let comp_undone = app.doc.items.iter().find(|i| i.id == comp_id).unwrap();
        let wire_undone = app.doc.items.iter().find(|i| i.id == wire_id).unwrap();
        assert_eq!(comp_undone.x1, 100);
        assert_eq!(wire_undone.x1, 100);

        // 3. Clique simples sem deslocamento não deve criar histórico de undo
        let pre_click_undo = app.undo_stack.len();
        let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: comp_id }));
        assert_eq!(
            app.undo_stack.len(),
            pre_click_undo,
            "Clique sem deslocamento não deve adicionar entrada na pilha de undo"
        );
    }

    #[test]
    fn test_teste_cad_loading_and_mushroom_button_dj() {
        let cad_path = "Projetos/teste.cad";
        let content =
            std::fs::read_to_string(cad_path).expect("teste.cad deve existir na pasta Projetos");
        let doc = crate::io::parse_cad_str(&content).expect("teste.cad deve carregar com sucesso");

        // Verifica que o elemento 139 existe no arquivo e foi decodificado como 8008 (Interruptor NA / DJ)
        let item_dj = doc
            .items
            .iter()
            .find(|i| i.id == 139)
            .expect("Item #139 (DJ) deve estar presente no teste.cad");
        assert_eq!(
            item_dj.tipo, 8008,
            "Item #139 deve ser tipo 8008 (Contacto interruptor NA - DJ)"
        );
        assert_eq!(item_dj.tag.trim(), "DJ", "Tag do elemento #139 deve ser DJ");
        assert_eq!(item_dj.pins[0].trim(), "13", "Pino 0 deve ser 13");
        assert_eq!(item_dj.pins[1].trim(), "14", "Pino 1 deve ser 14");

        // Verifica que o símbolo existe no catálogo do programa com primitivas vetoriais completas
        let sym = find_symbol(8008).expect("Símbolo 8008 deve existir no catálogo de símbolos");
        assert_eq!(
            sym.category,
            crate::symbols::SymbolCategory::PushbuttonsActuators
        );
        assert!(
            sym.name.contains("Interruptor") || sym.name.contains("Chave"),
            "8008 deve ser Interruptor / Chave"
        );
        assert!(
            !sym.idle_cmds.is_empty(),
            "Comandos idle não podem ser vazios"
        );
        assert!(
            !sym.active_cmds.is_empty(),
            "Comandos active não podem ser vazios"
        );
        // Verifica que contém o comando do traço degrau de retenção do interruptor (LINE,-030,+020,-030,+040)
        assert!(
            sym.idle_cmds
                .iter()
                .any(|cmd| cmd.contains("-030,+020,-030,+040")),
            "Deve conter degrau retentivo do interruptor"
        );
        assert_eq!(sym.pins.len(), 2, "Interruptor NA tem 2 pinos");
    }

    #[test]
    fn test_actuators_and_switches_catalog_completeness() {
        let expected_types = [
            8000, 8001, 8002, 8003, // Botões pulsadores
            8004, 8005, 8006, 8007, // Cogumelos pulsadores
            8008, 8009, 8010, 8011, // Chaves / Interruptores retentivos (8008 = DJ)
            8012, 8013, 8014, 8015, // Fim de Curso pulsadores
            8016, 8017, 8018, 8019, // Relé Térmico auxiliares
            8020, 8021, 8022, // Chaves Seletoras I-0-II, I-II-III, Inversora
            8023, // Botão Pulsador 2D
            8030, 8031, 8032, 8033,
            8034, // Botões de Parada de Emergência Retentivos com travas
        ];

        for &tipo in &expected_types {
            let sym = find_symbol(tipo)
                .unwrap_or_else(|| panic!("Tipo {} deve existir no catálogo", tipo));
            assert!(
                !sym.idle_cmds.is_empty(),
                "Tipo {} deve ter comandos idle",
                tipo
            );
            assert!(
                !sym.active_cmds.is_empty(),
                "Tipo {} deve ter comandos active",
                tipo
            );
            if tipo != 8023 {
                assert!(
                    !sym.pins.is_empty(),
                    "Tipo {} deve ter pinos declarados",
                    tipo
                );
            }
        }
    }

    #[test]
    fn test_actuator_switches_latching_and_momentary_distinction() {
        use crate::sim::{is_momentary_button, press_component_on_sim, release_component_on_sim};

        // 1. Verifica momentâneos (pulsadores simples, cogumelos simples 8004..=8007, fins de curso 8012..=8015)
        for &m in &[
            8000, 8001, 8002, 8003, 8004, 8005, 8006, 8007, 8012, 8013, 8014, 8015, 8023,
        ] {
            assert!(
                is_momentary_button(m),
                "Tipo {} deve ser momentâneo (diferencial/pulsador)",
                m
            );
        }
        // 2. Verifica retentivos / bi-estáveis (interruptores 8008..=8011, chaves 8020..=8022, emergências 8030..=8034)
        for &b in &[
            8008, 8009, 8010, 8011, 8020, 8021, 8022, 8030, 8031, 8032, 8033, 8034,
        ] {
            assert!(
                !is_momentary_button(b),
                "Tipo {} deve ser bi-estável / retentivo",
                b
            );
        }

        let mut doc = crate::model::SchematicDocument::default();
        doc.items.clear();

        // 3. Insere um botão pulsador momentâneo (8000), um interruptor retentivo (8008) e uma emergência NF (8030)
        let mut btn_mom = crate::model::SchematicItem::new_symbol(1, 8000, 50, 50, "-S1");
        btn_mom.pins[0] = "13".to_string();
        btn_mom.pins[1] = "14".to_string();
        let mut sw_ret = crate::model::SchematicItem::new_symbol(2, 8008, 100, 50, "-DJ");
        sw_ret.pins[0] = "13".to_string();
        sw_ret.pins[1] = "14".to_string();
        let mut emg_nc = crate::model::SchematicItem::new_symbol(3, 8030, 150, 50, "-S0");
        emg_nc.pins[0] = "11".to_string();
        emg_nc.pins[1] = "12".to_string();

        doc.items.push(btn_mom);
        doc.items.push(sw_ret);
        doc.items.push(emg_nc);

        // Pressiona o botão momentâneo -> fica ativo
        press_component_on_sim(&mut doc, 1);
        assert!(
            doc.items[0].sim_manual_latched,
            "Botão momentâneo deve ficar ativo ao pressionar"
        );

        // Solta o botão momentâneo -> volta a repouso
        release_component_on_sim(&mut doc, 1);
        assert!(
            !doc.items[0].sim_manual_latched,
            "Botão momentâneo deve soltar ao liberar o mouse"
        );

        // Pressiona a chave interruptor (8008) -> retém ativo
        press_component_on_sim(&mut doc, 2);
        assert!(
            doc.items[1].sim_manual_latched,
            "Chave interruptor deve reter ativo"
        );

        // Solta a chave interruptor -> DEVE CONTINUAR ATIVO (retentivo/bi-estável)
        release_component_on_sim(&mut doc, 2);
        assert!(
            doc.items[1].sim_manual_latched,
            "Chave interruptor deve permanecer retida após soltar mouse"
        );

        // Segundo clique na chave interruptor -> desativa (alterna)
        press_component_on_sim(&mut doc, 2);
        assert!(
            !doc.items[1].sim_manual_latched,
            "Segundo clique desativa chave interruptor"
        );

        // Pressiona o botão de emergência retentivo (8030)
        press_component_on_sim(&mut doc, 3);
        assert!(
            doc.items[2].sim_manual_latched,
            "Botão de emergência deve travar ao pressionar"
        );
        release_component_on_sim(&mut doc, 3);
        assert!(
            doc.items[2].sim_manual_latched,
            "Botão de emergência retém travado ao soltar mouse"
        );
        press_component_on_sim(&mut doc, 3);
        assert!(
            !doc.items[2].sim_manual_latched,
            "Segundo clique destrava o botão de emergência"
        );
    }

    #[test]
    fn test_canvas_selection_prioritizes_symbols_over_wires() {
        use crate::canvas_view::SchematicCanvas;

        let mut doc = crate::model::SchematicDocument::default();
        doc.items.clear();

        // Cria símbolo DJ (8008) em (50, 50)
        let sym_dj = find_symbol(8008).unwrap();
        let item_dj = sym_dj.instantiate(100, 50, 50, 0);
        let wp = sym_dj.world_pins(&item_dj);

        // Cria fios conectados exatamente nos pinos do símbolo
        let wire1 = crate::model::SchematicItem::new_wire(
            201,
            4000,
            wp[0].2 .0,
            wp[0].2 .1 - 20,
            wp[0].2 .0,
            wp[0].2 .1,
        );
        let wire2 = crate::model::SchematicItem::new_wire(
            202,
            4000,
            wp[1].2 .0,
            wp[1].2 .1,
            wp[1].2 .0,
            wp[1].2 .1 + 20,
        );

        // Fio horizontal cruzando por trás do símbolo
        let wire3 = crate::model::SchematicItem::new_wire(203, 4000, 30, 55, 70, 55);

        doc.items.push(wire1);
        doc.items.push(wire2);
        doc.items.push(wire3);
        doc.items.push(item_dj);

        let canvas = SchematicCanvas {
            doc: &doc,
            tool: crate::EditorTool::Select,
            selected_id: None,
            sim_running: false,
            show_grid: true,
            grid_density: crate::model::GridDensity::Medium,
            show_texts: true,
            is_dark: false,
            auto_connect: true,
            auto_disconnect: false,
            zoom: 1.0,
            pan: iced::Vector::ZERO,
            placement_orientation: 0,
            dragging_symbol: None,
            is_blocked: false,
        };

        // Clicar no corpo do símbolo deve retornar o ID do símbolo (100) e JAMAIS o ID de um fio
        let hit = canvas.hit_test(50, 55);
        assert_eq!(
            hit,
            Some(100),
            "Hit test no centro do símbolo deve selecionar o símbolo e não fios adjacentes"
        );

        let hit_pin = canvas.hit_test(wp[0].2 .0, wp[0].2 .1);
        assert_eq!(
            hit_pin,
            Some(100),
            "Hit test no pino do símbolo deve priorizar o símbolo sobre o fio conectado"
        );

        // Clicar longe do símbolo, em ponto exclusivo do fio
        let hit_wire = canvas.hit_test(wp[0].2 .0, wp[0].2 .1 - 15);
        assert_eq!(
            hit_wire,
            Some(201),
            "Hit test longe do símbolo deve selecionar o fio"
        );
    }

    #[test]
    fn test_crossover_switch_and_limit_switches_conduction() {
        use crate::model::Potential;
        use crate::sim::step_simulation;

        let mut doc = crate::model::SchematicDocument::default();
        doc.items.clear();

        let sym_l1 = find_symbol(3000).unwrap();
        let sym_n = find_symbol(3001).unwrap();
        let sym_cross = find_symbol(8022).unwrap();

        let src_l1 = sym_l1.instantiate(1, 50, 40, 0);
        let src_n = sym_n.instantiate(2, 56, 40, 0);
        let cross = sym_cross.instantiate(3, 50, 50, 0);

        let wp_l1 = sym_l1.world_pins(&src_l1);
        let wp_n = sym_n.world_pins(&src_n);
        let wp_cross = sym_cross.world_pins(&cross);

        let pt_l1 = wp_l1[0].2;
        let pt_n = wp_n[0].2;
        let pt_c0 = wp_cross[0].2; // pino 0 ("1")
        let pt_c1 = wp_cross[1].2; // pino 1 ("3")
        let pt_c2 = wp_cross[2].2; // pino 2 ("2")
        let pt_c3 = wp_cross[3].2; // pino 3 ("4")

        doc.items.push(src_l1);
        doc.items.push(src_n);
        doc.items.push(cross);

        // Fios conectando L1 ao pino 0 e N ao pino 1
        doc.items.push(crate::model::SchematicItem::new_wire(
            4, 4000, pt_l1.0, pt_l1.1, pt_c0.0, pt_c0.1,
        ));
        doc.items.push(crate::model::SchematicItem::new_wire(
            5, 4002, pt_n.0, pt_n.1, pt_c1.0, pt_c1.1,
        ));

        // Cabos de saída a partir dos pinos 2 e 3
        doc.items.push(crate::model::SchematicItem::new_wire(
            6,
            4000,
            pt_c2.0,
            pt_c2.1,
            pt_c2.0,
            pt_c2.1 + 20,
        ));
        doc.items.push(crate::model::SchematicItem::new_wire(
            7,
            4002,
            pt_c3.0,
            pt_c3.1,
            pt_c3.0,
            pt_c3.1 + 20,
        ));

        // Executa simulação em repouso (cross.sim_manual_latched = false)
        // No repouso: conexão direta -> pino 0 (L1) liga ao pino 2 (fio 6), pino 1 (N) liga ao pino 3 (fio 7)
        step_simulation(&mut doc, 50);

        let pot_out_left = doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential;
        let pot_out_right = doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential;
        assert_eq!(
            pot_out_left,
            Potential::L1,
            "Em repouso, saída esquerda deve ser L1"
        );
        assert_eq!(
            pot_out_right,
            Potential::N,
            "Em repouso, saída direita deve ser N"
        );

        // Ativa a chave inversora (cross.sim_manual_latched = true)
        // No ativo: conexão cruzada -> pino 0 (L1) liga ao pino 3 (fio 7), pino 1 (N) liga ao pino 2 (fio 6)
        doc.items
            .iter_mut()
            .find(|i| i.id == 3)
            .unwrap()
            .sim_manual_latched = true;
        step_simulation(&mut doc, 50);

        let pot_out_left_inv = doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential;
        let pot_out_right_inv = doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential;
        assert_eq!(
            pot_out_left_inv,
            Potential::N,
            "Ao inverter, saída esquerda deve trocar para N"
        );
        assert_eq!(
            pot_out_right_inv,
            Potential::L1,
            "Ao inverter, saída direita deve trocar para L1"
        );
    }

    #[test]
    fn test_limit_switches_and_thermal_aux_contacts() {
        use crate::model::Potential;
        use crate::sim::step_simulation;

        let mut doc = crate::model::SchematicDocument::default();
        doc.items.clear();

        // 1. Fonte L1 em (50, 40)
        let sym_l1 = find_symbol(3000).unwrap();
        let src_l1 = sym_l1.instantiate(1, 50, 40, 0);
        let pt_l1 = sym_l1.world_pins(&src_l1)[0].2;
        doc.items.push(src_l1);

        // 2. Fim de curso NA (8012) e NF (8013)
        let sym_no = find_symbol(8012).unwrap();
        let sym_nc = find_symbol(8013).unwrap();
        let lim_no = sym_no.instantiate(2, 50, 50, 0);
        let lim_nc = sym_nc.instantiate(3, 80, 50, 0);

        let wp_no = sym_no.world_pins(&lim_no);
        let wp_nc = sym_nc.world_pins(&lim_nc);

        doc.items.push(lim_no);
        doc.items.push(lim_nc);

        // Fios de entrada: L1 -> pino 0 de lim_no e lim_nc
        doc.items.push(crate::model::SchematicItem::new_wire(
            4,
            4000,
            pt_l1.0,
            pt_l1.1,
            wp_no[0].2 .0,
            wp_no[0].2 .1,
        ));
        doc.items.push(crate::model::SchematicItem::new_wire(
            5,
            4000,
            pt_l1.0,
            pt_l1.1,
            wp_nc[0].2 .0,
            wp_nc[0].2 .1,
        ));

        // Fios de saída: pino 1 -> carga
        doc.items.push(crate::model::SchematicItem::new_wire(
            6,
            4000,
            wp_no[1].2 .0,
            wp_no[1].2 .1,
            wp_no[1].2 .0,
            wp_no[1].2 .1 + 20,
        ));
        doc.items.push(crate::model::SchematicItem::new_wire(
            7,
            4000,
            wp_nc[1].2 .0,
            wp_nc[1].2 .1,
            wp_nc[1].2 .0,
            wp_nc[1].2 .1 + 20,
        ));

        // Em repouso: NO aberto (saída None), NC fechado (saída L1)
        step_simulation(&mut doc, 50);
        assert_eq!(
            doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential,
            Potential::None
        );
        assert_eq!(
            doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential,
            Potential::L1
        );

        // Ativa ambos os fins de curso: NO fecha (saída L1), NC abre (saída None)
        doc.items
            .iter_mut()
            .find(|i| i.id == 2)
            .unwrap()
            .sim_manual_latched = true;
        doc.items
            .iter_mut()
            .find(|i| i.id == 3)
            .unwrap()
            .sim_manual_latched = true;
        step_simulation(&mut doc, 50);
        assert_eq!(
            doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential,
            Potential::L1
        );
        assert_eq!(
            doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential,
            Potential::None
        );

        // 3. Testa atuação do relé térmico pai (6007) sobre contato auxiliar térmico duplo (8018)
        let sym_f = find_symbol(6007).unwrap();
        let sym_f_aux = find_symbol(8018).unwrap();
        let mut r_term = sym_f.instantiate(10, 120, 50, 0);
        r_term.tag = "-F1".to_string();
        let mut aux_term = sym_f_aux.instantiate(11, 150, 50, 0);
        aux_term.tag = "-F1".to_string();

        let wp_aux = sym_f_aux.world_pins(&aux_term);
        // pino 0 ("95") e pino 2 ("96") formam o contato NF
        // pino 1 ("97") e pino 3 ("98") formam o contato NA
        doc.items.push(r_term);
        doc.items.push(aux_term);

        // Alimenta 95 (NF) e 97 (NA) com L1
        doc.items.push(crate::model::SchematicItem::new_wire(
            12,
            4000,
            pt_l1.0,
            pt_l1.1,
            wp_aux[0].2 .0,
            wp_aux[0].2 .1,
        ));
        doc.items.push(crate::model::SchematicItem::new_wire(
            13,
            4000,
            pt_l1.0,
            pt_l1.1,
            wp_aux[1].2 .0,
            wp_aux[1].2 .1,
        ));
        // Saídas de 96 (NF) e 98 (NA)
        doc.items.push(crate::model::SchematicItem::new_wire(
            14,
            4000,
            wp_aux[2].2 .0,
            wp_aux[2].2 .1,
            wp_aux[2].2 .0,
            wp_aux[2].2 .1 + 20,
        ));
        doc.items.push(crate::model::SchematicItem::new_wire(
            15,
            4000,
            wp_aux[3].2 .0,
            wp_aux[3].2 .1,
            wp_aux[3].2 .0,
            wp_aux[3].2 .1 + 20,
        ));

        // Em repouso do relé térmico: 95-96 conduz (fio 14 = L1), 97-98 aberto (fio 15 = None)
        step_simulation(&mut doc, 50);
        assert_eq!(
            doc.items.iter().find(|i| i.id == 14).unwrap().sim_potential,
            Potential::L1
        );
        assert_eq!(
            doc.items.iter().find(|i| i.id == 15).unwrap().sim_potential,
            Potential::None
        );

        // Dispara o relé térmico principal -F1 -> o contato auxiliar 8018 comuta automaticamente via tag!
        doc.items
            .iter_mut()
            .find(|i| i.id == 10)
            .unwrap()
            .sim_manual_latched = true;
        step_simulation(&mut doc, 50);
        assert_eq!(
            doc.items.iter().find(|i| i.id == 14).unwrap().sim_potential,
            Potential::None,
            "95-96 abre quando -F1 desarma"
        );
        assert_eq!(
            doc.items.iter().find(|i| i.id == 15).unwrap().sim_potential,
            Potential::L1,
            "97-98 fecha quando -F1 desarma"
        );
    }

    #[test]
    fn test_dock_panels_physical_collapse_and_expand() {
        let (mut app, _) = CadeSimuApp::new();

        // Inicialmente ambos os painéis estão expandidos (288px)
        assert!(!app.left_dock_collapsed);
        assert!(!app.right_dock_collapsed);
        assert_eq!(app.layout.left_width, 288.0);
        assert_eq!(app.layout.right_width, 288.0);

        // Renderiza a interface normalmente
        let _ = app.view();

        // 1. Recolhe o painel esquerdo (Elementos e Projetos)
        let _ = app.update(Message::ToggleLeftDockCollapse);
        assert!(app.left_dock_collapsed);
        let _ = app.view();

        // 2. Recolhe o painel direito (Propriedades do Elemento)
        let _ = app.update(Message::ToggleRightDockCollapse);
        assert!(app.right_dock_collapsed);
        let _ = app.view();

        // 3. Expande o painel esquerdo novamente
        let _ = app.update(Message::ToggleLeftDockCollapse);
        assert!(!app.left_dock_collapsed);
        let _ = app.view();

        // 4. Expande o painel direito novamente
        let _ = app.update(Message::ToggleRightDockCollapse);
        assert!(!app.right_dock_collapsed);
        let _ = app.view();
    }

    #[test]
    fn test_dock_panels_resize_drag_and_clamp() {
        let (mut app, _) = CadeSimuApp::new();

        // 1. Testa redimensionamento do painel esquerdo
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragStart(
            crate::layout::Side::Left,
        )));
        assert_eq!(app.layout.drag, Some(crate::layout::Side::Left));

        // Move para 350px (dentro do intervalo 160..=480)
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragMove(350.0)));
        assert_eq!(app.layout.left_width, 350.0);

        // Clamping inferior: tenta mover para 50px -> limita a 160px
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragMove(50.0)));
        assert_eq!(app.layout.left_width, 160.0);

        // Clamping superior: tenta mover para 600px -> limita a 480px
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragMove(600.0)));
        assert_eq!(app.layout.left_width, 480.0);

        // Solta o botão do mouse
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragEnd));
        assert_eq!(app.layout.drag, None);

        // 2. Testa redimensionamento do painel direito (largura da janela = 1420px)
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragStart(
            crate::layout::Side::Right,
        )));
        assert_eq!(app.layout.drag, Some(crate::layout::Side::Right));

        // Posiciona o cursor em x = 1100 -> right_width = 1420 - 1100 = 320px
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragMove(1100.0)));
        assert_eq!(app.layout.right_width, 320.0);

        // Clamping inferior do painel direito: x = 1380 -> 1420 - 1380 = 40px -> limita a 160px
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragMove(1380.0)));
        assert_eq!(app.layout.right_width, 160.0);

        // Clamping superior do painel direito: x = 800 -> 1420 - 800 = 620px -> limita a 480px
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragMove(800.0)));
        assert_eq!(app.layout.right_width, 480.0);

        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::DragEnd));
        assert_eq!(app.layout.drag, None);

        // 3. Testa redimensionamento da janela
        let _ = app.update(Message::LayoutMsg(crate::layout::Msg::WindowResized(
            1600.0,
        )));
        assert_eq!(app.layout.window_w, 1600.0);

        // 4. Renderiza view com painéis redimensionados
        let _ = app.view();
    }

    #[test]
    fn test_toolbar_responsive_and_icon_buttons() {
        let (mut app, _) = CadeSimuApp::new();

        // Renderiza toolbar com tema padrão
        let _ = app.view_toolbar();

        // Alterna tema via CycleTheme (o botão de tema agora é ícone puro)
        assert_eq!(app.theme_mode, ThemeMode::Dark);
        let _ = app.update(Message::CycleTheme);
        assert_eq!(app.theme_mode, ThemeMode::System);
        let _ = app.view_toolbar();

        // Testa controles de simulação no toolbar
        assert!(!app.sim_running);
        let _ = app.update(Message::ToggleSimulation);
        assert!(app.sim_running);
        let _ = app.view_toolbar();

        // Stop zera a simulação (botão stop sem texto)
        let _ = app.update(Message::StopSimulation);
        assert!(!app.sim_running);
        let _ = app.view_toolbar();
    }

    #[test]
    fn test_menu_overlay_stays_open_over_canvas() {
        use iced::widget::canvas::Program;

        let (mut app, _) = CadeSimuApp::new();

        // 1. Abre o menu Arquivo
        let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Arquivo)));
        assert_eq!(app.open_menu, Some(ActiveMenu::Arquivo));

        // 2. Simula o cursor do mouse se movendo sobre a área de altura do canvas
        let _ = app.update(Message::Canvas(CanvasMessage::CursorMoved(120, 240)));
        assert_eq!(
            app.open_menu,
            Some(ActiveMenu::Arquivo),
            "Cursor se movendo sobre a altura do canvas NÃO pode fechar o menu flutuante!"
        );

        // 3. Renderiza a interface com o menu aberto e verifica que o canvas fica bloqueado
        let _ = app.view();

        let canvas_blocked = crate::canvas_view::SchematicCanvas {
            doc: &app.doc,
            tool: app.tool,
            selected_id: None,
            sim_running: false,
            show_grid: true,
            grid_density: app.grid_density,
            show_texts: true,
            is_dark: true,
            auto_connect: true,
            auto_disconnect: true,
            zoom: 1.0,
            pan: iced::Vector::ZERO,
            placement_orientation: 0,
            dragging_symbol: None,
            is_blocked: app.open_menu.is_some(),
        };

        let mut c_state = crate::canvas_view::CanvasInteraction::default();
        let (status, msg) = canvas_blocked.update(
            &mut c_state,
            iced::widget::canvas::Event::Mouse(iced::mouse::Event::CursorMoved {
                position: iced::Point::new(100.0, 300.0),
            }),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(800.0, 600.0)),
            iced::mouse::Cursor::Available(iced::Point::new(100.0, 300.0)),
        );

        assert_eq!(status, iced::widget::canvas::event::Status::Ignored);
        assert!(msg.is_none(), "Canvas bloqueado não gera mensagens");

        // 4. Clicar fora ou fechar menu encerra o overlay normalmente
        let _ = app.update(Message::ToggleMenu(None));
        assert_eq!(app.open_menu, None);
    }

    #[test]
    fn test_circuit_breakers_and_fuses_trip_on_short_circuit() {
        use crate::model::{Potential, SchematicDocument, SchematicItem};
        use crate::sim::{step_simulation, toggle_component_on_click};

        let mut doc = SchematicDocument::default();
        doc.items.clear();

        // 1. Testa Disjuntor Tripolar 6003 (-Q1) com curto-circuito entre fases L1 e L2
        let sym_src3 = find_symbol(3004).unwrap();
        let sym_q3p = find_symbol(6003).unwrap();

        let src = sym_src3.instantiate(1, 20, 20, 0);
        let wp_src = sym_src3.world_pins(&src);

        let mut q1 = sym_q3p.instantiate(2, 20, 40, 0);
        q1.tag = "-Q1".to_string();
        q1.sim_manual_latched = false; // Começa aberto
        let wp_q1 = sym_q3p.world_pins(&q1);

        doc.items.push(src);
        doc.items.push(q1);

        // Fios de alimentação: L1, L2, L3 para entradas do disjuntor (pinos 0, 1, 2)
        doc.items.push(SchematicItem::new_wire(
            3,
            4000,
            wp_src[0].2 .0,
            wp_src[0].2 .1,
            wp_q1[0].2 .0,
            wp_q1[0].2 .1,
        ));
        doc.items.push(SchematicItem::new_wire(
            4,
            4000,
            wp_src[1].2 .0,
            wp_src[1].2 .1,
            wp_q1[1].2 .0,
            wp_q1[1].2 .1,
        ));
        doc.items.push(SchematicItem::new_wire(
            5,
            4000,
            wp_src[2].2 .0,
            wp_src[2].2 .1,
            wp_q1[2].2 .0,
            wp_q1[2].2 .1,
        ));

        // Fio de curto-circuito franco entre saída L1 (pino 3) e saída L2 (pino 4)
        let sc_wire_id = 6;
        doc.items.push(SchematicItem::new_wire(
            sc_wire_id,
            4000,
            wp_q1[3].2 .0,
            wp_q1[3].2 .1,
            wp_q1[4].2 .0,
            wp_q1[4].2 .1,
        ));

        // Em repouso (disjuntor aberto), nada conduz a jusante e o disjuntor está sem falha
        step_simulation(&mut doc, 50);
        let q1_item = doc.items.iter().find(|i| i.id == 2).unwrap();
        assert!(!q1_item.sim_manual_latched);
        assert!(!q1_item.sim_fault);

        // Fecha o disjuntor -Q1 -> a corrente de curto entre L1 e L2 faz o disjuntor DESARMAR (trip magnético)!
        doc.items
            .iter_mut()
            .find(|i| i.id == 2)
            .unwrap()
            .sim_manual_latched = true;
        step_simulation(&mut doc, 50);

        let q1_after = doc.items.iter().find(|i| i.id == 2).unwrap();
        assert!(
            !q1_after.sim_manual_latched,
            "O disjuntor -Q1 deve desarmar (abrir) por curto-circuito!"
        );
        assert!(
            q1_after.sim_fault,
            "O disjuntor -Q1 deve registrar estado de trip/falha!"
        );
        assert!(
            !q1_after.sim_active,
            "O disjuntor desarmado não deve conduzir!"
        );

        // Como o disjuntor desarmou, as saídas são desenergizadas, extinguindo o curto
        let sc_wire_pot = doc
            .items
            .iter()
            .find(|i| i.id == sc_wire_id)
            .unwrap()
            .sim_potential;
        assert_ne!(
            sc_wire_pot,
            Potential::ShortCircuit,
            "A abertura do disjuntor deve extinguir o curto a jusante"
        );

        // 2. Testa rearme do disjuntor após remoção do curto
        doc.items.retain(|i| i.id != sc_wire_id);
        toggle_component_on_click(&mut doc, 2);
        let q1_rearmed = doc.items.iter().find(|i| i.id == 2).unwrap();
        assert!(q1_rearmed.sim_manual_latched);
        assert!(
            !q1_rearmed.sim_fault,
            "O rearme do disjuntor deve limpar o flag de trip"
        );

        step_simulation(&mut doc, 50);
        let q1_holding = doc.items.iter().find(|i| i.id == 2).unwrap();
        assert!(
            q1_holding.sim_manual_latched,
            "Sem curto-circuito, o disjuntor deve permanecer fechado"
        );
        assert!(!q1_holding.sim_fault);

        // 3. Testa Disjuntor Unipolar 6000 (-Q2) com curto fase-neutro
        let mut doc2 = SchematicDocument::default();
        doc2.items.clear();
        let sym_l1 = find_symbol(3000).unwrap();
        let sym_n = find_symbol(3001).unwrap();
        let sym_q1p = find_symbol(6000).unwrap();

        let l1 = sym_l1.instantiate(10, 10, 10, 0);
        let n = sym_n.instantiate(11, 30, 10, 0);
        let mut q2 = sym_q1p.instantiate(12, 10, 30, 0);
        q2.tag = "-Q2".to_string();
        q2.sim_manual_latched = true; // Fecha o disjuntor

        let wp_l1 = sym_l1.world_pins(&l1);
        let wp_n = sym_n.world_pins(&n);
        let wp_q2 = sym_q1p.world_pins(&q2);

        doc2.items.push(l1);
        doc2.items.push(n);
        doc2.items.push(q2);

        // Alimentação L1 -> entrada pino 0 de -Q2
        doc2.items.push(SchematicItem::new_wire(
            13,
            4000,
            wp_l1[0].2 .0,
            wp_l1[0].2 .1,
            wp_q2[0].2 .0,
            wp_q2[0].2 .1,
        ));
        // Curto direto: saída pino 1 de -Q2 -> Neutro N
        doc2.items.push(SchematicItem::new_wire(
            14,
            4009,
            wp_q2[1].2 .0,
            wp_q2[1].2 .1,
            wp_n[0].2 .0,
            wp_n[0].2 .1,
        ));

        step_simulation(&mut doc2, 50);
        let q2_tripped = doc2.items.iter().find(|i| i.id == 12).unwrap();
        assert!(
            !q2_tripped.sim_manual_latched,
            "Disjuntor 1P deve desarmar por curto fase-neutro!"
        );
        assert!(q2_tripped.sim_fault);

        // 4. Testa Fusível Seccionador 5008 (-F1) desarmando em curto-circuito
        let mut doc3 = SchematicDocument::default();
        doc3.items.clear();
        let sym_f3p = find_symbol(5008).unwrap();
        let src3 = sym_src3.instantiate(20, 10, 10, 0);
        let wp_src3 = sym_src3.world_pins(&src3);
        let mut f1 = sym_f3p.instantiate(21, 10, 30, 0);
        f1.tag = "-F1".to_string();
        f1.sim_manual_latched = true; // Fecha a seccionadora fusível
        let wp_f1 = sym_f3p.world_pins(&f1);

        doc3.items.push(src3);
        doc3.items.push(f1);

        // Alimenta L1 e L2 no fusível
        doc3.items.push(SchematicItem::new_wire(
            22,
            4000,
            wp_src3[0].2 .0,
            wp_src3[0].2 .1,
            wp_f1[0].2 .0,
            wp_f1[0].2 .1,
        ));
        doc3.items.push(SchematicItem::new_wire(
            23,
            4000,
            wp_src3[1].2 .0,
            wp_src3[1].2 .1,
            wp_f1[1].2 .0,
            wp_f1[1].2 .1,
        ));
        // Curto entre saídas L1 (pino 3) e L2 (pino 4) do fusível
        doc3.items.push(SchematicItem::new_wire(
            24,
            4000,
            wp_f1[3].2 .0,
            wp_f1[3].2 .1,
            wp_f1[4].2 .0,
            wp_f1[4].2 .1,
        ));

        step_simulation(&mut doc3, 50);
        let f1_blown = doc3.items.iter().find(|i| i.id == 21).unwrap();
        assert!(
            !f1_blown.sim_manual_latched,
            "Fusível seccionador deve desarmar (abrir) por curto-circuito!"
        );
        assert!(
            f1_blown.sim_fault,
            "Fusível queimado deve registrar estado de falha"
        );
        assert!(!f1_blown.sim_active);

        // 5. Testa Fusível Permanente 5003 (-F2) queimando em curto-circuito
        let mut doc4 = SchematicDocument::default();
        doc4.items.clear();
        let sym_f3_fixed = find_symbol(5003).unwrap();
        let src4 = sym_src3.instantiate(30, 10, 10, 0);
        let wp_src4 = sym_src3.world_pins(&src4);
        let mut f2 = sym_f3_fixed.instantiate(31, 10, 30, 0);
        f2.tag = "-F2".to_string();
        let wp_f2 = sym_f3_fixed.world_pins(&f2);

        doc4.items.push(src4);
        doc4.items.push(f2);
        doc4.items.push(SchematicItem::new_wire(
            32,
            4000,
            wp_src4[0].2 .0,
            wp_src4[0].2 .1,
            wp_f2[0].2 .0,
            wp_f2[0].2 .1,
        ));
        doc4.items.push(SchematicItem::new_wire(
            33,
            4000,
            wp_src4[1].2 .0,
            wp_src4[1].2 .1,
            wp_f2[1].2 .0,
            wp_f2[1].2 .1,
        ));
        doc4.items.push(SchematicItem::new_wire(
            34,
            4000,
            wp_f2[3].2 .0,
            wp_f2[3].2 .1,
            wp_f2[4].2 .0,
            wp_f2[4].2 .1,
        ));

        step_simulation(&mut doc4, 50);
        let f2_blown = doc4.items.iter().find(|i| i.id == 31).unwrap();
        assert!(
            f2_blown.sim_manual_latched,
            "Fusível fixo deve queimar (sim_manual_latched: true)!"
        );
        assert!(f2_blown.sim_fault);
        assert!(!f2_blown.sim_active);
    }
}
