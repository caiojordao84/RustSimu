use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, mouse_area, pick_list,
    row, scrollable, stack, svg, text, text_input, tooltip, vertical_space, Space, Stack,
};
use iced::{Alignment, Element, Font, Length, Padding, Subscription, Theme, Vector};

use crate::canvas_view::{CanvasMessage, EditorTool, SchematicCanvas};
use crate::io::{build_dol_starter_preset, load_document, save_document};
use crate::layout::{Kind, Layout, Msg as LayoutMsg};
use crate::model::{SchematicDocument, SchematicItem};
use crate::sim::{press_component_on_sim, release_component_on_sim, step_simulation};
use crate::symbols::{
    classify_apparatus_role, detect_contact_kind, detect_current_sequence, extract_tag_prefix,
    find_duplicate_apparatus, find_occupied_sequences_for_tag, find_parent_apparatus, find_symbol,
    list_available_parent_tags, next_sequential_tag_for_role, SymbolCategory,
};
use crate::theme::{self, tokens, ThemeMode, MENU_H, MONO, STATUS_H, TOOL_H, UI};

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

/// Formato de arquivo para salvamento nativo do projeto (Decisão D1: JSON padrão, TOML ou QET).
/// O formato legado `.cad` é monofolha e foi reclassificado exclusivamente para Exportação e Importação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFormat {
    Json,
    Toml,
    Qet,
}

impl SaveFormat {
    pub const ALL: [SaveFormat; 3] = [SaveFormat::Json, SaveFormat::Toml, SaveFormat::Qet];

    pub fn extension(self) -> &'static str {
        match self {
            SaveFormat::Json => "json",
            SaveFormat::Toml => "toml",
            SaveFormat::Qet => "qet",
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            SaveFormat::Json => "Projeto JSON (*.json)",
            SaveFormat::Toml => "Projeto TOML (*.toml)",
            SaveFormat::Qet => "Projeto QElectroTech (*.qet)",
        }
    }

    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
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
            SaveFormat::Json => write!(f, "Formato .JSON (Padrão)"),
            SaveFormat::Toml => write!(f, "Formato .TOML"),
            SaveFormat::Qet => write!(f, "Formato .QET (QElectroTech)"),
        }
    }
}

pub use crate::export::{ExportFormat, ExportScope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPageOption {
    pub index: usize,
    pub label: String,
}

impl std::fmt::Display for ExportPageOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
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
    SaveDocument,
    SaveFileDialog,
    FileSaved(Option<PathBuf>),
    OpenExportModal,
    CloseExportModal,
    SelectExportFormat(ExportFormat),
    SelectExportScope(ExportScope),
    SelectExportPage(usize),
    ToggleExportCajetin(bool),
    ConfirmExport,
    FileExported(
        Option<PathBuf>,
        Box<SchematicDocument>,
        String,
        ExportFormat,
        ExportScope,
        bool,
        Option<i32>,
    ),
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
    CanonicalSearchChanged(String),
    SelectCanonicalScope(crate::canonical::CanonicalScope),
    SelectCanonicalFamily(Option<String>),
    SelectCanonicalKind(Option<String>),
    SelectCanonicalPage(usize),
    NextCanonicalPage,
    PrevCanonicalPage,
    SelectCanonicalSymbol(String),
    StartCanonicalSymbolDrag(String),
    InsertSelectedCanonicalSymbol,
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
    // Biblioteca Canônica D2
    pub canonical_search: String,
    pub canonical_scope: crate::canonical::CanonicalScope,
    pub canonical_family_filter: Option<String>,
    pub canonical_kind_filter: Option<String>,
    pub canonical_page: usize,
    pub selected_canonical_id: Option<String>,
    pub dragging_canonical_id: Option<String>,
    pub last_canonical_click: Option<(String, std::time::Instant)>,
    pub prop_tab: usize,
    pub left_dock_tab: usize,
    pub left_dock_collapsed: bool,
    pub right_dock_collapsed: bool,
    pub open_menu: Option<ActiveMenu>,
    pub show_about_modal: bool,
    pub show_export_modal: bool,
    pub export_format: ExportFormat,
    pub export_scope: ExportScope,
    pub export_page_index: usize,
    pub export_include_cajetin: bool,
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
    pub fn new() -> (Self, iced::Task<Message>) {
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
                status_msg: "Pronto - RustSimu ativo".to_string(),
                theme_mode: ThemeMode::Dark,
                system_is_dark: crate::theme::detect_system_dark(),
                layout,
                collection_search: String::new(),
                selected_collection_sym: Some(6003),
                dragging_symbol: None,
                last_collection_click: None,
                last_placed_sym: None,
                canonical_search: String::new(),
                canonical_scope: crate::canonical::CanonicalScope::All,
                canonical_family_filter: None,
                canonical_kind_filter: None,
                canonical_page: 0,
                selected_canonical_id: Some("00001".to_string()),
                dragging_canonical_id: None,
                last_canonical_click: None,
                prop_tab: 0,
                left_dock_tab: 0,
                left_dock_collapsed: false,
                right_dock_collapsed: false,
                open_menu: None,
                show_about_modal: false,
                show_export_modal: false,
                export_format: ExportFormat::Cad,
                export_scope: ExportScope::Page,
                export_page_index: 0,
                export_include_cajetin: true,
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

    pub fn theme(&self) -> Theme {
        self.theme_mode.resolve(self.system_is_dark).theme()
    }

    pub fn subscription(&self) -> Subscription<Message> {
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
                        "s" if ctrl => Some(Message::SaveDocument),
                        "w" if ctrl => Some(Message::CloseDocument),
                        "p" if ctrl => Some(Message::PrintDocument),
                        "e" if ctrl && shift => Some(Message::OpenExportModal),
                        "e" if ctrl => Some(Message::OpenExportModal),
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

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
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
                if self.sim_running && !matches!(tool, EditorTool::Select) {
                    self.sim_running = false;
                    self.doc.reset_simulation_state();
                    self.status_msg = "Simulação encerrada para edição.".to_string();
                }
                self.tool = tool;
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
                                pages.push(crate::model::ProjectPage::with_origin(
                                    idx + 1,
                                    title,
                                    page_doc,
                                    Some(idx),
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
                    match crate::io::load_project(&path) {
                        Ok(project) => {
                            self.sim_running = false;
                            let total_pages = project.pages.len();
                            self.pages = project.pages;
                            self.active_page_index = project
                                .active_page_index
                                .min(self.pages.len().saturating_sub(1));
                            self.doc = self
                                .pages
                                .get(self.active_page_index)
                                .map(|p| p.doc.clone())
                                .unwrap_or_default();
                            self.selected_id = None;
                            self.is_modified = false;
                            self.editing_page_name = None;
                            self.page_context_menu = None;
                            let count = self.doc.items.len();
                            let page_title = self
                                .pages
                                .get(self.active_page_index)
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| "Principal".to_string());
                            self.status_msg = if total_pages > 1 {
                                format!(
                                    "Projeto '{}' aberto ({total_pages} folhas). Folha {}/{}: {} ({count} elementos).",
                                    path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path.display().to_string()),
                                    self.active_page_index + 1,
                                    total_pages,
                                    page_title
                                )
                            } else {
                                format!(
                                    "Projeto '{}' aberto com sucesso ({count} elementos).",
                                    path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path.display().to_string())
                                )
                            };
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
            Message::SaveDocument => {
                if let Some(ref path) = self.current_file {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_default();
                    if ext == "json" || ext == "toml" || ext == "qet" {
                        if self.active_page_index < self.pages.len() {
                            self.pages[self.active_page_index].doc = self.doc.clone();
                        }
                        let path_clone = path.clone();
                        let res = if ext == "qet" {
                            crate::qet::export_pages_to_qet(
                                &self.pages,
                                self.active_page_index,
                                &path_clone,
                                self.qet_project.as_ref(),
                            )
                        } else {
                            crate::io::save_project(
                                &path_clone,
                                &self.pages,
                                self.active_page_index,
                            )
                        };

                        match res {
                            Ok(()) => {
                                self.is_modified = false;
                                self.status_msg = format!(
                                    "Projeto salvo com sucesso em '{}'.",
                                    path_clone.display()
                                );
                            }
                            Err(err) => {
                                self.status_msg =
                                    format!("Erro ao salvar '{}': {err}", path_clone.display());
                            }
                        }
                        return iced::Task::none();
                    }
                }
                // Se for a primeira vez salvando ou se for um arquivo .cad importado, solicita nome e formato (JSON padrão)
                return self.update(Message::SaveFileDialog);
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
                    "Escolha onde salvar o projeto (.json, .toml ou .qet) na janela do sistema..."
                        .to_string();

                return iced::Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new()
                            .set_title("Salvar Projeto (.json, .toml, .qet)")
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
            Message::OpenExportModal => {
                if self.active_page_index < self.pages.len() {
                    self.pages[self.active_page_index].doc = self.doc.clone();
                }
                self.show_export_modal = true;
                self.export_page_index = self.active_page_index;
                self.status_msg =
                    "Exportação: Escolha o formato desejado (.cad, .pdf, .svg ou .png) e a folha a exportar."
                        .to_string();
            }
            Message::CloseExportModal => {
                self.show_export_modal = false;
            }
            Message::SelectExportFormat(fmt) => {
                self.export_format = fmt;
                let valid_scopes = ExportScope::valid_scopes(fmt, self.pages.len() > 1);
                if !valid_scopes.contains(&self.export_scope) {
                    self.export_scope = valid_scopes[0];
                }
            }
            Message::SelectExportScope(scope) => {
                self.export_scope = scope;
            }
            Message::ToggleExportCajetin(enabled) => {
                self.export_include_cajetin = enabled;
            }
            Message::SelectExportPage(idx) => {
                self.export_page_index = idx;
            }
            Message::ConfirmExport => {
                self.show_export_modal = false;
                let fmt = self.export_format;
                let scope = self.export_scope;
                let include_cajetin = self.export_include_cajetin;
                let selected_id = self.selected_id;

                if scope == ExportScope::Selection && selected_id.is_none() {
                    self.status_msg = "Aviso: Nenhum elemento selecionado na folha para exportar a seleção. Selecione um elemento antes de exportar.".to_string();
                    return iced::Task::none();
                }

                let page_idx = self
                    .export_page_index
                    .min(self.pages.len().saturating_sub(1));
                let doc_to_export = if page_idx == self.active_page_index {
                    self.doc.clone()
                } else if page_idx < self.pages.len() {
                    self.pages[page_idx].doc.clone()
                } else {
                    self.doc.clone()
                };

                let page_title = if page_idx < self.pages.len() {
                    self.pages[page_idx].name.clone()
                } else {
                    "Folha 1".to_string()
                };

                let start_dir = self.last_dir.clone();
                let stem = self
                    .current_file
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("projeto");

                let clean_title: String = page_title
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();

                let default_name = match scope {
                    ExportScope::Project => format!("{stem}_completo.{}", fmt.extension()),
                    ExportScope::Selection => format!(
                        "{stem}_{}_selecao.{}",
                        clean_title.trim_matches('_'),
                        fmt.extension()
                    ),
                    ExportScope::Page => {
                        if self.pages.len() > 1 {
                            format!(
                                "{stem}_{}.{}",
                                clean_title.trim_matches('_'),
                                fmt.extension()
                            )
                        } else {
                            format!("{stem}.{}", fmt.extension())
                        }
                    }
                };

                return iced::Task::perform(
                    async move {
                        let dialog = rfd::AsyncFileDialog::new()
                            .set_title("Exportar Esquema Elétrico")
                            .set_directory(&start_dir)
                            .set_file_name(&default_name)
                            .add_filter(fmt.filter_label(), &[fmt.extension()]);

                        dialog.save_file().await.map(|h| h.path().to_path_buf())
                    },
                    move |path| {
                        Message::FileExported(
                            path,
                            Box::new(doc_to_export.clone()),
                            page_title.clone(),
                            fmt,
                            scope,
                            include_cajetin,
                            selected_id,
                        )
                    },
                );
            }
            Message::FileExported(
                maybe_path,
                doc,
                page_title,
                fmt,
                scope,
                include_cajetin,
                selected_id,
            ) => {
                let Some(path) = maybe_path else {
                    self.status_msg = "Exportação cancelada.".to_string();
                    return iced::Task::none();
                };
                if let Some(parent) = path.parent() {
                    self.last_dir = parent.to_path_buf();
                }

                let filter_slice = if scope == ExportScope::Selection {
                    selected_id.map(|id| vec![id])
                } else {
                    None
                };
                let items_filter = filter_slice.as_deref();

                let res: Result<(), String> = match fmt {
                    ExportFormat::Cad => {
                        let bytes = crate::export::export_to_cad(&doc, include_cajetin);
                        crate::io::atomic_write(&path, bytes)
                    }
                    ExportFormat::Svg => {
                        let svg = crate::export::export_to_svg(&doc, items_filter, include_cajetin);
                        crate::io::atomic_write(&path, svg.as_bytes())
                    }
                    ExportFormat::Png => {
                        crate::export::export_to_png(&doc, items_filter, include_cajetin, 2.5)
                            .and_then(|bytes| crate::io::atomic_write(&path, bytes))
                    }
                    ExportFormat::Pdf => {
                        let pdf_res = if scope == ExportScope::Project {
                            crate::export::export_project_to_pdf(&self.pages, include_cajetin)
                        } else {
                            crate::export::export_doc_to_pdf(&doc, items_filter, include_cajetin)
                        };
                        pdf_res.and_then(|bytes| crate::io::atomic_write(&path, bytes))
                    }
                };

                match res {
                    Ok(()) => {
                        self.status_msg = format!(
                            "Exportação de '{page_title}' concluída com sucesso ({fmt} - {}): '{}'",
                            scope.label(),
                            path.display()
                        );
                    }
                    Err(err) => {
                        self.status_msg = format!("Erro ao exportar arquivo: {err}");
                    }
                }
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

                if self.active_page_index < self.pages.len() {
                    self.pages[self.active_page_index].doc = self.doc.clone();
                }

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();

                let res = if ext == "qet" {
                    crate::qet::export_pages_to_qet(
                        &self.pages,
                        self.active_page_index,
                        &path,
                        self.qet_project.as_ref(),
                    )
                } else {
                    crate::io::save_project(&path, &self.pages, self.active_page_index)
                };

                match res {
                    Ok(()) => {
                        self.is_modified = false;
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
                // Reclassificado conforme Decisão D1: .cad é formato de exportação de folha única
                return self.update(Message::OpenExportModal);
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
                CanvasMessage::PlaceCanonicalComponent { symbol_id, x, y } => {
                    self.dragging_canonical_id = None;
                    self.tool = EditorTool::Select;
                    if let Some(dict) = crate::canonical::global_dictionary() {
                        if let Some(el) = dict.get_by_id(&symbol_id) {
                            self.push_undo();
                            let new_id = self.doc.next_id();
                            let native_tipo = if el.family == "cade_native" {
                                el.slug
                                    .split('_')
                                    .next()
                                    .and_then(|p| p.parse::<i32>().ok())
                                    .unwrap_or(0)
                            } else {
                                0
                            };

                            let prefix = el.kind.default_tag_prefix();

                            let count = self
                                .doc
                                .items
                                .iter()
                                .filter(|i| i.tag.starts_with(prefix))
                                .count()
                                + 1;
                            let tag = format!("{}{}", prefix, count);

                            let mut pins: [String; 9] = Default::default();
                            for (idx, term) in el.terminals.iter().take(9).enumerate() {
                                pins[idx] = term.name.clone();
                            }

                            let item = SchematicItem {
                                id: new_id,
                                tipo: native_tipo,
                                tag,
                                value: String::new(),
                                pins,
                                pin_visibility: [1; 9],
                                x1: x,
                                y1: y,
                                x2: 0,
                                y2: 0,
                                aux_coords: [0, 0, 0, 0],
                                color: 0,
                                orientation: self.placement_orientation,
                                subvariant: 0,
                                show_tag: 1,
                                show_value: 0,
                                extra_int1: 0,
                                extra_int2: 0,
                                trailing_str: String::new(),
                                qet_type: Some(el.slug.clone()),
                                qet_uuid: None,
                                symbol_id: Some(el.id.clone()),
                                symbol_state: Some('0'),
                            };

                            self.doc.items.push(item);
                            self.selected_id = Some(new_id);
                            self.is_modified = true;
                            self.status_msg = format!(
                                "Símbolo '{}' ({}) inserido em ({x}, {y}).",
                                el.name_pt, el.id
                            );
                        }
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
                                if self.doc.sim_fault(item.id) {
                                    "DESARMADO POR CURTO-CIRCUITO (TRIP)"
                                } else if self.doc.sim_latched(item.id) {
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
                    self.dragging_canonical_id = None;
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
                if self.show_export_modal {
                    self.show_export_modal = false;
                    return iced::Task::none();
                }
                if self.show_about_modal {
                    self.show_about_modal = false;
                    return iced::Task::none();
                }
                if let Some(snapshot) = self.drag_undo_snapshot.take() {
                    self.doc = snapshot;
                    self.status_msg = "Movimento cancelado.".to_string();
                } else {
                    self.status_msg = "Ação cancelada. Modo de seleção ativado.".to_string();
                }
                self.open_menu = None;
                self.tool = EditorTool::Select;
                self.dragging_symbol = None;
                self.dragging_canonical_id = None;
            }
            Message::GlobalMouseReleased => {
                self.dragging_symbol = None;
                self.dragging_canonical_id = None;
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
                            "Símbolo '{}' pronto no ponteiro - clique na folha para posicionar.",
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
                            "Símbolo '{}' pronto no ponteiro - clique na folha para posicionar.",
                            sym.name
                        );
                    }
                }
            }
            Message::CanonicalSearchChanged(query) => {
                self.canonical_search = query;
                self.canonical_page = 0;
            }
            Message::SelectCanonicalScope(scope) => {
                self.canonical_scope = scope;
                self.canonical_page = 0;
            }
            Message::SelectCanonicalFamily(fam) => {
                self.canonical_family_filter = fam;
                self.canonical_kind_filter = None;
                self.canonical_page = 0;
            }
            Message::SelectCanonicalKind(kind) => {
                self.canonical_kind_filter = kind;
                self.canonical_page = 0;
            }
            Message::SelectCanonicalPage(page) => {
                self.canonical_page = page;
            }
            Message::NextCanonicalPage => {
                if let Some(dict) = crate::canonical::global_dictionary() {
                    let filtered = dict.filter_elements(
                        &self.canonical_search,
                        self.canonical_family_filter.as_deref(),
                        self.canonical_kind_filter.as_deref(),
                        self.canonical_scope,
                    );
                    let total_pages = if filtered.is_empty() {
                        1
                    } else {
                        filtered.len().div_ceil(35)
                    };
                    if self.canonical_page + 1 < total_pages {
                        self.canonical_page += 1;
                    }
                }
            }
            Message::PrevCanonicalPage => {
                if self.canonical_page > 0 {
                    self.canonical_page -= 1;
                }
            }
            Message::StartCanonicalSymbolDrag(id) => {
                self.dragging_canonical_id = Some(id.clone());
                self.selected_canonical_id = Some(id);
            }
            Message::SelectCanonicalSymbol(id) => {
                self.dragging_canonical_id = None;
                let now = std::time::Instant::now();
                let is_double_click =
                    if let Some((ref last_id, last_time)) = self.last_canonical_click {
                        last_id == &id && now.duration_since(last_time) < Duration::from_millis(400)
                    } else {
                        false
                    };
                self.last_canonical_click = Some((id.clone(), now));
                self.selected_canonical_id = Some(id.clone());

                if is_double_click {
                    self.tool = EditorTool::PlaceCanonicalSymbol(id.clone());
                    if let Some(dict) = crate::canonical::global_dictionary() {
                        if let Some(el) = dict.get_by_id(&id) {
                            self.status_msg = format!(
                                "Símbolo '{}' ({}) pronto no ponteiro - clique na folha para posicionar.",
                                el.name_pt, el.id
                            );
                        }
                    }
                } else if let Some(dict) = crate::canonical::global_dictionary() {
                    if let Some(el) = dict.get_by_id(&id) {
                        self.status_msg = format!(
                            "Símbolo '{}' ({}) selecionado. Dê duplo clique, arraste ou clique em 'Inserir no Esquema'.",
                            el.name_pt, el.id
                        );
                    }
                }
            }
            Message::InsertSelectedCanonicalSymbol => {
                if let Some(ref id) = self.selected_canonical_id {
                    self.tool = EditorTool::PlaceCanonicalSymbol(id.clone());
                    if let Some(dict) = crate::canonical::global_dictionary() {
                        if let Some(el) = dict.get_by_id(id) {
                            self.status_msg = format!(
                                "Símbolo '{}' ({}) pronto no ponteiro - clique na folha para posicionar.",
                                el.name_pt, el.id
                            );
                        }
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
                return self.update(Message::OpenExportModal);
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

    pub fn view(&self) -> Element<'_, Message> {
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
                .width(540)
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

        // Modal de Exportação do Esquema (Decisão D1)
        if self.show_export_modal {
            let page_options: Vec<ExportPageOption> = self
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
                .find(|p| p.index == self.export_page_index)
                .cloned()
                .or_else(|| page_options.first().cloned());

            let is_cad = self.export_format == ExportFormat::Cad;
            let is_multipage = self.pages.len() > 1;
            let valid_scopes = ExportScope::valid_scopes(self.export_format, is_multipage);

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
                        Some(self.export_format),
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
                        Some(self.export_scope),
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
            if self.export_scope == ExportScope::Page {
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
                        self.pages
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
            if self.export_scope == ExportScope::Selection {
                let sel_info = if let Some(id) = self.selected_id {
                    let item_label = self
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
            if self.export_scope == ExportScope::Project {
                let project_notice = container(
                    row![
                        crate::icons::view_icon(crate::icons::Icon::ExportPdf, 16, t.primary),
                        column![
                            text(format!(
                                "Exportando Projeto Completo ({} folhas):",
                                self.pages.len()
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
            if self.export_scope != ExportScope::Selection {
                let cajetin_box = column![
                    checkbox(
                        "Incluir Moldura e Cajetín (Margens, Réguas e Carimbo)",
                        self.export_include_cajetin,
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

            let modal_overlay = stack![
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

        let mode_dot = container(Space::new(6, 6)).style(theme::status_dot(if self.sim_running {
            t.amber
        } else {
            t.primary
        }));
        let mode_label = text(if self.sim_running {
            "Simulação"
        } else {
            "Edição"
        })
        .size(10.5)
        .font(MONO);

        let mode_pill = container(
            row![mode_dot, mode_label]
                .spacing(4)
                .align_y(Alignment::Center),
        )
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
                    Message::SaveDocument
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
                    Message::OpenExportModal
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

    pub fn view_toolbar(&self) -> Element<'_, Message> {
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
                    Message::SaveDocument
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
                    "Exportar Esquema (Ctrl+Shift+E)",
                    Message::OpenExportModal
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
            0 => self.view_unified_library_panel(),
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
                                row![
                                    crate::icons::view_icon(
                                        crate::icons::Icon::Check,
                                        11,
                                        t.primary
                                    ),
                                    text("Ativa").size(10.5).color(t.primary)
                                ]
                                .spacing(3)
                                .align_y(Alignment::Center)
                            } else {
                                row![text("").size(10.5)]
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

    fn get_subdivisions_for_family(
        family: Option<&str>,
    ) -> &'static [(&'static str, Option<&'static str>)] {
        match family {
            None => &[
                ("Todas", None),
                ("Motores", Some("motor")),
                ("Disjuntores", Some("circuit_breaker")),
                ("Fusíveis", Some("fuse")),
                ("Bobinas", Some("relay_coil")),
                ("Contatos", Some("contactor_contact")),
                ("Botoeiras", Some("momentary_button")),
                ("Chaves", Some("latching_switch")),
                ("Sinaleiros", Some("lamp")),
                ("Fontes", Some("source")),
                ("Bornes", Some("terminal")),
                ("Cargas", Some("load")),
                ("Blocos / Montagem", Some("block")),
                ("Válvulas", Some("pneu_valves")),
                ("Atuadores", Some("pneu_actuators")),
                ("Lógica", Some("logic_gates")),
            ],
            Some("electric") => &[
                ("Todas", None),
                ("Motores", Some("motor")),
                ("Disjuntores", Some("circuit_breaker")),
                ("Fusíveis", Some("fuse")),
                ("Bobinas", Some("relay_coil")),
                ("Contatos", Some("contactor_contact")),
                ("Botoeiras", Some("momentary_button")),
                ("Chaves", Some("latching_switch")),
                ("Sinaleiros", Some("lamp")),
                ("Fontes", Some("source")),
                ("Bornes", Some("terminal")),
                ("Cargas", Some("load")),
                ("Blocos / Montagem", Some("block")),
            ],
            Some("pneumatic") => &[
                ("Todas", None),
                ("Válvulas", Some("pneu_valves")),
                ("Ar Comprimido", Some("pneu_air")),
                ("Atuadores / Cilindros", Some("pneu_actuators")),
                ("Sensores", Some("pneu_sensors")),
            ],
            Some("hydraulic") => &[
                ("Todas", None),
                ("Válvulas de Controle", Some("hyd_valves")),
                ("Bombas", Some("hyd_pumps")),
                ("Cilindros", Some("hyd_cylinders")),
                ("Tanques", Some("hyd_tanks")),
                ("Filtros", Some("hyd_filters")),
            ],
            Some("logic") => &[
                ("Todas", None),
                ("Portas Lógicas", Some("logic_gates")),
                ("Ladder", Some("logic_ladder")),
                ("Fluxograma e Grafcet", Some("logic_flowchart")),
            ],
            Some("energy") => &[
                ("Todas", None),
                ("Água e Calefação", Some("energy_water")),
                ("Refrigeração", Some("energy_refrig")),
                ("Solar Térmica", Some("energy_solar")),
                ("Fabricantes", Some("energy_mfg")),
            ],
            Some("cade_native") => &[
                ("Todas", None),
                ("Alimentações", Some("cade_power")),
                ("Disjuntores / Fusíveis", Some("cade_protection")),
                ("Contatores / Relés", Some("cade_control")),
                ("Motores", Some("cade_motors")),
                ("Botoeiras / Chaves", Some("cade_switches")),
                ("Sinalização", Some("cade_signaling")),
            ],
            Some("import") => &[
                ("Todas", None),
                ("Automotivo / Veicular", Some("classic_car")),
                ("Elétrica Importada", Some("10_electric")),
                ("Outros", Some("import")),
            ],
            Some(_) => &[("Todas", None)],
        }
    }

    fn view_unified_library_panel(&self) -> Element<'_, Message> {
        let t = tokens(&self.theme());
        let is_dark = self.is_dark();

        // 1. Atalhos de cabeamento rápido sempre visíveis
        let wire_shortcuts = row![
            button(text("+ Fase L").size(10.5))
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectTool(EditorTool::DrawWire(4000))),
            button(text("+ Neutro N").size(10.5))
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectTool(EditorTool::DrawWire(4009))),
            button(text("+ Terra").size(10.5))
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectTool(EditorTool::DrawWire(4010))),
            button(text("+ Nó").size(10.5))
                .style(theme::tool)
                .padding(Padding::from([2.0, 6.0]))
                .on_press(Message::SelectTool(EditorTool::PlaceJunction)),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        let shortcuts_box = container(wire_shortcuts)
            .padding(Padding::from([4.0, 10.0]))
            .width(Length::Fill);

        // 2. Barra de busca com botão de limpar
        let search_input = text_input(
            "Buscar na biblioteca unificada (9.3k)...",
            &self.canonical_search,
        )
        .on_input(Message::CanonicalSearchChanged)
        .padding(Padding::from([5.0, 8.0]))
        .size(11.5)
        .style(theme::field);

        let search_box = if self.canonical_search.is_empty() {
            container(search_input).padding(Padding::from([4.0, 10.0]))
        } else {
            container(
                row![
                    search_input,
                    button(text("x").size(10.0).font(UI))
                        .style(theme::tool)
                        .padding(Padding::from([4.0, 6.0]))
                        .on_press(Message::CanonicalSearchChanged(String::new())),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .padding(Padding::from([4.0, 10.0]))
        };

        // 3. Chips de Escopo: Todos / Esquema (Cabáveis) / Painel (Blocos)
        let mut scope_chips = row![].spacing(4).align_y(Alignment::Center);
        for (label, sc) in [
            ("Todos", crate::canonical::CanonicalScope::All),
            (
                "Esquema (Cabáveis)",
                crate::canonical::CanonicalScope::Wireable,
            ),
            ("Painel (Blocos)", crate::canonical::CanonicalScope::Block),
        ] {
            let is_active = self.canonical_scope == sc;
            scope_chips = scope_chips.push(
                button(text(label).size(10.0).font(UI))
                    .style(if is_active {
                        theme::tool_active
                    } else {
                        theme::tool
                    })
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::SelectCanonicalScope(sc)),
            );
        }
        let scope_scroll = container(
            scrollable(scope_chips)
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new()
                        .width(2.0)
                        .scroller_width(2.0)
                        .margin(0.0),
                ))
                .width(Length::Fill),
        )
        .padding(Padding::from([2.0, 10.0]));

        // 4. Chips de filtro por Família
        let mut fam_chips = row![].spacing(4).align_y(Alignment::Center);
        for (label, fam_opt) in [
            ("Todas", None),
            ("Elétrica", Some("electric")),
            ("Energia", Some("energy")),
            ("Pneumática", Some("pneumatic")),
            ("Hidráulica", Some("hydraulic")),
            ("Lógica", Some("logic")),
            ("Importados", Some("import")),
            ("CADe", Some("cade_native")),
        ] {
            let is_active = self.canonical_family_filter.as_deref() == fam_opt;
            let fam_val = fam_opt.map(|s| s.to_string());
            fam_chips = fam_chips.push(
                button(text(label).size(10.0).font(UI))
                    .style(if is_active {
                        theme::tool_active
                    } else {
                        theme::tool
                    })
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::SelectCanonicalFamily(fam_val)),
            );
        }
        let fam_scroll = container(
            scrollable(fam_chips)
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new()
                        .width(2.0)
                        .scroller_width(2.0)
                        .margin(0.0),
                ))
                .width(Length::Fill),
        )
        .padding(Padding::from([0.0, 10.0]));

        // 4. Chips de filtro por Subdivisão temática dinâmica (obedecendo a família selecionada)
        let mut sub_chips = row![].spacing(4).align_y(Alignment::Center);
        let subdivisions =
            Self::get_subdivisions_for_family(self.canonical_family_filter.as_deref());
        for &(label, sub_opt) in subdivisions {
            let is_active = self.canonical_kind_filter.as_deref() == sub_opt;
            let sub_val = sub_opt.map(|s| s.to_string());
            sub_chips = sub_chips.push(
                button(text(label).size(10.0).font(UI))
                    .style(if is_active {
                        theme::tool_active
                    } else {
                        theme::tool
                    })
                    .padding(Padding::from([2.0, 6.0]))
                    .on_press(Message::SelectCanonicalKind(sub_val)),
            );
        }
        let sub_scroll = container(
            scrollable(sub_chips)
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new()
                        .width(2.0)
                        .scroller_width(2.0)
                        .margin(0.0),
                ))
                .width(Length::Fill),
        )
        .padding(Padding::from([2.0, 10.0]));

        // 5. Filtragem e Paginação dos Elementos
        let (page_elements, current_page, total_pages, total_count) =
            if let Some(dict) = crate::canonical::global_dictionary() {
                let filtered = dict.filter_elements(
                    &self.canonical_search,
                    self.canonical_family_filter.as_deref(),
                    self.canonical_kind_filter.as_deref(),
                    self.canonical_scope,
                );
                let total = filtered.len();
                let total_pages = if total == 0 { 1 } else { total.div_ceil(35) };
                let cur_page = self.canonical_page.min(total_pages.saturating_sub(1));
                let start = cur_page * 35;
                let end = total.min(start + 35);
                let page_slice: Vec<_> = filtered[start..end].to_vec();
                (page_slice, cur_page, total_pages, total)
            } else {
                (Vec::new(), 0, 1, 0)
            };

        // 6. Barra de controle de paginação
        let prev_btn = button(text("◀").size(10.0).font(UI))
            .style(theme::tool)
            .padding(Padding::from([2.0, 6.0]));
        let prev_btn = if current_page > 0 {
            prev_btn.on_press(Message::PrevCanonicalPage)
        } else {
            prev_btn
        };

        let next_btn = button(text("▶").size(10.0).font(UI))
            .style(theme::tool)
            .padding(Padding::from([2.0, 6.0]));
        let next_btn = if current_page + 1 < total_pages {
            next_btn.on_press(Message::NextCanonicalPage)
        } else {
            next_btn
        };

        let pagination_bar = container(
            row![
                prev_btn,
                horizontal_space(),
                text(format!(
                    "Pág {}/{} ({} símbolos)",
                    current_page + 1,
                    total_pages,
                    total_count
                ))
                .size(10.5)
                .font(UI)
                .color(t.text2),
                horizontal_space(),
                next_btn,
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .padding(Padding::from([4.0, 10.0]));

        // 7. Lista paginada de símbolos
        let mut list_col = column![].spacing(3).padding(Padding::from([0.0, 10.0]));
        if page_elements.is_empty() {
            list_col = list_col.push(
                container(
                    text("Nenhum símbolo encontrado para os filtros atuais.")
                        .size(11.0)
                        .color(t.text2),
                )
                .padding(Padding::from([12.0, 4.0])),
            );
        } else {
            for el in page_elements {
                let is_selected = self.selected_canonical_id.as_deref() == Some(&el.id);
                let pin_count = el.terminals.len();
                let is_wireable = el.is_wireable();

                let badge_or_pins: Element<'_, Message> = if !is_wireable {
                    container(text("Bloco").size(9.0).font(UI))
                        .style(theme::tag_badge)
                        .padding(Padding::from([1.0, 4.0]))
                        .into()
                } else {
                    text(format!("{pin_count}b"))
                        .size(10.0)
                        .font(MONO)
                        .color(t.text2)
                        .into()
                };

                let item_row = container(
                    row![
                        container(text(format!("[{}]", el.id)).size(9.5).font(MONO))
                            .style(theme::tag_badge)
                            .padding(Padding::from([1.0, 3.0])),
                        column![
                            text(el.name_pt.clone())
                                .size(11.0)
                                .font(UI)
                                .width(Length::Fill),
                            text(format!("{}: {}", el.family, el.kind))
                                .size(9.0)
                                .font(MONO)
                                .color(t.text2),
                        ]
                        .spacing(1)
                        .width(Length::Fill),
                        badge_or_pins,
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                )
                .style(theme::catalog_item(is_selected))
                .padding(Padding::from([3.0, 5.0]))
                .width(Length::Fill);

                let item_area = mouse_area(item_row)
                    .interaction(iced::mouse::Interaction::Pointer)
                    .on_press(Message::StartCanonicalSymbolDrag(el.id.clone()))
                    .on_release(Message::SelectCanonicalSymbol(el.id.clone()));

                list_col = list_col.push(item_area);
            }
        }
        let list_scroll = container(scrollable(list_col)).height(Length::Fill);

        // 8. Preview box na parte inferior do painel
        let preview_content = if let Some(ref id) = self.selected_canonical_id {
            if let Some(dict) = crate::canonical::global_dictionary() {
                if let Some(el) = dict.get_by_id(id) {
                    let svg_preview: Element<'_, Message> =
                        if let Some(path) = dict.resolve_svg_path(&el.id, None) {
                            if let Ok(raw) = std::fs::read_to_string(path) {
                                let themed = crate::symbols::adapt_svg_colors(&raw, is_dark);
                                container(
                                    svg(svg::Handle::from_memory(themed.into_bytes()))
                                        .width(Length::Fixed(80.0))
                                        .height(Length::Fixed(60.0)),
                                )
                                .align_x(iced::alignment::Horizontal::Center)
                                .align_y(iced::alignment::Vertical::Center)
                                .into()
                            } else {
                                container(text("Erro ao carregar SVG").size(10.5).color(t.danger))
                                    .into()
                            }
                        } else {
                            container(text("SVG não encontrado").size(10.5).color(t.text2)).into()
                        };

                    let term_names: Vec<&str> =
                        el.terminals.iter().map(|t| t.name.as_str()).collect();
                    let term_str = if !el.is_wireable() {
                        "Bloco de montagem visual (não simulável)".to_string()
                    } else if term_names.is_empty() {
                        "Sem bornes".to_string()
                    } else {
                        format!("Bornes: {}", term_names.join(", "))
                    };

                    column![
                        row![
                            text(el.name_pt.clone())
                                .size(11.5)
                                .font(Font {
                                    weight: iced::font::Weight::Semibold,
                                    ..UI
                                })
                                .width(Length::Fill),
                            text(format!("[{}]", el.id))
                                .size(10.0)
                                .font(MONO)
                                .color(t.primary),
                        ]
                        .align_y(Alignment::Center),
                        row![
                            text(format!("{} • {}", el.family, el.kind))
                                .size(9.5)
                                .font(MONO)
                                .color(t.text2)
                                .width(Length::Fill),
                            text(term_str).size(9.5).font(MONO).color(t.text2),
                        ]
                        .align_y(Alignment::Center),
                        svg_preview,
                        button(text("Inserir no Esquema").size(11.0).font(UI))
                            .style(theme::primary)
                            .padding(Padding::from([3.0, 8.0]))
                            .width(Length::Fill)
                            .on_press(Message::InsertSelectedCanonicalSymbol),
                    ]
                    .spacing(3)
                    .align_x(Alignment::Center)
                } else {
                    column![text("Símbolo não encontrado").size(11.5).color(t.text2)]
                }
            } else {
                column![text("Dicionário não carregado").size(11.5).color(t.text2)]
            }
        } else {
            column![text("Selecione um símbolo acima").size(11.5).color(t.text2)]
                .align_x(Alignment::Center)
        };

        let preview_dock = container(preview_content)
            .style(theme::preview_box)
            .padding(Padding::from([6.0, 10.0]))
            .width(Length::Fill)
            .height(145);

        column![
            shortcuts_box,
            horizontal_rule(1),
            search_box,
            scope_scroll,
            fam_scroll,
            sub_scroll,
            horizontal_rule(1),
            pagination_bar,
            list_scroll,
            preview_dock
        ]
        .height(Length::Fill)
        .into()
    }

    fn view_sheet_panel(&self) -> Element<'_, Message> {
        let title_line = self.view_canvas_title_line();
        let pages_bar = self.view_canvas_pages_bar();

        let is_dark = self.is_dark();

        let canvas_el = SchematicCanvas {
            doc: &self.doc,
            tool: self.tool.clone(),
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
            dragging_canonical_symbol: self.dragging_canonical_id.as_deref(),
            is_blocked: self.open_menu.is_some() || self.show_about_modal || self.show_export_modal,
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
                        let title = if let Some(sym) = find_symbol(item.tipo) {
                            sym.name.to_string()
                        } else if let Some(qet_id) = item.symbol_id_or_slug() {
                            if let Some(dict) = crate::canonical::global_dictionary() {
                                if let Some(el) = dict.get(qet_id) {
                                    el.name_pt.clone()
                                } else {
                                    "Elemento QET".to_string()
                                }
                            } else {
                                "Elemento QET".to_string()
                            }
                        } else if item.is_wire() {
                            "Cabo Condutor".to_string()
                        } else {
                            "Elemento".to_string()
                        };

                        body_col = body_col.push(
                            text(format!("#{sel_id} · {title}")).size(13).font(Font {
                                weight: iced::font::Weight::Bold,
                                ..UI
                            }),
                        );

                        if let Some(qet_id) = item.symbol_id_or_slug() {
                            if let Some(dict) = crate::canonical::global_dictionary() {
                                if let Some(el) = dict.get(qet_id) {
                                    if !el.is_wireable() {
                                        body_col = body_col.push(
                                            container(
                                                row![
                                                    crate::icons::view_icon(
                                                        crate::icons::Icon::Info,
                                                        12,
                                                        t.primary
                                                    ),
                                                    text("Bloco de Montagem Visual (Painel/Layout). Não simulável.")
                                                        .size(10.5)
                                                        .color(t.primary),
                                                ]
                                                .spacing(4)
                                                .align_y(Alignment::Center),
                                            )
                                            .style(theme::card)
                                            .padding(Padding::from([4.0, 6.0])),
                                        );
                                    }
                                }
                            }
                        }

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
                                        row![
                                            crate::icons::view_icon(
                                                crate::icons::Icon::Check,
                                                12,
                                                t.success
                                            ),
                                            text(format!(
                                                "Vinculado a '{}' ({})",
                                                parent.tag, parent_name
                                            ))
                                            .size(10.5)
                                            .color(t.success),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center),
                                    );
                                } else {
                                    let parent_display = role.parent_display_name();
                                    let mut alert_col = column![
                                        row![
                                            crate::icons::view_icon(
                                                crate::icons::Icon::Warning,
                                                12,
                                                t.amber
                                            ),
                                            text(format!(
                                                "Contato sem bobina: Nenhuma {} com a tag '{}' no esquema.",
                                                parent_display, item.tag
                                            ))
                                            .size(10.5)
                                            .color(t.amber),
                                        ]
                                        .spacing(4)
                                        .align_y(Alignment::Center),
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
                                                row![
                                                    crate::icons::view_icon(
                                                        crate::icons::Icon::Warning,
                                                        12,
                                                        t.amber
                                                    ),
                                                    text(format!(
                                                        "Tag duplicada: já existe outro aparelho primário com a tag '{}'",
                                                        item.tag
                                                    ))
                                                    .size(10.5)
                                                    .color(t.amber),
                                                ]
                                                .spacing(4)
                                                .align_y(Alignment::Center),
                                                button(
                                                    row![
                                                        crate::icons::view_icon(
                                                            crate::icons::Icon::Bolt,
                                                            12,
                                                            t.primary
                                                        ),
                                                        text(format!("Renomear para {suggested_tag}"))
                                                            .size(10.5)
                                                            .font(UI),
                                                    ]
                                                    .spacing(4)
                                                    .align_y(Alignment::Center)
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
                                            row![
                                                crate::icons::view_icon(
                                                    crate::icons::Icon::Check,
                                                    12,
                                                    t.success
                                                ),
                                                text(format!(
                                                    "Contatos associados no esquema: {} elemento(s)",
                                                    sub_contacts_count
                                                ))
                                                .size(10.5)
                                                .color(t.success),
                                            ]
                                            .spacing(4)
                                            .align_y(Alignment::Center),
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
                                                            row![
                                                                crate::icons::view_icon(
                                                                    crate::icons::Icon::Warning,
                                                                    12,
                                                                    t.amber
                                                                ),
                                                                text(format!(
                                                                    "Sequência {} já em uso por outro contato de '{}'",
                                                                    cur_seq, item.tag
                                                                ))
                                                                .size(10.5)
                                                                .color(t.amber),
                                                            ]
                                                            .spacing(4)
                                                            .align_y(Alignment::Center),
                                                            button(
                                                                row![
                                                                    crate::icons::view_icon(
                                                                        crate::icons::Icon::Bolt,
                                                                        12,
                                                                        t.primary
                                                                    ),
                                                                    text(format!(
                                                                        "Usar Sequência {} recomendada ({})",
                                                                        suggested_digit,
                                                                        kind.generate_pins(suggested_digit).join("-")
                                                                    ))
                                                                    .size(10.5)
                                                                    .font(UI),
                                                                ]
                                                                .spacing(4)
                                                                .align_y(Alignment::Center)
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
                                                    row![
                                                        crate::icons::view_icon(
                                                            crate::icons::Icon::Check,
                                                            12,
                                                            t.success
                                                        ),
                                                        text(format!(
                                                            "Sequência {} conforme IEC 60947",
                                                            cur_seq
                                                        ))
                                                        .size(10.5)
                                                        .color(t.success),
                                                    ]
                                                    .spacing(4)
                                                    .align_y(Alignment::Center),
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

                                let color_btn = |code: i32, label: &'static str| {
                                    let is_sel = item.color == code;
                                    button(
                                        row![
                                            container(Space::with_width(8).height(8))
                                                .style(theme::lamp_swatch(t.lamp_color(code))),
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
                                    color_btn(1, "Verde"),
                                    color_btn(2, "Vermelho"),
                                    color_btn(3, "Amarelo"),
                                    color_btn(4, "Azul"),
                                    color_btn(0, "Branco"),
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
                            "Dica:\n- Clique em qualquer símbolo na folha para editar suas propriedades.\n- Arraste com botão direito ou scroll para navegar.",
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
                                    row![
                                        crate::icons::view_icon(
                                            crate::icons::Icon::Search,
                                            13,
                                            t.primary
                                        ),
                                        text(format!(
                                            "Referência do Elemento Selecionado: #{} ({})",
                                            sel_id, clean_sel_tag
                                        ))
                                        .size(12)
                                        .font(Font {
                                            weight: iced::font::Weight::Bold,
                                            ..UI
                                        })
                                        .color(t.primary),
                                    ]
                                    .spacing(4)
                                    .align_y(Alignment::Center),
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
                                        let status = if self.doc.sim_active(item.id) {
                                            "[Ativo]"
                                        } else {
                                            "[Desligado]"
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
                                                format!("- bornes: {}", item.pins.join("-"))
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
                                        t.on_danger
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
        let dock_dot = container(Space::new(7, 7)).style(theme::status_dot(led_color));
        let dock_foot = container(
            row![
                dock_dot,
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

        let sep = || text("|").size(11).color(t.border);

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
                {
                    let status_color = if self.sim_running { t.primary } else { t.text2 };
                    let status_dot =
                        container(Space::new(6, 6)).style(theme::status_dot(status_color));
                    row![
                        status_dot,
                        text(&self.status_msg).size(11).color(status_color)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                },
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
