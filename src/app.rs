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

/// Padrão funcional de bobinas e temporizadores (IEC 60947 / EN 50012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoilPattern {
    Standard, // Instantânea (Contator / Relé auxiliar) - tipo 9000
    OnDelay,  // Temporizador ao Trabalho (TON) - tipo 9004
    OffDelay, // Temporizador ao Repouso (TOF) - tipo 9005
    Cyclic,   // Temporizador Cíclico / Intermitente - subvariant 2
    Latching, // Bobina com Retenção (Biestável / Latch) - subvariant 1
}

impl CoilPattern {
    pub const ALL: [CoilPattern; 5] = [
        CoilPattern::Standard,
        CoilPattern::OnDelay,
        CoilPattern::OffDelay,
        CoilPattern::Cyclic,
        CoilPattern::Latching,
    ];

    pub fn from_item(item: &crate::model::SchematicItem) -> Self {
        if item.subvariant == 1 {
            CoilPattern::Latching
        } else if item.subvariant == 2 {
            CoilPattern::Cyclic
        } else if item.tipo == 9004 {
            CoilPattern::OnDelay
        } else if item.tipo == 9005 {
            CoilPattern::OffDelay
        } else {
            CoilPattern::Standard
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CoilPattern::Standard => "Padrão Instantânea (Contator)",
            CoilPattern::OnDelay => "Ao Trabalho (TON / On-Delay)",
            CoilPattern::OffDelay => "Ao Repouso (TOF / Off-Delay)",
            CoilPattern::Cyclic => "Cíclico / Pulso Intermitente",
            CoilPattern::Latching => "Retenção / Biestável (Latching)",
        }
    }
}

impl std::fmt::Display for CoilPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
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
pub use crate::sim::timers::TimerScaleMultiplier;

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

/// Opções do dropdown de navegação de páginas do projeto na aba Projetos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPageOption {
    pub index: usize,
    pub label: String,
}

impl std::fmt::Display for ProjectPageOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// Opções do dropdown de visibilidade de textos e tags do esquema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextVisibility {
    #[default]
    On,
    Off,
}

impl TextVisibility {
    pub const ALL: [Self; 2] = [Self::On, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::On => "Textos: Ligado",
            Self::Off => "Textos: Desligado",
        }
    }
}

impl std::fmt::Display for TextVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
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
    EditFunction(String),
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
    StartEditingPageNumber(usize),
    UpdateEditingPageNumber(String),
    CommitEditingPageNumber,
    TogglePageContextMenu(usize),
    ClosePageContextMenu,
    SaveAndCloseLastPage,
    ConfirmCloseLastPage,
    CancelCloseLastPage,
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
    SaveCopyDialog,
    FileCopySaved(Option<PathBuf>),
    ClearRecentFiles,
    PrintDocument,
    OpenPrintModal,
    ClosePrintModal,
    SelectPrintScope(ExportScope),
    SelectPrintPage(usize),
    TogglePrintCajetin(bool),
    TogglePrintMonochrome(bool),
    ConfirmPrint,
    SavePrintPdf,
    ShowAbout,
    CloseAbout,
    ToggleLeftDockCollapse,
    ToggleRightDockCollapse,
    SelectLeftDockTab(usize),
    RecentProjectSelected(String),
    SelectRecentProject(String),
    RequestDeleteRecentProject(String),
    ConfirmDeleteRecentProject,
    CancelDeleteRecentProject,
    DeleteRecentProject(String),
    SetLampColor(i32),
    SetPulseDuration(String),
    TogglePulsingSignaling(bool),
    SetCoilPattern(CoilPattern),
    SetTimerBaseValue(String),
    SetTimerScaleMultiplier(TimerScaleMultiplier),
    AdjustTimerPreset(f64),
    ApplyTimerQuickPreset(f64),
    SetTimerCyclicOffBaseValue(String),
    SetTimerCyclicOffMultiplier(TimerScaleMultiplier),
    AdjustTimerCyclicOff(f64),
    ResetTimerSimulation(i32),
    ApplyTerminalStripTagPreset(&'static str),
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
    SetTextVisibility(TextVisibility),
    CloseColorModal,
    SelectColor(i32),
    InputColorHex(String),
    ConfirmColorModal,
    ApplyColor(i32),
    InsertImageDialog,
    ImageFileSelected(Option<PathBuf>),
    AddGridRow,
    RemoveGridRow,
    AddGridCol,
    RemoveGridCol,
    OpenPagePropertiesModal,
    ClosePagePropertiesModal,
    EditPagePropName(String),
    EditPagePropTitle(String),
    EditPagePropSubtitle(String),
    EditPagePropEntity(String),
    EditPagePropAuthor(String),
    EditPagePropWidthMm(String),
    EditPagePropHeightMm(String),
    ApplyPageProperties,
    EditItemTrailingText(String),
    EditItemArcStart(String),
    EditItemArcSweep(String),
    ToggleSubmenu(&'static str),
    ShowProjectsDock,
    ToggleFullCrosshair,
    ToggleTextGrid,
    ToggleAlignmentGuides,
    ToggleKeepHeadersVisible,
    ToggleCellBounds,
    ToggleShowTerminals,
    ToggleShowCajetin,
    SetSheetBgPreset(Option<i32>),
    ZoomFit,
    CenterOnCursor,
    OpenProjectPropertiesModal,
    CloseProjectPropertiesModal,
    EditProjectPropTitle(String),
    EditProjectPropEntity(String),
    EditProjectPropAuthor(String),
    EditProjectPropVersion(String),
    ToggleProjectAutoSave(bool),
    ApplyProjectProperties,
    OpenSettingsModal,
    CloseSettingsModal,
    SetThemeMode(ThemeMode),
    RefreshSystemTheme,
    SystemThemeDetected(bool),
    OpenShortcutsModal,
    CloseShortcutsModal,
    ToggleFocusCanvas,
    ExitApplication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveMenu {
    Arquivo,
    Editar,
    Projeto,
    Exibicao,
    Configuracoes,
    Janela,
    Ajuda,
}

pub struct CadeSimuApp {
    pub doc: SchematicDocument,
    pub pages: Vec<crate::model::ProjectPage>,
    pub active_page_index: usize,
    pub is_modified: bool,
    pub editing_page_name: Option<String>,
    pub editing_page_target: Option<usize>,
    pub editing_page_number: Option<(usize, String)>,
    pub last_page_tab_click: Option<(usize, std::time::Instant)>,
    pub page_context_menu: Option<usize>,
    pub show_close_last_page_modal: bool,
    pub pending_close_after_save: bool,
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
    pub selected_recent_project: Option<String>,
    pub last_recent_project_click: Option<(String, std::time::Instant)>,
    pub show_confirm_delete_modal: bool,
    pub project_to_delete: Option<String>,
    pub left_dock_collapsed: bool,
    pub right_dock_collapsed: bool,
    pub open_menu: Option<ActiveMenu>,
    pub show_about_modal: bool,
    pub show_export_modal: bool,
    pub show_print_modal: bool,
    pub show_page_properties_modal: bool,
    pub show_color_modal: bool,
    pub page_prop_name: String,
    pub page_prop_title: String,
    pub page_prop_subtitle: String,
    pub page_prop_entity: String,
    pub page_prop_author: String,
    pub page_prop_width_mm: String,
    pub page_prop_height_mm: String,
    pub pending_color_hex: String,
    pub pending_color_int: i32,
    pub export_format: ExportFormat,
    pub export_scope: ExportScope,
    pub export_page_index: usize,
    pub export_include_cajetin: bool,
    pub export_show_terminals: bool,
    pub export_monochrome: bool,
    pub print_scope: ExportScope,
    pub print_page_index: usize,
    pub print_include_cajetin: bool,
    pub print_monochrome: bool,
    pub open_submenu: Option<&'static str>,
    pub show_full_crosshair: bool,
    pub show_text_grid: bool,
    pub show_alignment_guides: bool,
    pub keep_headers_visible: bool,
    pub show_cell_bounds: bool,
    pub show_terminals: bool,
    pub show_cajetin: bool,
    pub show_project_properties_modal: bool,
    pub show_settings_modal: bool,
    pub show_shortcuts_modal: bool,
    pub proj_prop_title: String,
    pub proj_prop_entity: String,
    pub proj_prop_author: String,
    pub proj_prop_version: String,
    pub proj_prop_auto_save: bool,
    pub focus_canvas: bool,
    pub undo_stack: Vec<SchematicDocument>,
    pub redo_stack: Vec<SchematicDocument>,
    pub drag_undo_snapshot: Option<SchematicDocument>,
    pub clipboard: Option<Vec<SchematicItem>>,
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

async fn pick_save_project_path(
    title: String,
    start_dir: PathBuf,
    default_name: String,
    preferred_fmt: SaveFormat,
) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title(&title)
        .set_directory(&start_dir)
        .set_file_name(&default_name);

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
                editing_page_target: None,
                editing_page_number: None,
                last_page_tab_click: None,
                page_context_menu: None,
                show_close_last_page_modal: false,
                pending_close_after_save: false,
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
                selected_recent_project: None,
                last_recent_project_click: None,
                show_confirm_delete_modal: false,
                project_to_delete: None,
                left_dock_collapsed: false,
                right_dock_collapsed: false,
                open_menu: None,
                show_about_modal: false,
                show_export_modal: false,
                show_print_modal: false,
                show_page_properties_modal: false,
                show_color_modal: false,
                page_prop_name: "Potência e Comando".to_string(),
                page_prop_title: String::new(),
                page_prop_subtitle: String::new(),
                page_prop_entity: String::new(),
                page_prop_author: String::new(),
                page_prop_width_mm: "210".to_string(),
                page_prop_height_mm: "279".to_string(),
                pending_color_hex: "#000000".to_string(),
                pending_color_int: 0,
                export_format: ExportFormat::Cad,
                export_scope: ExportScope::Page,
                export_page_index: 0,
                export_include_cajetin: true,
                export_show_terminals: false,
                export_monochrome: false,
                print_scope: ExportScope::Page,
                print_page_index: 0,
                print_include_cajetin: true,
                print_monochrome: false,
                open_submenu: None,
                show_full_crosshair: false,
                show_text_grid: false,
                show_alignment_guides: false,
                keep_headers_visible: false,
                show_cell_bounds: false,
                show_terminals: true,
                show_cajetin: true,
                show_project_properties_modal: false,
                show_settings_modal: false,
                show_shortcuts_modal: false,
                proj_prop_title: String::new(),
                proj_prop_entity: String::new(),
                proj_prop_author: String::new(),
                proj_prop_version: "Rev 1.0".to_string(),
                proj_prop_auto_save: false,
                focus_canvas: false,
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
                let count = self.selected_ids.len();
                self.status_msg = if count > 1 {
                    format!("{count} elementos movidos.")
                } else {
                    "Elemento movido.".to_string()
                };
            }
        }
    }

    pub fn is_dark(&self) -> bool {
        match self.theme_mode.resolve(self.system_is_dark) {
            crate::theme::AppTheme::Dark => true,
            crate::theme::AppTheme::Light => false,
        }
    }

    /// Reorganiza os IDs sequenciais das folhas (1..=N) mantendo consistência visual e estrutural.
    pub fn renumber_pages(&mut self) {
        for (idx, page) in self.pages.iter_mut().enumerate() {
            page.id = idx + 1;
        }
    }

    /// Reordena uma folha de `from_idx` para `to_idx`, preservando a folha ativa e a numeração sequencial.
    fn move_page(&mut self, from_idx: usize, to_idx: usize) -> bool {
        if from_idx >= self.pages.len() || to_idx >= self.pages.len() || from_idx == to_idx {
            return false;
        }
        self.pages[self.active_page_index].doc = self.doc.clone();
        let moved = self.pages.remove(from_idx);
        self.pages.insert(to_idx, moved);
        if self.active_page_index == from_idx {
            self.active_page_index = to_idx;
        } else if from_idx < self.active_page_index && to_idx >= self.active_page_index {
            self.active_page_index -= 1;
        } else if from_idx > self.active_page_index && to_idx <= self.active_page_index {
            self.active_page_index += 1;
        }
        self.doc = self.pages[self.active_page_index].doc.clone();
        self.renumber_pages();
        self.is_modified = true;
        true
    }

    /// Reinicia a área de trabalho para uma única folha limpa sem arquivo vinculado.
    fn reset_to_blank_workspace(&mut self, status_msg: String) {
        self.sim_running = false;
        self.doc = SchematicDocument::default();
        self.pages = vec![crate::model::ProjectPage::new(
            1,
            "Folha 1",
            SchematicDocument::default(),
        )];
        self.active_page_index = 0;
        self.current_file = None;
        self.file_path.clear();
        self.qet_project = None;
        self.selected_id = None;
        self.selected_ids.clear();
        self.is_modified = false;
        self.editing_page_name = None;
        self.editing_page_target = None;
        self.editing_page_number = None;
        self.last_page_tab_click = None;
        self.page_context_menu = None;
        self.show_close_last_page_modal = false;
        self.pending_close_after_save = false;
        self.status_msg = status_msg;
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

    /// Sincroniza o documento em edição com a página ativa na lista de páginas.
    pub fn sync_active_page(&mut self) {
        if self.active_page_index < self.pages.len() {
            self.pages[self.active_page_index].doc = self.doc.clone();
        }
    }

    /// Registra um arquivo no topo da lista dinâmica de "Últimos Arquivos".
    pub fn push_recent_file(&mut self, path: &std::path::Path) {
        let p_str = path.to_string_lossy().to_string();
        if p_str.is_empty() {
            return;
        }
        self.recent_files.retain(|x| x != &p_str);
        self.recent_files.insert(0, p_str);
        if self.recent_files.len() > 25 {
            self.recent_files.truncate(25);
        }
    }

    fn write_project_to_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.sync_active_page();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "qet" {
            crate::qet::export_pages_to_qet(
                &self.pages,
                self.active_page_index,
                path,
                self.qet_project.as_ref(),
            )
        } else {
            crate::io::save_project(path, &self.pages, self.active_page_index)
        }
    }

    /// Gera os bytes PDF vetoriais prontos para impressão conforme as opções do modal de impressão.
    pub fn build_print_pdf_bytes(&self) -> Result<Vec<u8>, String> {
        let opts = crate::export::ExportOptions {
            include_cajetin: self.print_include_cajetin,
            show_terminals: false,
            monochrome: self.print_monochrome,
        };
        if self.print_scope == ExportScope::Project && self.pages.len() > 1 {
            let mut snapshot = self.pages.clone();
            if self.active_page_index < snapshot.len() {
                snapshot[self.active_page_index].doc = self.doc.clone();
            }
            crate::export::export_project_to_pdf(&snapshot, opts)
        } else {
            let page_idx = self
                .print_page_index
                .min(self.pages.len().saturating_sub(1));
            let doc_ref = if page_idx == self.active_page_index {
                &self.doc
            } else if page_idx < self.pages.len() {
                &self.pages[page_idx].doc
            } else {
                &self.doc
            };
            crate::export::export_doc_to_pdf(doc_ref, None, opts)
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
        if self.theme_mode == ThemeMode::System {
            subs.push(
                iced::time::every(Duration::from_secs(4)).map(|_| Message::RefreshSystemTheme),
            );
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
                Key::Named(Named::F1) => Some(Message::OpenShortcutsModal),
                Key::Named(Named::Delete) => Some(Message::DeleteSelected),
                Key::Named(Named::Enter) if ctrl && shift => Some(Message::OpenPagePropertiesModal),
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
                        "s" if ctrl && alt => Some(Message::SaveCopyDialog),
                        "s" if ctrl => Some(Message::SaveDocument),
                        "w" if ctrl => Some(Message::CloseDocument),
                        "p" if ctrl => Some(Message::OpenPrintModal),
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
                | Message::RefreshSystemTheme
                | Message::SystemThemeDetected(_)
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
            | Message::SaveCopyDialog
            | Message::FileCopySaved(_)
            | Message::ClearRecentFiles
            | Message::PrintDocument
            | Message::OpenPrintModal
            | Message::ClosePrintModal
            | Message::SelectPrintScope(_)
            | Message::SelectPrintPage(_)
            | Message::TogglePrintCajetin(_)
            | Message::TogglePrintMonochrome(_)
            | Message::ConfirmPrint
            | Message::SavePrintPdf
            | Message::RecentProjectSelected(_)
            | Message::SelectRecentProject(_)
            | Message::RequestDeleteRecentProject(_)
            | Message::ConfirmDeleteRecentProject
            | Message::CancelDeleteRecentProject
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
            | Message::CloseColorModal
            | Message::SelectColor(_)
            | Message::InputColorHex(_)
            | Message::ConfirmColorModal
            | Message::ApplyColor(_)
            | Message::InsertImageDialog
            | Message::ImageFileSelected(_)
            | Message::AddGridRow
            | Message::RemoveGridRow
            | Message::AddGridCol
            | Message::RemoveGridCol
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
            | Message::StartEditingPageNumber(_)
            | Message::UpdateEditingPageNumber(_)
            | Message::CommitEditingPageNumber
            | Message::TogglePageContextMenu(_)
            | Message::ClosePageContextMenu
            | Message::SaveAndCloseLastPage
            | Message::ConfirmCloseLastPage
            | Message::CancelCloseLastPage
            | Message::PrevFolio
            | Message::NextFolio
            | Message::SelectFolio(_)
            | Message::OpenPagePropertiesModal
            | Message::ClosePagePropertiesModal
            | Message::EditPagePropName(_)
            | Message::EditPagePropTitle(_)
            | Message::EditPagePropSubtitle(_)
            | Message::EditPagePropEntity(_)
            | Message::EditPagePropAuthor(_)
            | Message::EditPagePropWidthMm(_)
            | Message::EditPagePropHeightMm(_)
            | Message::ApplyPageProperties
            | Message::OpenProjectPropertiesModal
            | Message::CloseProjectPropertiesModal
            | Message::EditProjectPropTitle(_)
            | Message::EditProjectPropEntity(_)
            | Message::EditProjectPropAuthor(_)
            | Message::EditProjectPropVersion(_)
            | Message::ToggleProjectAutoSave(_)
            | Message::ApplyProjectProperties => self.update_pages(message),

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
            | Message::EditFunction(_)
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
            | Message::TogglePulsingSignaling(_)
            | Message::SetCoilPattern(_)
            | Message::SetTimerBaseValue(_)
            | Message::SetTimerScaleMultiplier(_)
            | Message::AdjustTimerPreset(_)
            | Message::ApplyTimerQuickPreset(_)
            | Message::SetTimerCyclicOffBaseValue(_)
            | Message::SetTimerCyclicOffMultiplier(_)
            | Message::AdjustTimerCyclicOff(_)
            | Message::ResetTimerSimulation(_)
            | Message::ApplyTerminalStripTagPreset(_)
            | Message::ApplyIecNumberingPreset(_)
            | Message::SelectPinSequence(_)
            | Message::NavigateToElement { .. }
            | Message::RenumberTerminalStrip(_)
            | Message::RenumberAllTerminalStrips
            | Message::XrefSearchChanged(_)
            | Message::EditItemTrailingText(_)
            | Message::EditItemArcStart(_)
            | Message::EditItemArcSweep(_) => self.update_properties(message),

            Message::ToggleSimulation | Message::StopSimulation | Message::SimTick => {
                self.update_simulation(message)
            }

            Message::SelectTool(_)
            | Message::ToggleGrid(_)
            | Message::ZoomIn
            | Message::ZoomOut
            | Message::ZoomReset
            | Message::ZoomFit
            | Message::ZoomContent
            | Message::CenterOnCursor
            | Message::ToggleShowTexts
            | Message::SetTextVisibility(_)
            | Message::ToggleAutoConnect
            | Message::ToggleAutoDisconnect
            | Message::SetGridDensity(_)
            | Message::ToggleSnap
            | Message::ToggleFullCrosshair
            | Message::ToggleTextGrid
            | Message::ToggleAlignmentGuides
            | Message::ToggleKeepHeadersVisible
            | Message::ToggleCellBounds
            | Message::ToggleShowTerminals
            | Message::ToggleShowCajetin
            | Message::SetSheetBgPreset(_)
            | Message::CycleTheme
            | Message::SetThemeMode(_)
            | Message::RefreshSystemTheme
            | Message::SystemThemeDetected(_)
            | Message::LayoutMsg(_)
            | Message::ToggleLeftDockCollapse
            | Message::ToggleRightDockCollapse
            | Message::SelectLeftDockTab(_)
            | Message::ShowProjectsDock
            | Message::ToggleFocusCanvas
            | Message::ToggleMenu(_)
            | Message::ToggleSubmenu(_)
            | Message::CloseMenu
            | Message::OpenSettingsModal
            | Message::CloseSettingsModal
            | Message::OpenShortcutsModal
            | Message::CloseShortcutsModal
            | Message::ShowAbout
            | Message::CloseAbout
            | Message::ExitApplication
            | Message::Canvas(_) => self.update_view_layout(message),
        }
    }

    fn update_file(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::NewDocument => {
                self.reset_to_blank_workspace("Novo diagrama em branco criado.".to_string());
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
                self.file_path = "Partida Direta Trifásica".to_string();
                self.qet_project = None;
                self.selected_id = None;
                self.selected_ids.clear();
                self.is_modified = false;
                self.editing_page_name = None;
                self.editing_page_target = None;
                self.editing_page_number = None;
                self.page_context_menu = None;
                self.show_close_last_page_modal = false;
                self.pending_close_after_save = false;
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
                            self.push_recent_file(&path);
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
                            self.push_recent_file(&path);
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
                        let path_clone = path.clone();
                        match self.write_project_to_path(&path_clone) {
                            Ok(()) => {
                                self.is_modified = false;
                                self.push_recent_file(&path_clone);
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
                    pick_save_project_path(
                        "Salvar Projeto (.json, .toml, .qet)".to_string(),
                        start_dir,
                        default_name,
                        preferred_fmt,
                    ),
                    Message::FileSaved,
                );
            }
            Message::OpenExportModal => {
                self.sync_active_page();
                self.show_export_modal = true;
                self.export_page_index = self.active_page_index;
                self.status_msg =
                    "Exportação: Escolha o formato desejado (.png, .jpg, .bmp, .svg, .dxf, .cad ou .pdf) e a folha a exportar."
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
                    ExportFormat::Dxf => {
                        let dxf = crate::export::export_to_dxf(&doc, items_filter, export_options);
                        crate::io::atomic_write(&path, dxf)
                    }
                    ExportFormat::Png => {
                        crate::export::export_to_png(&doc, items_filter, export_options, None)
                            .and_then(|bytes| crate::io::atomic_write(&path, bytes))
                    }
                    ExportFormat::Jpg => {
                        crate::export::export_to_jpg(&doc, items_filter, export_options, None)
                            .and_then(|bytes| crate::io::atomic_write(&path, bytes))
                    }
                    ExportFormat::Bmp => {
                        crate::export::export_to_bmp(&doc, items_filter, export_options, None)
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
                    self.pending_close_after_save = false;
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

                match self.write_project_to_path(&path) {
                    Ok(()) => {
                        self.is_modified = false;
                        self.push_recent_file(&path);
                        if std::mem::take(&mut self.pending_close_after_save) {
                            self.reset_to_blank_workspace(format!(
                                "Projeto salvo em '{}' e fechado.",
                                path.display()
                            ));
                        } else {
                            self.status_msg = format!(
                                "Projeto salvo com sucesso em '{}' ({}).",
                                path.display(),
                                self.save_format
                            );
                        }
                    }
                    Err(err) => {
                        self.pending_close_after_save = false;
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
                            self.push_recent_file(&path);
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
                            self.push_recent_file(&path);
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
                self.reset_to_blank_workspace("Esquema fechado.".to_string());
            }
            Message::SaveAll => {
                self.sync_active_page();
                self.is_modified = false;
                self.status_msg = "Todas as folhas salvas com sucesso.".to_string();
            }
            Message::SaveCopyDialog => {
                self.sync_active_page();
                let start_dir = self.last_dir.clone();
                let preferred_fmt = self.save_format;
                let stem = self
                    .current_file
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("projeto_cadesimu");
                let default_name = format!("{}_copia.{}", stem, preferred_fmt.extension());
                self.status_msg = "Escolha onde salvar uma cópia do projeto...".to_string();
                return iced::Task::perform(
                    pick_save_project_path(
                        "Salvar Cópia do Projeto (.json, .toml, .qet)".to_string(),
                        start_dir,
                        default_name,
                        preferred_fmt,
                    ),
                    Message::FileCopySaved,
                );
            }
            Message::FileCopySaved(maybe_path) => {
                let Some(mut path) = maybe_path else {
                    self.status_msg = "Salvamento de cópia cancelado.".to_string();
                    return iced::Task::none();
                };
                if path.extension().is_none() {
                    path.set_extension(self.save_format.extension());
                }
                match self.write_project_to_path(&path) {
                    Ok(()) => {
                        self.push_recent_file(&path);
                        self.status_msg = format!(
                            "Cópia salva com sucesso em '{}' (arquivo original mantido).",
                            path.display()
                        );
                    }
                    Err(err) => {
                        self.status_msg =
                            format!("Erro ao salvar cópia '{}': {err}", path.display());
                    }
                }
            }
            Message::ClearRecentFiles => {
                self.recent_files.clear();
                self.selected_recent_project = None;
                self.status_msg = "Lista de últimos arquivos limpa.".to_string();
            }
            Message::PrintDocument | Message::OpenPrintModal => {
                self.sync_active_page();
                self.show_print_modal = true;
                self.print_page_index = self.active_page_index;
                self.status_msg =
                    "Impressão: Ajuste o escopo e as opções vetoriais antes de imprimir."
                        .to_string();
            }
            Message::ClosePrintModal => {
                self.show_print_modal = false;
            }
            Message::SelectPrintScope(scope) => {
                self.print_scope = scope;
            }
            Message::SelectPrintPage(idx) => {
                self.print_page_index = idx;
            }
            Message::TogglePrintCajetin(v) => {
                self.print_include_cajetin = v;
            }
            Message::TogglePrintMonochrome(v) => {
                self.print_monochrome = v;
            }
            Message::ConfirmPrint => {
                self.show_print_modal = false;
                match self.build_print_pdf_bytes() {
                    Ok(pdf_bytes) => {
                        let tmp_pdf = std::env::temp_dir().join("rustsimu_impressao.pdf");
                        if let Err(err) = crate::io::atomic_write(&tmp_pdf, &pdf_bytes) {
                            self.status_msg = format!("Erro ao preparar PDF de impressão: {err}");
                            return iced::Task::none();
                        }
                        #[cfg(target_os = "linux")]
                        let launch_res =
                            std::process::Command::new("xdg-open").arg(&tmp_pdf).spawn();
                        #[cfg(target_os = "macos")]
                        let launch_res = std::process::Command::new("open").arg(&tmp_pdf).spawn();
                        #[cfg(target_os = "windows")]
                        let launch_res = std::process::Command::new("cmd")
                            .args(["/C", "start", ""])
                            .arg(&tmp_pdf)
                            .spawn();
                        #[cfg(not(any(
                            target_os = "linux",
                            target_os = "macos",
                            target_os = "windows"
                        )))]
                        let launch_res: Result<
                            std::process::Child,
                            std::io::Error,
                        > = Err(std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            "OS não suportado",
                        ));

                        match launch_res {
                            Ok(_) => {
                                self.status_msg = format!(
                                    "PDF de impressão gerado em '{}' e enviado ao visualizador do sistema.",
                                    tmp_pdf.display()
                                );
                            }
                            Err(_) => {
                                self.status_msg = format!(
                                    "PDF de impressão gerado em '{}' ({} bytes).",
                                    tmp_pdf.display(),
                                    pdf_bytes.len()
                                );
                            }
                        }
                    }
                    Err(err) => {
                        self.status_msg = format!("Erro ao gerar impressão PDF: {err}");
                    }
                }
            }
            Message::SavePrintPdf => {
                self.show_print_modal = false;
                self.export_format = ExportFormat::Pdf;
                self.export_scope = self.print_scope;
                self.export_page_index = self.print_page_index;
                self.export_include_cajetin = self.print_include_cajetin;
                self.export_monochrome = self.print_monochrome;
                self.export_show_terminals = false;
                return self.update(Message::ConfirmExport);
            }
            Message::RecentProjectSelected(path_str) => {
                let p = PathBuf::from(&path_str);
                return iced::Task::done(Message::FileOpened(Some(p)));
            }
            Message::SelectRecentProject(path) => {
                let now = std::time::Instant::now();
                let is_double_click =
                    if let Some((ref last_path, last_time)) = self.last_recent_project_click {
                        last_path == &path
                            && now.duration_since(last_time) < std::time::Duration::from_millis(400)
                    } else {
                        false
                    };
                self.last_recent_project_click = Some((path.clone(), now));
                self.selected_recent_project = Some(path.clone());

                if is_double_click {
                    return self.update(Message::RecentProjectSelected(path));
                }
                self.status_msg = format!(
                    "Projeto recente selecionado: {path}. Pressione 'Delete' ou clique 'Apagar' para remover, ou duplo clique para abrir."
                );
            }
            Message::RequestDeleteRecentProject(path_str) => {
                self.project_to_delete = Some(path_str);
                self.show_confirm_delete_modal = true;
                self.status_msg = "Confirmar remoção de projeto recente...".to_string();
            }
            Message::ConfirmDeleteRecentProject => {
                if let Some(path_str) = self.project_to_delete.take() {
                    self.recent_files.retain(|p| p != &path_str);
                    if self.selected_recent_project.as_deref() == Some(&path_str) {
                        self.selected_recent_project = None;
                    }
                    self.status_msg = "Projeto removido da lista recente.".to_string();
                }
                self.show_confirm_delete_modal = false;
            }
            Message::CancelDeleteRecentProject => {
                self.project_to_delete = None;
                self.show_confirm_delete_modal = false;
                self.status_msg = "Exclusão de projeto recente cancelada.".to_string();
            }
            Message::DeleteRecentProject(path_str) => {
                self.recent_files.retain(|p| p != &path_str);
                if self.selected_recent_project.as_deref() == Some(&path_str) {
                    self.selected_recent_project = None;
                }
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
                return self.update_edit(Message::RotateClockwise);
            }
            Message::MirrorSelectedOrTool => {
                return self.update_edit(Message::MirrorHorizontal);
            }
            Message::DeleteSelected => {
                if self.left_dock_tab == 1 {
                    if let Some(ref path) = self.selected_recent_project {
                        return self.update(Message::RequestDeleteRecentProject(path.clone()));
                    }
                }

                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if !target_ids.is_empty() {
                    self.push_undo();
                    let count = target_ids.len();
                    self.doc.items.retain(|i| !target_ids.contains(&i.id));
                    self.deselect_all();
                    self.is_modified = true;
                    self.status_msg = format!("{count} elemento(s) excluído(s).");
                }
            }
            Message::CancelAction => {
                if self.show_close_last_page_modal {
                    self.show_close_last_page_modal = false;
                    self.pending_close_after_save = false;
                    self.status_msg = "Exclusão da última folha cancelada.".to_string();
                    return iced::Task::none();
                }
                if self.editing_page_name.is_some() || self.editing_page_number.is_some() {
                    self.editing_page_name = None;
                    self.editing_page_target = None;
                    self.editing_page_number = None;
                    self.status_msg = "Edição da aba cancelada.".to_string();
                    return iced::Task::none();
                }
                if self.show_confirm_delete_modal {
                    self.show_confirm_delete_modal = false;
                    self.project_to_delete = None;
                    self.status_msg = "Exclusão cancelada.".to_string();
                    return iced::Task::none();
                }
                if self.show_color_modal {
                    self.show_color_modal = false;
                    return iced::Task::none();
                }
                if self.show_export_modal {
                    self.show_export_modal = false;
                    return iced::Task::none();
                }
                if self.show_page_properties_modal {
                    self.show_page_properties_modal = false;
                    return iced::Task::none();
                }
                if self.show_project_properties_modal {
                    self.show_project_properties_modal = false;
                    return iced::Task::none();
                }
                if self.show_settings_modal {
                    self.show_settings_modal = false;
                    return iced::Task::none();
                }
                if self.show_shortcuts_modal {
                    self.show_shortcuts_modal = false;
                    return iced::Task::none();
                }
                if self.show_print_modal {
                    self.show_print_modal = false;
                    return iced::Task::none();
                }
                if self.show_about_modal {
                    self.show_about_modal = false;
                    return iced::Task::none();
                }
                if self.focus_canvas {
                    self.focus_canvas = false;
                }
                if let Some(snapshot) = self.drag_undo_snapshot.take() {
                    self.doc = snapshot;
                    self.status_msg = "Movimento cancelado.".to_string();
                } else {
                    self.status_msg = "Ação cancelada. Modo de seleção ativado.".to_string();
                }
                self.open_menu = None;
                self.open_submenu = None;
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
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if !target_ids.is_empty() {
                    let cut_items: Vec<SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .cloned()
                        .collect();
                    let count = cut_items.len();
                    self.clipboard = Some(cut_items);
                    self.push_undo();
                    self.doc.items.retain(|i| !target_ids.contains(&i.id));
                    self.deselect_all();
                    self.is_modified = true;
                    self.status_msg = format!("{count} elemento(s) recortado(s).");
                }
            }
            Message::Copy => {
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if !target_ids.is_empty() {
                    let copied_items: Vec<SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .cloned()
                        .collect();
                    let count = copied_items.len();
                    self.clipboard = Some(copied_items);
                    self.status_msg = format!("{count} elemento(s) copiado(s).");
                }
            }
            Message::Paste => {
                if let Some(items) = self.clipboard.clone() {
                    if !items.is_empty() {
                        self.push_undo();
                        let max_gid = self
                            .doc
                            .items
                            .iter()
                            .filter_map(|i| i.group_id)
                            .max()
                            .unwrap_or(0);
                        let mut gid_map: std::collections::HashMap<u32, u32> =
                            std::collections::HashMap::new();
                        let mut next_gid = max_gid + 1;

                        let mut new_ids = Vec::new();
                        for (new_id, item) in (self.doc.next_id()..).zip(items) {
                            let mut new_item = item;
                            new_item.id = new_id;
                            new_item.x1 += 10;
                            new_item.y1 += 10;
                            new_item.x2 += 10;
                            new_item.y2 += 10;

                            if let Some(old_gid) = new_item.group_id {
                                let mapped_gid = *gid_map.entry(old_gid).or_insert_with(|| {
                                    let g = next_gid;
                                    next_gid += 1;
                                    g
                                });
                                new_item.group_id = Some(mapped_gid);
                            }

                            let role = classify_apparatus_role(new_item.tipo);
                            if role.is_independent_apparatus() {
                                let prefix = extract_tag_prefix(&new_item.tag);
                                new_item.tag =
                                    next_sequential_tag_for_role(&self.doc.items, role, prefix);
                            }

                            self.doc.items.push(new_item);
                            new_ids.push(new_id);
                        }

                        self.selected_ids = new_ids.into_iter().collect();
                        self.sync_primary_selected_id();
                        self.is_modified = true;
                        self.status_msg =
                            format!("{} elemento(s) colado(s).", self.selected_ids.len());
                    }
                }
            }
            Message::Duplicate => {
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if !target_ids.is_empty() {
                    self.push_undo();
                    let items_to_dup: Vec<SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .cloned()
                        .collect();

                    let max_gid = self
                        .doc
                        .items
                        .iter()
                        .filter_map(|i| i.group_id)
                        .max()
                        .unwrap_or(0);
                    let mut gid_map: std::collections::HashMap<u32, u32> =
                        std::collections::HashMap::new();
                    let mut next_gid = max_gid + 1;

                    let mut new_ids = Vec::new();
                    for (new_id, item) in (self.doc.next_id()..).zip(items_to_dup) {
                        let mut new_item = item;
                        new_item.id = new_id;
                        new_item.x1 += 10;
                        new_item.y1 += 10;
                        new_item.x2 += 10;
                        new_item.y2 += 10;

                        if let Some(old_gid) = new_item.group_id {
                            let mapped_gid = *gid_map.entry(old_gid).or_insert_with(|| {
                                let g = next_gid;
                                next_gid += 1;
                                g
                            });
                            new_item.group_id = Some(mapped_gid);
                        }

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

                        self.doc.items.push(new_item);
                        new_ids.push(new_id);
                    }

                    self.selected_ids = new_ids.into_iter().collect();
                    self.sync_primary_selected_id();
                    self.is_modified = true;
                    self.status_msg =
                        format!("{} elemento(s) duplicado(s).", self.selected_ids.len());
                }
            }
            Message::RotateClockwise => {
                self.placement_orientation = (self.placement_orientation + 1) % 8;
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if target_ids.len() == 1 {
                    let id = target_ids[0];
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation = (item.orientation + 1) % 8;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Elemento #{} girado para orientação {}.",
                            id, item.orientation
                        );
                    }
                } else if target_ids.len() > 1 {
                    self.push_undo();
                    let selected_items: Vec<&SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .collect();
                    let mut min_x = i32::MAX;
                    let mut min_y = i32::MAX;
                    let mut max_x = i32::MIN;
                    let mut max_y = i32::MIN;
                    for it in &selected_items {
                        min_x = min_x.min(it.x1);
                        min_y = min_y.min(it.y1);
                        max_x = max_x.max(it.x1);
                        max_y = max_y.max(it.y1);
                        if it.is_wire() {
                            min_x = min_x.min(it.x2);
                            min_y = min_y.min(it.y2);
                            max_x = max_x.max(it.x2);
                            max_y = max_y.max(it.y2);
                        }
                    }
                    let cx = (min_x + max_x) / 2;
                    let cy = (min_y + max_y) / 2;

                    for item in &mut self.doc.items {
                        if target_ids.contains(&item.id) {
                            let rel_x = item.x1 - cx;
                            let rel_y = item.y1 - cy;
                            item.x1 = cx - rel_y;
                            item.y1 = cy + rel_x;

                            if item.is_wire() {
                                let rel_x2 = item.x2 - cx;
                                let rel_y2 = item.y2 - cy;
                                item.x2 = cx - rel_y2;
                                item.y2 = cy + rel_x2;
                            } else {
                                item.x2 = item.x1;
                                item.y2 = item.y1;
                                item.orientation = (item.orientation + 1) % 8;
                            }
                            self.is_modified = true;
                        }
                    }
                    self.status_msg = format!(
                        "{} elementos rotacionados como grupo (horário).",
                        target_ids.len()
                    );
                }
            }
            Message::RotateCounterClockwise => {
                self.placement_orientation = (self.placement_orientation + 7) % 8;
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if target_ids.len() == 1 {
                    let id = target_ids[0];
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation = (item.orientation + 7) % 8;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Elemento #{} girado anti-horário para orientação {}.",
                            id, item.orientation
                        );
                    }
                } else if target_ids.len() > 1 {
                    self.push_undo();
                    let selected_items: Vec<&SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .collect();
                    let mut min_x = i32::MAX;
                    let mut min_y = i32::MAX;
                    let mut max_x = i32::MIN;
                    let mut max_y = i32::MIN;
                    for it in &selected_items {
                        min_x = min_x.min(it.x1);
                        min_y = min_y.min(it.y1);
                        max_x = max_x.max(it.x1);
                        max_y = max_y.max(it.y1);
                        if it.is_wire() {
                            min_x = min_x.min(it.x2);
                            min_y = min_y.min(it.y2);
                            max_x = max_x.max(it.x2);
                            max_y = max_y.max(it.y2);
                        }
                    }
                    let cx = (min_x + max_x) / 2;
                    let cy = (min_y + max_y) / 2;

                    for item in &mut self.doc.items {
                        if target_ids.contains(&item.id) {
                            let rel_x = item.x1 - cx;
                            let rel_y = item.y1 - cy;
                            item.x1 = cx + rel_y;
                            item.y1 = cy - rel_x;

                            if item.is_wire() {
                                let rel_x2 = item.x2 - cx;
                                let rel_y2 = item.y2 - cy;
                                item.x2 = cx + rel_y2;
                                item.y2 = cy - rel_x2;
                            } else {
                                item.x2 = item.x1;
                                item.y2 = item.y1;
                                item.orientation = (item.orientation + 7) % 8;
                            }
                            self.is_modified = true;
                        }
                    }
                    self.status_msg = format!(
                        "{} elementos rotacionados como grupo (anti-horário).",
                        target_ids.len()
                    );
                }
            }
            Message::MirrorHorizontal => {
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if target_ids.len() == 1 {
                    let id = target_ids[0];
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation ^= 4;
                        self.is_modified = true;
                        self.status_msg = format!("Elemento #{} espelhado horizontalmente.", id);
                    }
                } else if target_ids.len() > 1 {
                    self.push_undo();
                    let selected_items: Vec<&SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .collect();
                    let mut min_x = i32::MAX;
                    let mut max_x = i32::MIN;
                    for it in &selected_items {
                        min_x = min_x.min(it.x1);
                        max_x = max_x.max(it.x1);
                        if it.is_wire() {
                            min_x = min_x.min(it.x2);
                            max_x = max_x.max(it.x2);
                        }
                    }
                    let cx = (min_x + max_x) / 2;

                    for item in &mut self.doc.items {
                        if target_ids.contains(&item.id) {
                            item.x1 = 2 * cx - item.x1;
                            if item.is_wire() {
                                item.x2 = 2 * cx - item.x2;
                            } else {
                                item.x2 = item.x1;
                                item.orientation ^= 4;
                            }
                            self.is_modified = true;
                        }
                    }
                    self.status_msg = format!(
                        "{} elementos espelhados horizontalmente como grupo.",
                        target_ids.len()
                    );
                }
            }
            Message::MirrorVertical => {
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if target_ids.len() == 1 {
                    let id = target_ids[0];
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.orientation ^= 2;
                        self.is_modified = true;
                        self.status_msg = format!("Elemento #{} espelhado verticalmente.", id);
                    }
                } else if target_ids.len() > 1 {
                    self.push_undo();
                    let selected_items: Vec<&SchematicItem> = self
                        .doc
                        .items
                        .iter()
                        .filter(|i| target_ids.contains(&i.id))
                        .collect();
                    let mut min_y = i32::MAX;
                    let mut max_y = i32::MIN;
                    for it in &selected_items {
                        min_y = min_y.min(it.y1);
                        max_y = max_y.max(it.y1);
                        if it.is_wire() {
                            min_y = min_y.min(it.y2);
                            max_y = max_y.max(it.y2);
                        }
                    }
                    let cy = (min_y + max_y) / 2;

                    for item in &mut self.doc.items {
                        if target_ids.contains(&item.id) {
                            item.y1 = 2 * cy - item.y1;
                            if item.is_wire() {
                                item.y2 = 2 * cy - item.y2;
                            } else {
                                item.y2 = item.y1;
                                item.orientation ^= 2;
                            }
                            self.is_modified = true;
                        }
                    }
                    self.status_msg = format!(
                        "{} elementos espelhados verticalmente como grupo.",
                        target_ids.len()
                    );
                }
            }
            Message::AlignToGrid => {
                let target_ids: Vec<i32> = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                let step = self.grid_density.step();
                if target_ids.len() == 1 {
                    let id = target_ids[0];
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.x1 = ((item.x1 as f32 / step as f32).round() as i32) * step;
                        item.y1 = ((item.y1 as f32 / step as f32).round() as i32) * step;
                        item.x2 = item.x1;
                        item.y2 = item.y1;
                        self.is_modified = true;
                        self.status_msg = format!(
                            "Elemento #{} alinhado à grade ({}, {}).",
                            id, item.x1, item.y1
                        );
                    }
                } else if target_ids.len() > 1 {
                    if let Some(ref_item) =
                        self.doc.items.iter().find(|i| target_ids.contains(&i.id))
                    {
                        let target_x = ((ref_item.x1 as f32 / step as f32).round() as i32) * step;
                        let target_y = ((ref_item.y1 as f32 / step as f32).round() as i32) * step;
                        let dx = target_x - ref_item.x1;
                        let dy = target_y - ref_item.y1;
                        if dx != 0 || dy != 0 {
                            self.push_undo();
                            for item in &mut self.doc.items {
                                if target_ids.contains(&item.id) {
                                    item.x1 += dx;
                                    item.y1 += dy;
                                    item.x2 += dx;
                                    item.y2 += dy;
                                    self.is_modified = true;
                                }
                            }
                            self.status_msg = format!(
                                "{} elementos alinhados à grade em conjunto.",
                                target_ids.len()
                            );
                        }
                    }
                }
            }
            Message::FitToPage => {
                self.zoom = 2.8;
                self.pan = Vector::new(20.0, 10.0);
                self.status_msg = "Visualização ajustada à página.".to_string();
            }
            Message::PasteAtOriginalPos => {
                if let Some(items) = self.clipboard.clone() {
                    if !items.is_empty() {
                        self.push_undo();
                        let max_gid = self
                            .doc
                            .items
                            .iter()
                            .filter_map(|i| i.group_id)
                            .max()
                            .unwrap_or(0);
                        let mut gid_map: std::collections::HashMap<u32, u32> =
                            std::collections::HashMap::new();
                        let mut next_gid = max_gid + 1;

                        let mut new_ids = Vec::new();
                        for (new_id, item) in (self.doc.next_id()..).zip(items) {
                            let mut new_item = item;
                            new_item.id = new_id;

                            if let Some(old_gid) = new_item.group_id {
                                let mapped_gid = *gid_map.entry(old_gid).or_insert_with(|| {
                                    let g = next_gid;
                                    next_gid += 1;
                                    g
                                });
                                new_item.group_id = Some(mapped_gid);
                            }

                            let role = classify_apparatus_role(new_item.tipo);
                            if role.is_independent_apparatus() {
                                let prefix = extract_tag_prefix(&new_item.tag);
                                new_item.tag =
                                    next_sequential_tag_for_role(&self.doc.items, role, prefix);
                            }

                            self.doc.items.push(new_item);
                            new_ids.push(new_id);
                        }

                        self.selected_ids = new_ids.into_iter().collect();
                        self.sync_primary_selected_id();
                        self.is_modified = true;
                        self.status_msg = format!(
                            "{} elemento(s) colado(s) na posição original.",
                            self.selected_ids.len()
                        );
                    }
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
                let targets = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect::<Vec<_>>()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if targets.is_empty() {
                    self.status_msg = "Selecione ao menos um elemento para colorir.".to_string();
                } else {
                    let initial_color = targets
                        .iter()
                        .find_map(|id| self.doc.items.iter().find(|i| i.id == *id).map(|i| i.color))
                        .unwrap_or(0);
                    self.pending_color_int = initial_color;
                    self.pending_color_hex = if initial_color != 0 {
                        format!("#{:06X}", initial_color & 0xFFFFFF)
                    } else {
                        "#000000".to_string()
                    };
                    self.show_color_modal = true;
                }
            }
            Message::CloseColorModal => {
                self.show_color_modal = false;
            }
            Message::SelectColor(color_code) => {
                self.pending_color_int = color_code;
                self.pending_color_hex = if color_code != 0 {
                    format!("#{:06X}", color_code & 0xFFFFFF)
                } else {
                    "#000000".to_string()
                };
            }
            Message::InputColorHex(hex) => {
                let clean = hex.trim().trim_start_matches('#').to_string();
                if clean.len() <= 6 {
                    self.pending_color_hex = if hex.starts_with('#') {
                        hex
                    } else {
                        format!("#{clean}")
                    };
                    if clean.len() == 6 {
                        if let Ok(val) = i32::from_str_radix(&clean, 16) {
                            self.pending_color_int = val;
                        }
                    }
                }
            }
            Message::ApplyColor(color_code) => {
                self.pending_color_int = color_code;
                let targets = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect::<Vec<_>>()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if !targets.is_empty() {
                    self.push_undo();
                    for item in &mut self.doc.items {
                        if targets.contains(&item.id) {
                            item.color = color_code;
                        }
                    }
                    self.is_modified = true;
                    self.status_msg = if color_code == 0 {
                        format!("Cor padrão restaurada para {} elemento(s).", targets.len())
                    } else {
                        format!(
                            "Cor #{:06X} aplicada a {} elemento(s).",
                            color_code & 0xFFFFFF,
                            targets.len()
                        )
                    };
                }
                self.show_color_modal = false;
            }
            Message::ConfirmColorModal => {
                let color_code = self.pending_color_int;
                let targets = if !self.selected_ids.is_empty() {
                    self.selected_ids.iter().copied().collect::<Vec<_>>()
                } else if let Some(id) = self.selected_id {
                    vec![id]
                } else {
                    Vec::new()
                };

                if !targets.is_empty() {
                    self.push_undo();
                    for item in &mut self.doc.items {
                        if targets.contains(&item.id) {
                            item.color = color_code;
                        }
                    }
                    self.is_modified = true;
                    self.status_msg = if color_code == 0 {
                        format!("Cor padrão restaurada para {} elemento(s).", targets.len())
                    } else {
                        format!(
                            "Cor #{:06X} aplicada a {} elemento(s).",
                            color_code & 0xFFFFFF,
                            targets.len()
                        )
                    };
                }
                self.show_color_modal = false;
            }
            Message::InsertImageDialog => {
                let start_dir = self.last_dir.clone();
                self.status_msg =
                    "Selecione uma imagem (PNG, JPG, BMP ou SVG) para inserir na folha..."
                        .to_string();
                return iced::Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Inserir Imagem no Esquema")
                            .set_directory(&start_dir)
                            .add_filter(
                                "Imagens (*.png, *.jpg, *.jpeg, *.bmp, *.svg)",
                                &[
                                    "png", "PNG", "jpg", "JPG", "jpeg", "JPEG", "bmp", "BMP",
                                    "svg", "SVG",
                                ],
                            )
                            .pick_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::ImageFileSelected,
                );
            }
            Message::ImageFileSelected(maybe_path) => {
                let Some(path) = maybe_path else {
                    self.status_msg = "Inserção de imagem cancelada.".to_string();
                    return iced::Task::none();
                };
                if let Some(parent) = path.parent() {
                    self.last_dir = parent.to_path_buf();
                }
                self.push_undo();
                let new_id = self.doc.next_id();
                let (cx, cy) = if self.cursor_world != (0, 0) {
                    self.cursor_world
                } else {
                    (60, 48)
                };
                let (w_cad, h_cad) = if let Ok(bytes) = std::fs::read(&path) {
                    if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                        let w = dyn_img.width().max(1) as f32;
                        let h = dyn_img.height().max(1) as f32;
                        let target_w = 48.0_f32;
                        let target_h = (target_w * (h / w)).clamp(18.0, 96.0);
                        (target_w.round() as i32, target_h.round() as i32)
                    } else {
                        (48, 36)
                    }
                } else {
                    (48, 36)
                };
                let mut item =
                    SchematicItem::new_graphic(new_id, 5, cx, cy, cx + w_cad, cy + h_cad);
                item.trailing_str = path.to_string_lossy().to_string();
                self.doc.items.push(item);
                self.select_single(Some(new_id));
                self.tool = EditorTool::Select;
                self.is_modified = true;
                self.status_msg = format!("Imagem '{}' inserida na folha.", path.display());
            }
            Message::AddGridRow => {
                let cur = self.doc.settings.effective_grid_rows();
                if cur < 16 {
                    self.push_undo();
                    self.doc.settings.grid_rows = cur + 1;
                    self.is_modified = true;
                    self.status_msg = format!(
                        "Linha adicionada à grade: {} linhas.",
                        self.doc.settings.grid_rows
                    );
                } else {
                    self.status_msg = "Limite máximo de 16 linhas (A-P) atingido.".to_string();
                }
            }
            Message::RemoveGridRow => {
                let cur = self.doc.settings.effective_grid_rows();
                if cur > 4 {
                    self.push_undo();
                    self.doc.settings.grid_rows = cur - 1;
                    self.is_modified = true;
                    self.status_msg = format!(
                        "Linha removida da grade: {} linhas.",
                        self.doc.settings.grid_rows
                    );
                } else {
                    self.status_msg = "Limite mínimo de 4 linhas na folha atingido.".to_string();
                }
            }
            Message::AddGridCol => {
                let cur = self.doc.settings.effective_grid_cols();
                if cur < 24 {
                    self.push_undo();
                    self.doc.settings.grid_cols = cur + 1;
                    self.is_modified = true;
                    self.status_msg = format!(
                        "Coluna adicionada à grade: {} colunas.",
                        self.doc.settings.grid_cols
                    );
                } else {
                    self.status_msg = "Limite máximo de 24 colunas atingido.".to_string();
                }
            }
            Message::RemoveGridCol => {
                let cur = self.doc.settings.effective_grid_cols();
                if cur > 4 {
                    self.push_undo();
                    self.doc.settings.grid_cols = cur - 1;
                    self.is_modified = true;
                    self.status_msg = format!(
                        "Coluna removida da grade: {} colunas.",
                        self.doc.settings.grid_cols
                    );
                } else {
                    self.status_msg = "Limite mínimo de 4 colunas na folha atingido.".to_string();
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
                if idx < self.pages.len() {
                    let now = std::time::Instant::now();
                    let is_double_click =
                        self.last_page_tab_click
                            .is_some_and(|(last_idx, last_time)| {
                                last_idx == idx
                                    && now.duration_since(last_time)
                                        < std::time::Duration::from_millis(400)
                            });
                    self.last_page_tab_click = Some((idx, now));

                    if idx != self.active_page_index {
                        self.pages[self.active_page_index].doc = self.doc.clone();
                        self.active_page_index = idx;
                        self.doc = self.pages[idx].doc.clone();
                        self.deselect_all();
                        self.sim_running = false;
                        self.editing_page_name = None;
                        self.editing_page_target = None;
                        self.editing_page_number = None;
                        self.page_context_menu = None;
                        self.status_msg =
                            format!("Folha {} ativa: {}", idx + 1, self.pages[idx].name);
                    }

                    if is_double_click {
                        self.editing_page_target = Some(idx);
                        self.editing_page_name = Some(self.pages[idx].name.clone());
                        self.editing_page_number = None;
                        self.page_context_menu = None;
                        self.last_page_tab_click = None;
                        self.status_msg = format!(
                            "Editando nome da folha {}: '{}'.",
                            idx + 1,
                            self.pages[idx].name
                        );
                    }
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
                self.renumber_pages();
                self.deselect_all();
                self.sim_running = false;
                self.is_modified = true;
                self.editing_page_name = None;
                self.editing_page_target = None;
                self.editing_page_number = None;
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
                self.renumber_pages();
                self.deselect_all();
                self.sim_running = false;
                self.is_modified = true;
                self.editing_page_name = None;
                self.editing_page_target = None;
                self.editing_page_number = None;
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
                    self.page_context_menu = None;
                    self.editing_page_name = None;
                    self.editing_page_target = None;
                    self.editing_page_number = None;
                    self.show_close_last_page_modal = true;
                    self.status_msg =
                        "Aviso: excluir a única folha fechará o projeto atual.".to_string();
                } else if idx < self.pages.len() {
                    self.pages.remove(idx);
                    if self.active_page_index >= self.pages.len() {
                        self.active_page_index = self.pages.len() - 1;
                    } else if idx < self.active_page_index {
                        self.active_page_index -= 1;
                    }
                    self.doc = self.pages[self.active_page_index].doc.clone();
                    self.renumber_pages();
                    self.deselect_all();
                    self.sim_running = false;
                    self.is_modified = true;
                    self.editing_page_name = None;
                    self.editing_page_target = None;
                    self.editing_page_number = None;
                    self.page_context_menu = None;
                    self.status_msg = format!("Folha {} excluída.", idx + 1);
                }
            }
            Message::SaveAndCloseLastPage => {
                self.show_close_last_page_modal = false;
                self.sync_active_page();
                if let Some(path) = self.current_file.clone() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_default();
                    if ext == "json" || ext == "toml" || ext == "qet" {
                        match self.write_project_to_path(&path) {
                            Ok(()) => {
                                self.push_recent_file(&path);
                                self.reset_to_blank_workspace(format!(
                                    "Projeto salvo em '{}' e fechado.",
                                    path.display()
                                ));
                            }
                            Err(err) => {
                                self.status_msg =
                                    format!("Erro ao salvar '{}': {err}", path.display());
                            }
                        }
                        return iced::Task::none();
                    }
                }
                self.pending_close_after_save = true;
                return self.update_file(Message::SaveFileDialog);
            }
            Message::ConfirmCloseLastPage => {
                self.reset_to_blank_workspace(
                    "Projeto fechado após excluir última folha.".to_string(),
                );
            }
            Message::CancelCloseLastPage => {
                self.show_close_last_page_modal = false;
                self.pending_close_after_save = false;
                self.status_msg = "Exclusão da última folha cancelada.".to_string();
            }
            Message::MovePage(from_idx, to_idx) => {
                if self.move_page(from_idx, to_idx) {
                    self.editing_page_number = None;
                    self.page_context_menu = None;
                    self.status_msg = format!("Folha movida para a posição {}.", to_idx + 1);
                }
            }
            Message::StartEditingPageName(idx) => {
                if idx < self.pages.len() {
                    self.editing_page_target = Some(idx);
                    self.editing_page_name = Some(self.pages[idx].name.clone());
                    self.editing_page_number = None;
                    self.page_context_menu = None;
                }
            }
            Message::UpdateEditingPageName(new_name) => {
                self.editing_page_name = Some(new_name);
            }
            Message::CommitEditingPageName => {
                let target_idx = self
                    .editing_page_target
                    .take()
                    .unwrap_or(self.active_page_index);
                if let Some(new_name) = self.editing_page_name.take() {
                    let trimmed = new_name.trim();
                    if !trimmed.is_empty() && target_idx < self.pages.len() {
                        self.pages[target_idx].name = trimmed.to_string();
                        self.is_modified = true;
                        self.status_msg =
                            format!("Folha {} renomeada para '{}'.", target_idx + 1, trimmed);
                    }
                }
            }
            Message::StartEditingPageNumber(idx) => {
                if self.pages.len() > 1 && idx < self.pages.len() {
                    self.editing_page_number = Some((idx, (idx + 1).to_string()));
                    self.editing_page_name = None;
                    self.editing_page_target = None;
                    self.page_context_menu = None;
                    self.status_msg = format!(
                        "Informe o novo número (1..={}) para a folha '{}'.",
                        self.pages.len(),
                        self.pages[idx].name
                    );
                }
            }
            Message::UpdateEditingPageNumber(val) => {
                if let Some((idx, _)) = self.editing_page_number {
                    let filtered: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
                    self.editing_page_number = Some((idx, filtered));
                }
            }
            Message::CommitEditingPageNumber => {
                if let Some((from_idx, raw)) = self.editing_page_number.take() {
                    if self.pages.len() > 1 && from_idx < self.pages.len() {
                        if let Ok(parsed) = raw.trim().parse::<usize>() {
                            let target_num = parsed.clamp(1, self.pages.len());
                            let to_idx = target_num - 1;
                            if self.move_page(from_idx, to_idx) {
                                self.status_msg = format!(
                                    "Número da folha atualizado para posição {}.",
                                    to_idx + 1
                                );
                            }
                        } else {
                            self.status_msg = "Número de página inválido.".to_string();
                        }
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
            Message::OpenPagePropertiesModal => {
                let page_name = self
                    .pages
                    .get(self.active_page_index)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Folha 1".to_string());
                self.page_prop_name = page_name;
                self.page_prop_title = self.doc.title_block.title.clone();
                self.page_prop_subtitle = self.doc.title_block.subtitle.clone();
                self.page_prop_entity = self.doc.title_block.entity.clone();
                self.page_prop_author = self.doc.title_block.drawn_by.clone();
                self.page_prop_width_mm = self.doc.settings.custom_width_mm.to_string();
                self.page_prop_height_mm = self.doc.settings.custom_height_mm.to_string();
                self.show_page_properties_modal = true;
            }
            Message::ClosePagePropertiesModal => {
                self.show_page_properties_modal = false;
            }
            Message::EditPagePropName(v) => {
                self.page_prop_name = v;
            }
            Message::EditPagePropTitle(v) => {
                self.page_prop_title = v;
            }
            Message::EditPagePropSubtitle(v) => {
                self.page_prop_subtitle = v;
            }
            Message::EditPagePropEntity(v) => {
                self.page_prop_entity = v;
            }
            Message::EditPagePropAuthor(v) => {
                self.page_prop_author = v;
            }
            Message::EditPagePropWidthMm(v) => {
                self.page_prop_width_mm = v;
            }
            Message::EditPagePropHeightMm(v) => {
                self.page_prop_height_mm = v;
            }
            Message::ApplyPageProperties => {
                self.push_undo();
                let new_name = self.page_prop_name.trim().to_string();
                if !new_name.is_empty() && self.active_page_index < self.pages.len() {
                    self.pages[self.active_page_index].name = new_name.clone();
                    self.doc.title_block.sheet = new_name;
                }
                self.doc.title_block.title = self.page_prop_title.clone();
                self.doc.title_block.subtitle = self.page_prop_subtitle.clone();
                self.doc.title_block.entity = self.page_prop_entity.clone();
                self.doc.title_block.drawn_by = self.page_prop_author.clone();
                if let Ok(w_mm) = self.page_prop_width_mm.trim().parse::<i32>() {
                    self.doc.settings.custom_width_mm = w_mm.clamp(100, 2000);
                }
                if let Ok(h_mm) = self.page_prop_height_mm.trim().parse::<i32>() {
                    self.doc.settings.custom_height_mm = h_mm.clamp(100, 2000);
                }
                self.sync_active_page();
                self.is_modified = true;
                self.show_page_properties_modal = false;
                self.status_msg = "Propriedades da folha e cajetín atualizadas.".to_string();
            }
            Message::OpenProjectPropertiesModal => {
                self.proj_prop_title = self.doc.title_block.title.clone();
                self.proj_prop_entity = self.doc.title_block.entity.clone();
                self.proj_prop_author = self.doc.title_block.drawn_by.clone();
                self.proj_prop_version = if self.doc.title_block.signature.trim().is_empty() {
                    "Rev 1.0".to_string()
                } else {
                    self.doc.title_block.signature.clone()
                };
                self.show_project_properties_modal = true;
                self.open_menu = None;
                self.open_submenu = None;
            }
            Message::CloseProjectPropertiesModal => {
                self.show_project_properties_modal = false;
            }
            Message::EditProjectPropTitle(v) => {
                self.proj_prop_title = v;
            }
            Message::EditProjectPropEntity(v) => {
                self.proj_prop_entity = v;
            }
            Message::EditProjectPropAuthor(v) => {
                self.proj_prop_author = v;
            }
            Message::EditProjectPropVersion(v) => {
                self.proj_prop_version = v;
            }
            Message::ToggleProjectAutoSave(v) => {
                self.proj_prop_auto_save = v;
            }
            Message::ApplyProjectProperties => {
                self.push_undo();
                self.doc.title_block.title = self.proj_prop_title.clone();
                self.doc.title_block.entity = self.proj_prop_entity.clone();
                self.doc.title_block.drawn_by = self.proj_prop_author.clone();
                self.doc.title_block.signature = self.proj_prop_version.clone();
                for page in &mut self.pages {
                    page.doc.title_block.title = self.proj_prop_title.clone();
                    page.doc.title_block.entity = self.proj_prop_entity.clone();
                    page.doc.title_block.drawn_by = self.proj_prop_author.clone();
                    page.doc.title_block.signature = self.proj_prop_version.clone();
                }
                self.sync_active_page();
                self.is_modified = true;
                self.show_project_properties_modal = false;
                self.status_msg = format!(
                    "Propriedades globais aplicadas a todas as {} folhas do projeto.",
                    self.pages.len()
                );
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
            Message::EditFunction(new_fn) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.function = if new_fn.trim().is_empty() {
                            None
                        } else {
                            Some(new_fn)
                        };
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
            Message::TogglePulsingSignaling(pulsing) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.extra_int1 = if pulsing { 1 } else { 0 };
                        if pulsing && item.value.trim().is_empty() {
                            item.value = "0.5".to_string();
                        }
                        self.is_modified = true;
                        self.status_msg = if pulsing {
                            "Sinalizador configurado para modo pulsante.".to_string()
                        } else {
                            "Sinalizador configurado para modo contínuo.".to_string()
                        };
                    }
                }
            }
            Message::SetCoilPattern(pat) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        match pat {
                            CoilPattern::Standard => {
                                item.tipo = 9000;
                                item.subvariant = 0;
                            }
                            CoilPattern::OnDelay => {
                                item.tipo = 9004;
                                item.subvariant = 0;
                                if item.pins[2].trim().is_empty() {
                                    item.pins[2] = "0".to_string(); // x 1 Seg
                                }
                                if item.pins[3].trim().is_empty() {
                                    item.pins[3] = if item.value.trim().is_empty() {
                                        "3".to_string()
                                    } else {
                                        item.value.clone()
                                    };
                                }
                                item.value = item.pins[3].clone();
                            }
                            CoilPattern::OffDelay => {
                                item.tipo = 9005;
                                item.subvariant = 0;
                                if item.pins[2].trim().is_empty() {
                                    item.pins[2] = "0".to_string(); // x 1 Seg
                                }
                                if item.pins[3].trim().is_empty() {
                                    item.pins[3] = if item.value.trim().is_empty() {
                                        "3".to_string()
                                    } else {
                                        item.value.clone()
                                    };
                                }
                                item.value = item.pins[3].clone();
                            }
                            CoilPattern::Cyclic => {
                                item.tipo = 9004;
                                item.subvariant = 2;
                                if item.pins[2].trim().is_empty() {
                                    item.pins[2] = "0".to_string(); // T_ON mult x 1 Seg
                                }
                                if item.pins[3].trim().is_empty() {
                                    item.pins[3] = if item.value.trim().is_empty() {
                                        "1".to_string()
                                    } else {
                                        item.value.clone()
                                    };
                                }
                                if item.pins[4].trim().is_empty() {
                                    item.pins[4] = "0".to_string(); // T_OFF mult x 1 Seg
                                }
                                if item.pins[5].trim().is_empty() {
                                    item.pins[5] = "1".to_string(); // T_OFF base 1s
                                }
                                item.value = item.pins[3].clone();
                            }
                            CoilPattern::Latching => {
                                item.tipo = 9000;
                                item.subvariant = 1;
                            }
                        }
                        self.is_modified = true;
                        self.status_msg =
                            format!("Padrão da bobina alterado para '{}'.", pat.label());
                    }
                }
            }
            Message::SetTimerBaseValue(val) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        let mult = crate::sim::timers::timer_scale_multiplier(&item.pins);
                        let base_num = val.trim().parse::<f64>().unwrap_or(1.0);
                        let eff_sec = (base_num * mult.factor()).max(0.01);
                        let formatted_eff = crate::sim::timers::format_timer_seconds(eff_sec);
                        item.pins[3] = formatted_eff.clone();
                        item.value = formatted_eff;
                        self.is_modified = true;
                    }
                }
            }
            Message::SetTimerScaleMultiplier(mult) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        let cur_base =
                            crate::sim::timers::timer_base_value(&item.pins, &item.value);
                        item.pins[2] = mult.code().to_string();
                        let new_eff_sec = (cur_base * mult.factor()).max(0.01);
                        let formatted_eff = crate::sim::timers::format_timer_seconds(new_eff_sec);
                        item.pins[3] = formatted_eff.clone();
                        item.value = formatted_eff;
                        self.is_modified = true;
                        self.status_msg =
                            format!("Escala do temporizador alterada para '{}'.", mult.label());
                    }
                }
            }
            Message::AdjustTimerPreset(delta) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        let mult = crate::sim::timers::timer_scale_multiplier(&item.pins);
                        let cur_eff = crate::sim::timers::timer_preset_ms(&item.pins, &item.value)
                            as f64
                            / 1000.0;
                        let new_eff = (cur_eff + delta).max(0.01);
                        let formatted_eff = crate::sim::timers::format_timer_seconds(new_eff);
                        item.pins[3] = formatted_eff.clone();
                        item.value = formatted_eff;
                        self.is_modified = true;
                        let base = new_eff / mult.factor();
                        let disp_eff = crate::sim::timers::format_timer_seconds(new_eff);
                        let disp_base = crate::sim::timers::format_timer_seconds(base);
                        self.status_msg =
                            format!("Temporização ajustada para {disp_eff}s (Base {disp_base}).");
                    }
                }
            }
            Message::ApplyTimerQuickPreset(target_sec) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        if target_sec < 1.0 {
                            item.pins[2] = crate::sim::timers::TimerScaleMultiplier::X0_1
                                .code()
                                .to_string();
                        } else if target_sec >= 60.0 && (target_sec % 60.0).abs() < 1e-6 {
                            item.pins[2] = crate::sim::timers::TimerScaleMultiplier::X60
                                .code()
                                .to_string();
                        } else if target_sec >= 10.0 && (target_sec % 10.0).abs() < 1e-6 {
                            item.pins[2] = crate::sim::timers::TimerScaleMultiplier::X10
                                .code()
                                .to_string();
                        } else {
                            item.pins[2] = crate::sim::timers::TimerScaleMultiplier::X1
                                .code()
                                .to_string();
                        }
                        let formatted = crate::sim::timers::format_timer_seconds(target_sec);
                        item.pins[3] = formatted.clone();
                        item.value = formatted.clone();
                        self.is_modified = true;
                        self.status_msg = format!("Preset rápido de {formatted}s aplicado.");
                    }
                }
            }
            Message::SetTimerCyclicOffBaseValue(val) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        let mult = crate::sim::timers::timer_off_scale_multiplier(&item.pins);
                        let base_num = val.trim().parse::<f64>().unwrap_or(1.0);
                        let eff_sec = (base_num * mult.factor()).max(0.01);
                        let formatted_eff = crate::sim::timers::format_timer_seconds(eff_sec);
                        item.pins[5] = formatted_eff;
                        self.is_modified = true;
                    }
                }
            }
            Message::SetTimerCyclicOffMultiplier(mult) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        let cur_base = crate::sim::timers::timer_off_base_value(&item.pins);
                        item.pins[4] = mult.code().to_string();
                        let new_eff_sec = (cur_base * mult.factor()).max(0.01);
                        let formatted_eff = crate::sim::timers::format_timer_seconds(new_eff_sec);
                        item.pins[5] = formatted_eff;
                        self.is_modified = true;
                        self.status_msg = format!("Escala T_OFF alterada para '{}'.", mult.label());
                    }
                }
            }
            Message::AdjustTimerCyclicOff(delta) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        let cur_eff =
                            crate::sim::timers::timer_off_preset_ms(&item.pins) as f64 / 1000.0;
                        let new_eff = (cur_eff + delta).max(0.01);
                        let formatted_eff = crate::sim::timers::format_timer_seconds(new_eff);
                        item.pins[5] = formatted_eff;
                        self.is_modified = true;
                    }
                }
            }
            Message::ResetTimerSimulation(item_id) => {
                let s = self.doc.sim.get_mut(item_id);
                s.timer_elapsed_ms = 0;
                s.timer_phase = 0;
                self.status_msg = "Temporizador reiniciado para o início da contagem.".to_string();
            }
            Message::ApplyTerminalStripTagPreset(strip_tag) => {
                if let Some(id) = self.selected_id {
                    self.push_undo();
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.tag = strip_tag.to_string();
                        self.is_modified = true;
                        self.status_msg =
                            format!("Régua de bornes classificada como '{}'.", strip_tag);
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
                            "power_3p_phases" => {
                                item.pins[0] = "L1".to_string();
                                item.pins[1] = "L2".to_string();
                                item.pins[2] = "L3".to_string();
                                item.pins[3] = "T1".to_string();
                                item.pins[4] = "T2".to_string();
                                item.pins[5] = "T3".to_string();
                            }
                            "power_1p" => {
                                item.pins[0] = "1".to_string();
                                item.pins[1] = "2".to_string();
                            }
                            "power_1p_phases" => {
                                item.pins[0] = "L1".to_string();
                                item.pins[1] = "T1".to_string();
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
            Message::EditItemTrailingText(txt) => {
                if let Some(id) = self.selected_id {
                    if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                        item.trailing_str = txt;
                        self.is_modified = true;
                    }
                }
            }
            Message::EditItemArcStart(val) => {
                if let Some(id) = self.selected_id {
                    if let Ok(deg) = val.trim().parse::<i32>() {
                        if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                            item.aux_coords[0] = deg.clamp(-360, 360);
                            self.is_modified = true;
                        }
                    }
                }
            }
            Message::EditItemArcSweep(val) => {
                if let Some(id) = self.selected_id {
                    if let Ok(deg) = val.trim().parse::<i32>() {
                        if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                            item.aux_coords[1] = deg.clamp(-360, 360);
                            self.is_modified = true;
                        }
                    }
                }
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
                if self.sim_running && !matches!(tool, EditorTool::Select | EditorTool::Pan) {
                    self.sim_running = false;
                    self.doc.reset_simulation_state();
                    self.status_msg = "Simulação encerrada para edição.".to_string();
                }
                self.tool = tool;
                match self.tool {
                    EditorTool::Select => {
                        self.status_msg =
                            "Modo Seleção: clique para selecionar ou arraste para seleção múltipla."
                                .to_string();
                    }
                    EditorTool::Pan => {
                        self.status_msg =
                            "Modo Mover: arraste o canvas livremente com o botão do mouse."
                                .to_string();
                    }
                    _ => {}
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
            Message::ZoomFit => {
                let (sheet_w, sheet_h) = crate::xref::sheet_dimensions(&self.doc);
                let left_w = if self.focus_canvas {
                    0.0
                } else if self.left_dock_collapsed {
                    28.0
                } else {
                    self.layout.left_width
                };
                let right_w = if self.focus_canvas {
                    0.0
                } else if self.right_dock_collapsed {
                    28.0
                } else {
                    self.layout.right_width
                };
                let avail_w = (self.layout.window_w - left_w - right_w - 40.0).max(240.0);
                let avail_h = 620.0_f32;
                let zx = avail_w / sheet_w.max(100.0);
                let zy = avail_h / sheet_h.max(100.0);
                self.zoom = zx.min(zy).clamp(0.6, 6.0);
                self.pan = Vector::new((avail_w - sheet_w * self.zoom) * 0.5 + 20.0, 20.0);
                self.status_msg = "Zoom ajustado às dimensões da folha.".to_string();
            }
            Message::CenterOnCursor => {
                let (tx, ty) = if self.cursor_world != (0, 0) {
                    (self.cursor_world.0 as f32, self.cursor_world.1 as f32)
                } else if let Some(sel) = self
                    .selected_id
                    .and_then(|id| self.doc.items.iter().find(|i| i.id == id))
                {
                    (sel.x1 as f32, sel.y1 as f32)
                } else {
                    let (sw, sh) = crate::xref::sheet_dimensions(&self.doc);
                    (sw * 0.5, sh * 0.5)
                };
                let left_w = if self.focus_canvas {
                    0.0
                } else if self.left_dock_collapsed {
                    28.0
                } else {
                    self.layout.left_width
                };
                let right_w = if self.focus_canvas {
                    0.0
                } else if self.right_dock_collapsed {
                    28.0
                } else {
                    self.layout.right_width
                };
                let cx_px = ((self.layout.window_w - left_w - right_w) * 0.5).max(160.0);
                let cy_px = 320.0;
                self.pan = Vector::new(cx_px - tx * self.zoom, cy_px - ty * self.zoom);
                self.status_msg = format!("Visualização centralizada em ({tx:.0}, {ty:.0}).");
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
                            let is_multi =
                                self.selected_ids.contains(&id) && self.selected_ids.len() > 1;
                            if is_multi {
                                let old_origins: Vec<(i32, i32)> = self
                                    .doc
                                    .items
                                    .iter()
                                    .filter(|i| self.selected_ids.contains(&i.id))
                                    .map(|i| (i.x1, i.y1))
                                    .collect();

                                for item in &mut self.doc.items {
                                    if self.selected_ids.contains(&item.id) {
                                        item.x1 += dx;
                                        item.y1 += dy;
                                        item.x2 += dx;
                                        item.y2 += dy;
                                        self.is_modified = true;
                                    }
                                }

                                if self.auto_disconnect {
                                    self.doc.items.retain(|i| {
                                        if i.is_wire() && !self.selected_ids.contains(&i.id) {
                                            !old_origins.iter().any(|&(cx, cy)| {
                                                (i.x1 == cx && i.y1 == cy)
                                                    || (i.x2 == cx && i.y2 == cy)
                                            })
                                        } else {
                                            true
                                        }
                                    });
                                } else {
                                    for w in self.doc.items.iter_mut() {
                                        if w.is_wire() && !self.selected_ids.contains(&w.id) {
                                            for &(cx, cy) in &old_origins {
                                                if w.x1 == cx && w.y1 == cy {
                                                    w.x1 += dx;
                                                    w.y1 += dy;
                                                }
                                                if w.x2 == cx && w.y2 == cy {
                                                    w.x2 += dx;
                                                    w.y2 += dy;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                if let Some(item) = self.doc.items.iter_mut().find(|i| i.id == id) {
                                    if item.is_wire() || item.is_graphic() {
                                        item.x1 += dx;
                                        item.y1 += dy;
                                        item.x2 += dx;
                                        item.y2 += dy;
                                    } else {
                                        item.x1 = x1;
                                        item.y1 = y1;
                                        item.x2 = x1;
                                        item.y2 = y1;
                                    }
                                    self.is_modified = true;
                                }
                                if self.auto_disconnect {
                                    // Remove fios conectados ao ponto anterior
                                    self.doc.items.retain(|i| {
                                        if i.is_wire() {
                                            !((i.x1 == ox && i.y1 == oy)
                                                || (i.x2 == ox && i.y2 == oy))
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
                                function: None,
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
                CanvasMessage::AddGraphicItem {
                    tipo,
                    subvariant,
                    x1,
                    y1,
                    x2,
                    y2,
                } => {
                    self.push_undo();
                    let new_id = self.doc.next_id();
                    let mut item = SchematicItem::new_graphic(new_id, tipo, x1, y1, x2, y2);
                    item.subvariant = subvariant;
                    if tipo == 8 {
                        item.trailing_str = if subvariant == 1 {
                            "Campo de texto".to_string()
                        } else {
                            "Texto".to_string()
                        };
                    } else if tipo == 4 {
                        item.aux_coords[0] = 0;
                        item.aux_coords[1] = 90;
                    }
                    self.doc.items.push(item);
                    self.select_single(Some(new_id));
                    self.tool = EditorTool::Select;
                    if tipo == 8 {
                        self.prop_tab = 0;
                    }
                    self.is_modified = true;
                    let kind_name = match (tipo, subvariant) {
                        (8, 1) => "Campo de Texto",
                        (8, _) => "Texto em Linha",
                        (1, _) => "Linha Gráfica",
                        (2, _) => "Retângulo",
                        (3, _) => "Círculo / Elipse",
                        (4, _) => "Arco",
                        _ => "Objeto Gráfico",
                    };
                    self.status_msg = format!("{kind_name} inserido em ({x1}, {y1}).");
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
                if self.theme_mode == ThemeMode::System {
                    self.system_is_dark = crate::theme::detect_system_dark();
                }
            }
            Message::SetThemeMode(mode) => {
                self.theme_mode = mode;
                if mode == ThemeMode::System {
                    self.system_is_dark = crate::theme::detect_system_dark();
                }
                self.status_msg = match mode {
                    ThemeMode::System => "Tema da interface: Automático (Sistema).".to_string(),
                    ThemeMode::Light => "Tema da interface: Claro.".to_string(),
                    ThemeMode::Dark => "Tema da interface: Escuro.".to_string(),
                };
            }
            Message::RefreshSystemTheme => {
                if self.theme_mode == ThemeMode::System {
                    return iced::Task::perform(
                        async { crate::theme::detect_system_dark() },
                        Message::SystemThemeDetected,
                    );
                }
            }
            Message::SystemThemeDetected(is_dark) => {
                if self.system_is_dark != is_dark {
                    self.system_is_dark = is_dark;
                    if self.theme_mode == ThemeMode::System {
                        self.status_msg = if is_dark {
                            "Tema do sistema alterado para Escuro.".to_string()
                        } else {
                            "Tema do sistema alterado para Claro.".to_string()
                        };
                    }
                }
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
                self.open_submenu = None;
            }
            Message::ToggleSubmenu(key) => {
                if self.open_submenu == Some(key) {
                    self.open_submenu = None;
                } else {
                    self.open_submenu = Some(key);
                }
            }
            Message::CloseMenu => {
                self.open_menu = None;
                self.open_submenu = None;
                self.tool = EditorTool::Select;
            }
            Message::ShowProjectsDock => {
                self.left_dock_collapsed = false;
                self.left_dock_tab = 1;
                self.focus_canvas = false;
                self.open_menu = None;
                self.open_submenu = None;
                self.status_msg = "Painel de Projetos exibido no dock esquerdo.".to_string();
            }
            Message::ToggleFullCrosshair => {
                self.show_full_crosshair = !self.show_full_crosshair;
                self.status_msg = if self.show_full_crosshair {
                    "Mira em todo o fólio ativada.".to_string()
                } else {
                    "Mira em todo o fólio desativada.".to_string()
                };
            }
            Message::ToggleTextGrid => {
                self.show_text_grid = !self.show_text_grid;
                self.status_msg = if self.show_text_grid {
                    "Grelha de alinhamento de texto exibida.".to_string()
                } else {
                    "Grelha de alinhamento de texto oculta.".to_string()
                };
            }
            Message::ToggleAlignmentGuides => {
                self.show_alignment_guides = !self.show_alignment_guides;
                self.status_msg = if self.show_alignment_guides {
                    "Guias de alinhamento entre elementos ativadas.".to_string()
                } else {
                    "Guias de alinhamento desativadas.".to_string()
                };
            }
            Message::ToggleKeepHeadersVisible => {
                self.keep_headers_visible = !self.keep_headers_visible;
                self.status_msg = if self.keep_headers_visible {
                    "Cabeçalhos da grade mantidos visíveis ao rolar.".to_string()
                } else {
                    "Cabeçalhos fixos da grade desativados.".to_string()
                };
            }
            Message::ToggleCellBounds => {
                self.show_cell_bounds = !self.show_cell_bounds;
                self.status_msg = if self.show_cell_bounds {
                    "Limites internos das células da grade visíveis.".to_string()
                } else {
                    "Limites das células da grade ocultos.".to_string()
                };
            }
            Message::ToggleShowTerminals => {
                self.show_terminals = !self.show_terminals;
                self.status_msg = if self.show_terminals {
                    "Indicadores de bornes/terminais visíveis.".to_string()
                } else {
                    "Indicadores de bornes/terminais ocultos.".to_string()
                };
            }
            Message::ToggleShowCajetin => {
                self.show_cajetin = !self.show_cajetin;
                self.status_msg = if self.show_cajetin {
                    "Cajetín (legenda da folha) visível.".to_string()
                } else {
                    "Cajetín (legenda da folha) oculto.".to_string()
                };
            }
            Message::SetSheetBgPreset(preset) => {
                self.push_undo();
                self.doc.settings.sheet_bg_color = preset;
                self.sync_active_page();
                self.is_modified = true;
                self.status_msg = match preset {
                    None => "Cor de fundo da folha restaurada para o padrão do tema.".to_string(),
                    Some(rgb) => format!("Cor de fundo da folha alterada para #{:06X}.", rgb),
                };
            }
            Message::OpenSettingsModal => {
                self.show_settings_modal = true;
                self.open_menu = None;
                self.open_submenu = None;
            }
            Message::CloseSettingsModal => {
                self.show_settings_modal = false;
            }
            Message::OpenShortcutsModal => {
                self.show_shortcuts_modal = true;
                self.open_menu = None;
                self.open_submenu = None;
            }
            Message::CloseShortcutsModal => {
                self.show_shortcuts_modal = false;
            }
            Message::ToggleFocusCanvas => {
                self.focus_canvas = !self.focus_canvas;
                self.open_menu = None;
                self.open_submenu = None;
                self.status_msg = if self.focus_canvas {
                    "Modo foco do fólio ativado (Esc para restaurar painéis).".to_string()
                } else {
                    "Painéis laterais restaurados.".to_string()
                };
            }
            Message::ExitApplication => {
                return iced::exit();
            }
            Message::SetTextVisibility(vis) => {
                self.show_texts = matches!(vis, TextVisibility::On);
                self.status_msg = if self.show_texts {
                    "Visibilidade de textos ativada.".to_string()
                } else {
                    "Visibilidade de textos desativada.".to_string()
                };
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
        let workspace_body: Element<'_, Message> = if self.focus_canvas {
            iced::widget::container(crate::ui::view_sheet_panel(self))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(crate::theme::workspace)
                .into()
        } else {
            self.layout.view(
                self.left_dock_collapsed,
                self.right_dock_collapsed,
                Message::LayoutMsg,
                |kind| match kind {
                    Kind::Collections => crate::ui::view_collections_panel(self),
                    Kind::Sheet => crate::ui::view_sheet_panel(self),
                    Kind::Properties => crate::ui::view_properties_panel(self),
                },
            )
        };

        let main_ui = column![
            crate::ui::view_menubar(self),
            crate::ui::view_toolbar(self),
            workspace_body,
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

        // Modal de Impressão Vetorial
        if self.show_print_modal {
            layers.push(crate::ui::view_print_modal(self));
        }

        // Modal de Propriedades da Página
        if self.show_page_properties_modal {
            layers.push(crate::ui::view_page_properties_modal(self));
        }

        // Modal de Propriedades do Projeto
        if self.show_project_properties_modal {
            layers.push(crate::ui::view_project_properties_modal(self));
        }

        // Modal de Configurações Gerais
        if self.show_settings_modal {
            layers.push(crate::ui::view_settings_modal(self));
        }

        // Modal de Atalhos de Teclado
        if self.show_shortcuts_modal {
            layers.push(crate::ui::view_shortcuts_modal(self));
        }

        // Modal de Seleção de Cor do Elemento
        if self.show_color_modal {
            layers.push(crate::ui::view_color_modal(self));
        }

        // Modal de Confirmação de Exclusão de Projeto Recente
        if self.show_confirm_delete_modal {
            layers.push(crate::ui::view_confirm_delete_project_modal(self));
        }

        // Modal de Aviso de Fechamento ao Excluir a Última Folha
        if self.show_close_last_page_modal {
            layers.push(crate::ui::view_close_last_page_modal(self));
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
