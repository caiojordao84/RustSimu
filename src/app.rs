use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{column, mouse_area, stack, Space, Stack};
use iced::{Element, Length, Subscription, Theme, Vector};

use crate::canvas_view::{CanvasMessage, EditorTool};
use crate::io::{build_dol_starter_preset, load_document, save_document};
use crate::layout::{Kind, Layout, Msg as LayoutMsg};
use crate::model::{SchematicDocument, SchematicItem};
use crate::sim::{press_component_on_sim, release_component_on_sim, step_simulation};
use crate::symbols::{
    classify_apparatus_role, detect_contact_kind, extract_tag_prefix,
    find_occupied_sequences_for_tag, find_symbol, next_sequential_tag_for_role, SymbolCategory,
};
use crate::theme::{ThemeMode, MENU_H};

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
    ToggleExportShowTerminals(bool),
    ToggleExportMonochrome(bool),
    ConfirmExport,
    FileExported(
        Option<PathBuf>,
        Box<SchematicDocument>,
        String,
        ExportFormat,
        ExportScope,
        crate::export::ExportOptions,
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
    NavigateToElement {
        page_index: usize,
        item_id: i32,
    },
    RenumberTerminalStrip(String),
    RenumberAllTerminalStrips,
    XrefSearchChanged(String),
    ModifiersChanged(iced::keyboard::Modifiers),
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
    GroupSelected,
    UngroupSelected,
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
    pub selected_ids: std::collections::HashSet<i32>,
    pub is_shift_pressed: bool,
    pub is_ctrl_pressed: bool,
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
    pub xref_search: String,
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
    pub export_show_terminals: bool,
    pub export_monochrome: bool,
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
                selected_ids: std::collections::HashSet::new(),
                is_shift_pressed: false,
                is_ctrl_pressed: false,
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
                xref_search: String::new(),
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
                export_show_terminals: false,
                export_monochrome: false,
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

    /// Seleciona um único elemento exclusivamente. Se pertencer a um grupo,
    /// todos os membros do grupo são selecionados em conjunto.
    pub fn select_single(&mut self, id: Option<i32>) {
        self.selected_ids.clear();
        if let Some(i) = id {
            if let Some(item) = self.doc.items.iter().find(|it| it.id == i) {
                if let Some(gid) = item.group_id {
                    for it in &self.doc.items {
                        if it.group_id == Some(gid) {
                            self.selected_ids.insert(it.id);
                        }
                    }
                } else {
                    self.selected_ids.insert(i);
                }
            } else {
                self.selected_ids.insert(i);
            }
        }
        self.selected_id = id;
    }

    /// Seleciona múltiplos elementos (substitui ou adiciona conforme `additive`).
    /// Se qualquer elemento pertencer a um grupo, expande para incluir todos os membros do grupo.
    pub fn select_multiple(&mut self, ids: impl IntoIterator<Item = i32>, additive: bool) {
        if !additive {
            self.selected_ids.clear();
        }
        let input_ids: Vec<i32> = ids.into_iter().collect();
        let mut group_ids = std::collections::HashSet::new();
        for id in &input_ids {
            self.selected_ids.insert(*id);
            if let Some(item) = self.doc.items.iter().find(|i| i.id == *id) {
                if let Some(gid) = item.group_id {
                    group_ids.insert(gid);
                }
            }
        }
        if !group_ids.is_empty() {
            for item in &self.doc.items {
                if let Some(gid) = item.group_id {
                    if group_ids.contains(&gid) {
                        self.selected_ids.insert(item.id);
                    }
                }
            }
        }
        self.sync_primary_selected_id();
    }

    /// Alterna a presença de um item na seleção (Shift-click ou Ctrl-click).
    /// Se o item pertencer a um grupo, todos os elementos do grupo são alternados em conjunto.
    pub fn toggle_selection(&mut self, id: i32) {
        let group_members: Vec<i32> = if let Some(item) = self.doc.items.iter().find(|i| i.id == id)
        {
            if let Some(gid) = item.group_id {
                self.doc
                    .items
                    .iter()
                    .filter(|i| i.group_id == Some(gid))
                    .map(|i| i.id)
                    .collect()
            } else {
                vec![id]
            }
        } else {
            vec![id]
        };

        let should_remove = self.selected_ids.contains(&id);
        for member_id in group_members {
            if should_remove {
                self.selected_ids.remove(&member_id);
            } else {
                self.selected_ids.insert(member_id);
            }
        }
        self.sync_primary_selected_id();
    }

    /// Anula completamente a seleção atual.
    pub fn deselect_all(&mut self) {
        self.selected_ids.clear();
        self.selected_id = None;
    }

    /// Sincroniza o item principal em foco (`selected_id`) com o conjunto `selected_ids`.
    pub fn sync_primary_selected_id(&mut self) {
        if let Some(cur) = self.selected_id {
            if !self.selected_ids.contains(&cur) {
                self.selected_id = self.selected_ids.iter().next().copied();
            }
        } else {
            self.selected_id = self.selected_ids.iter().next().copied();
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
                        "g" if ctrl && !shift => Some(Message::GroupSelected),
                        "u" if ctrl && !shift => Some(Message::UngroupSelected),
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
                iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                    Some(Message::ModifiersChanged(modifiers))
                }
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
                | Message::ModifiersChanged(_)
                | Message::Canvas(CanvasMessage::CursorMoved(_, _))
        ) {
            self.open_menu = None;
        }

        match message {
            Message::ModifiersChanged(modifiers) => {
                self.is_shift_pressed = modifiers.shift();
                self.is_ctrl_pressed = modifiers.control() || modifiers.command();
                iced::Task::none()
            }
            Message::NewDocument
            | Message::LoadPresetDol
            | Message::OpenFileDialog
            | Message::FileOpened(_)
            | Message::SelectSaveFormat(_)
            | Message::SaveDocument
            | Message::SaveFileDialog
            | Message::FileSaved(_)
            | Message::OpenExportModal
            | Message::CloseExportModal
            | Message::SelectExportFormat(_)
            | Message::SelectExportScope(_)
            | Message::SelectExportPage(_)
            | Message::ToggleExportCajetin(_)
            | Message::ToggleExportShowTerminals(_)
            | Message::ToggleExportMonochrome(_)
            | Message::ConfirmExport
            | Message::FileExported(..)
            | Message::FilePathChanged(_)
            | Message::LoadFromPath
            | Message::SaveAsCad
            | Message::SaveAsJson
            | Message::SaveAsToml
            | Message::CloseDocument
            | Message::SaveAll
            | Message::PrintDocument
            | Message::RecentProjectSelected(_)
            | Message::DeleteRecentProject(_)
            | Message::ExportDialog => self.update_file(message),

            Message::Undo
            | Message::Redo
            | Message::Cut
            | Message::Copy
            | Message::Paste
            | Message::Duplicate
            | Message::DeleteSelected
            | Message::SelectAll
            | Message::DeselectAll
            | Message::InvertSelection
            | Message::GroupSelected
            | Message::UngroupSelected
            | Message::PasteAtOriginalPos
            | Message::CopyElementInfo
            | Message::PasteElementInfo
            | Message::RotateClockwise
            | Message::RotateCounterClockwise
            | Message::RotateSelectedOrTool
            | Message::MirrorHorizontal
            | Message::MirrorVertical
            | Message::MirrorSelectedOrTool
            | Message::AlignToGrid
            | Message::FitToPage
            | Message::InsertLastElement
            | Message::ColorElement
            | Message::CancelAction
            | Message::GlobalMouseReleased => self.update_edit(message),

            Message::SelectPage(_)
            | Message::AddPageBefore(_)
            | Message::AddPageAfter(_)
            | Message::DeletePage(_)
            | Message::MovePage(..)
            | Message::StartEditingPageName(_)
            | Message::UpdateEditingPageName(_)
            | Message::CommitEditingPageName
            | Message::TogglePageContextMenu(_)
            | Message::ClosePageContextMenu
            | Message::PrevFolio
            | Message::NextFolio
            | Message::SelectFolio(_) => self.update_pages(message),

            Message::SelectCategory(_)
            | Message::CollectionSearchChanged(_)
            | Message::SelectCollectionSymbol(_)
            | Message::StartSymbolDrag(_)
            | Message::InsertSelectedCollectionSymbol
            | Message::CanonicalSearchChanged(_)
            | Message::SelectCanonicalScope(_)
            | Message::SelectCanonicalFamily(_)
            | Message::SelectCanonicalKind(_)
            | Message::SelectCanonicalPage(_)
            | Message::NextCanonicalPage
            | Message::PrevCanonicalPage
            | Message::SelectCanonicalSymbol(_)
            | Message::StartCanonicalSymbolDrag(_)
            | Message::InsertSelectedCanonicalSymbol => self.update_library(message),

            Message::EditTag(_)
            | Message::EditValue(_)
            | Message::EditPin(..)
            | Message::ToggleShowTag(_)
            | Message::ToggleShowValue(_)
            | Message::EditTitleBlockTitle(_)
            | Message::EditTitleBlockSubtitle(_)
            | Message::EditTitleBlockAuthor(_)
            | Message::SelectPropTab(_)
            | Message::SetLampColor(_)
            | Message::SetPulseDuration(_)
            | Message::ApplyIecNumberingPreset(_)
            | Message::SelectPinSequence(_)
            | Message::NavigateToElement { .. }
            | Message::RenumberTerminalStrip(_)
            | Message::RenumberAllTerminalStrips
            | Message::XrefSearchChanged(_) => self.update_properties(message),

            Message::ToggleSimulation | Message::StopSimulation | Message::SimTick => {
                self.update_simulation(message)
            }

            Message::SelectTool(_)
            | Message::ToggleGrid(_)
            | Message::ZoomIn
            | Message::ZoomOut
            | Message::ZoomReset
            | Message::ZoomContent
            | Message::ToggleShowTexts
            | Message::ToggleAutoConnect
            | Message::ToggleAutoDisconnect
            | Message::SetGridDensity(_)
            | Message::ToggleSnap
            | Message::CycleTheme
            | Message::LayoutMsg(_)
            | Message::ToggleLeftDockCollapse
            | Message::ToggleRightDockCollapse
            | Message::SelectLeftDockTab(_)
            | Message::ToggleMenu(_)
            | Message::CloseMenu
            | Message::ShowAbout
            | Message::CloseAbout
            | Message::Canvas(_) => self.update_view_layout(message),
        }
    }

    fn update_file(&mut self, message: Message) -> iced::Task<Message> {
        match message {
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
            Message::ToggleExportShowTerminals(enabled) => {
                self.export_show_terminals = enabled;
            }
            Message::ToggleExportMonochrome(enabled) => {
                self.export_monochrome = enabled;
            }
            Message::SelectExportPage(idx) => {
                self.export_page_index = idx;
            }
            Message::ConfirmExport => {
                self.show_export_modal = false;
                let fmt = self.export_format;
                let scope = self.export_scope;
                let export_options = crate::export::ExportOptions {
                    include_cajetin: self.export_include_cajetin,
                    show_terminals: self.export_show_terminals,
                    monochrome: self.export_monochrome,
                };
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
                            export_options,
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
                export_options,
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
                        let bytes =
                            crate::export::export_to_cad(&doc, export_options.include_cajetin);
                        crate::io::atomic_write(&path, bytes)
                    }
                    ExportFormat::Svg => {
                        let svg = crate::export::export_to_svg(&doc, items_filter, export_options);
                        crate::io::atomic_write(&path, svg.as_bytes())
                    }
                    ExportFormat::Png => {
                        crate::export::export_to_png(&doc, items_filter, export_options, None)
                            .and_then(|bytes| crate::io::atomic_write(&path, bytes))
                    }
                    ExportFormat::Pdf => {
                        let pdf_res = if scope == ExportScope::Project {
                            crate::export::export_project_to_pdf(&self.pages, export_options)
                        } else {
                            crate::export::export_doc_to_pdf(&doc, items_filter, export_options)
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
            Message::SaveAll => {
                self.pages[self.active_page_index].doc = self.doc.clone();
                self.is_modified = false;
                self.status_msg = "Todas as folhas salvas com sucesso.".to_string();
            }
            Message::PrintDocument => {
                self.status_msg = "Preparando esquema para impressão...".to_string();
            }
            Message::RecentProjectSelected(path_str) => {
                let p = PathBuf::from(&path_str);
                return iced::Task::done(Message::FileOpened(Some(p)));
            }
            Message::DeleteRecentProject(path_str) => {
                self.recent_files.retain(|p| p != &path_str);
                self.status_msg = "Projeto removido da lista recente.".to_string();
            }
            Message::ExportDialog => {
                return self.update(Message::OpenExportModal);
            }
            _ => {}
        }
        iced::Task::none()
    }

    fn update_edit(&mut self, message: Message) -> iced::Task<Message> {
        match message {
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
                if !self.selected_ids.is_empty() {
                    self.push_undo();
                    let count = self.selected_ids.len();
                    self.doc
                        .items
                        .retain(|i| !self.selected_ids.contains(&i.id));
                    self.deselect_all();
                    self.is_modified = true;
                    self.status_msg = format!("{count} elemento(s) excluído(s).");
                } else if let Some(id) = self.selected_id.take() {
                    self.push_undo();
                    self.doc.items.retain(|i| i.id != id);
                    self.is_modified = true;
                    self.status_msg = format!("Item #{id} excluído.");
                }
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
                self.selected_ids = self.doc.items.iter().map(|i| i.id).collect();
                self.selected_id = self.selected_ids.iter().next().copied();
                self.status_msg = format!(
                    "Todos os elementos selecionados (total: {}).",
                    self.selected_ids.len()
                );
            }
            Message::DeselectAll => {
                self.deselect_all();
                self.status_msg = "Seleção anulada.".to_string();
            }
            Message::InvertSelection => {
                let all_ids: std::collections::HashSet<i32> =
                    self.doc.items.iter().map(|i| i.id).collect();
                self.selected_ids = all_ids.difference(&self.selected_ids).copied().collect();
                self.sync_primary_selected_id();
                self.status_msg =
                    format!("Seleção invertida ({} elementos).", self.selected_ids.len());
            }
            Message::GroupSelected => {
                if self.selected_ids.len() < 2 {
                    self.status_msg =
                        "Selecione ao menos 2 elementos para agrupar (Ctrl+G).".to_string();
                } else {
                    self.push_undo();
                    let max_gid = self
                        .doc
                        .items
                        .iter()
                        .filter_map(|i| i.group_id)
                        .max()
                        .unwrap_or(0);
                    let new_gid = max_gid + 1;
                    let mut count = 0;
                    for item in &mut self.doc.items {
                        if self.selected_ids.contains(&item.id) {
                            item.group_id = Some(new_gid);
                            count += 1;
                        }
                    }
                    self.is_modified = true;
                    self.status_msg = format!("{count} elementos agrupados no Grupo #{new_gid}.");
                }
            }
            Message::UngroupSelected => {
                let groups_to_dissolve: std::collections::HashSet<u32> = self
                    .doc
                    .items
                    .iter()
                    .filter(|i| self.selected_ids.contains(&i.id))
                    .filter_map(|i| i.group_id)
                    .collect();

                if groups_to_dissolve.is_empty() {
                    self.status_msg =
                        "Nenhum grupo encontrado na seleção atual para desagrupar (Ctrl+U)."
                            .to_string();
                } else {
                    self.push_undo();
                    let mut count = 0;
                    for item in &mut self.doc.items {
                        if let Some(gid) = item.group_id {
                            if groups_to_dissolve.contains(&gid) {
                                item.group_id = None;
                                count += 1;
                            }
                        }
                    }
                    self.is_modified = true;
                    self.status_msg = format!(
                        "Grupo(s) desagrupado(s) ({count} elementos tornaram-se independentes)."
                    );
                }
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
            _ => {}
        }
        iced::Task::none()
    }

    fn update_pages(&mut self, message: Message) -> iced::Task<Message> {
        match message {
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
            _ => {}
        }
        iced::Task::none()
    }

    fn update_library(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SelectCategory(cat) => {
                self.active_category = cat;
            }
            Message::CollectionSearchChanged(query) => {
                self.collection_search = query;
            }
            Message::StartSymbolDrag(tipo) => {
                self.dragging_symbol = Some(tipo);
                self.selected_collection_sym = Some(tipo);
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
            _ => {}
        }
        iced::Task::none()
    }

    fn update_properties(&mut self, message: Message) -> iced::Task<Message> {
        match message {
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
            Message::SelectPropTab(tab) => {
                self.prop_tab = tab;
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
            Message::XrefSearchChanged(q) => {
                self.xref_search = q;
            }
            Message::NavigateToElement {
                page_index,
                item_id,
            } => {
                if page_index < self.pages.len() && page_index != self.active_page_index {
                    self.pages[self.active_page_index].doc = self.doc.clone();
                    self.active_page_index = page_index;
                    self.doc = self.pages[page_index].doc.clone();
                }
                self.select_single(Some(item_id));
                if let Some(item) = self.doc.items.iter().find(|i| i.id == item_id) {
                    let (sheet_w, sheet_h) = crate::xref::sheet_dimensions(&self.doc);
                    let cell = crate::xref::calculate_grid_cell(
                        item.x1 as f32,
                        item.y1 as f32,
                        sheet_w,
                        sheet_h,
                    );
                    self.status_msg = format!(
                        "Navegado para elemento #{} '{}' na Folha {} ({cell})",
                        item_id,
                        item.tag,
                        page_index + 1
                    );
                }
            }
            Message::RenumberTerminalStrip(strip_tag) => {
                self.push_undo();
                crate::xref::renumber_terminal_strip(
                    &strip_tag,
                    &mut self.pages,
                    self.active_page_index,
                    &mut self.doc,
                );
                self.is_modified = true;
                self.status_msg = format!(
                    "Régua de bornes '{}' renumerada sequencialmente.",
                    strip_tag
                );
            }
            Message::RenumberAllTerminalStrips => {
                self.push_undo();
                crate::xref::renumber_all_terminal_strips(
                    &mut self.pages,
                    self.active_page_index,
                    &mut self.doc,
                );
                self.is_modified = true;
                self.status_msg =
                    "Todas as réguas de bornes do projeto foram renumeradas sequencialmente."
                        .to_string();
            }
            _ => {}
        }
        iced::Task::none()
    }

    fn update_simulation(&mut self, message: Message) -> iced::Task<Message> {
        match message {
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
            Message::SimTick if self.sim_running => {
                step_simulation(&mut self.doc, 80);
            }
            _ => {}
        }
        iced::Task::none()
    }

    fn update_view_layout(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SelectTool(tool) => {
                if self.sim_running && !matches!(tool, EditorTool::Select) {
                    self.sim_running = false;
                    self.doc.reset_simulation_state();
                    self.status_msg = "Simulação encerrada para edição.".to_string();
                }
                self.tool = tool;
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
            Message::Canvas(cmsg) => match cmsg {
                CanvasMessage::SelectItem(sel) => {
                    self.select_single(sel);
                }
                CanvasMessage::ToggleItemSelection(id) => {
                    self.toggle_selection(id);
                }
                CanvasMessage::SelectItems { ids, additive } => {
                    self.select_multiple(ids, additive);
                    let count = self.selected_ids.len();
                    if count > 1 {
                        self.status_msg = format!("{count} elementos selecionados.");
                    }
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
                                group_id: None,
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
            Message::ToggleSnap => {
                self.snap_to_grid = !self.snap_to_grid;
                self.status_msg = if self.snap_to_grid {
                    "Snap à grelha ativado.".to_string()
                } else {
                    "Snap à grelha desativado.".to_string()
                };
            }
            _ => {}
        }
        iced::Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let main_ui = column![
            crate::ui::view_menubar(self),
            crate::ui::view_toolbar(self),
            self.layout.view(
                self.left_dock_collapsed,
                self.right_dock_collapsed,
                Message::LayoutMsg,
                |kind| match kind {
                    Kind::Collections => crate::ui::view_collections_panel(self),
                    Kind::Sheet => crate::ui::view_sheet_panel(self),
                    Kind::Properties => crate::ui::view_properties_panel(self),
                },
            ),
            crate::ui::view_statusbar(self),
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

            let dropdown_content = column![
                Space::with_height(MENU_H),
                crate::ui::view_dropdown_menu(self, active),
            ]
            .width(Length::Fill);

            let menu_overlay = stack![backdrop, dropdown_content,]
                .width(Length::Fill)
                .height(Length::Fill);

            layers.push(menu_overlay.into());
        }

        // Modal Sobre o RustSimu: camada modal elevada com escurecimento de fundo
        if self.show_about_modal {
            layers.push(crate::ui::view_about_modal(self));
        }

        // Modal de Exportação Universal (Decisão D1)
        if self.show_export_modal {
            layers.push(crate::ui::view_export_modal(self));
        }

        Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn view_toolbar(&self) -> Element<'_, Message> {
        crate::ui::view_toolbar(self)
    }
}
