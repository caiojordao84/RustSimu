pub mod canvas_area;
pub mod library;
pub mod menu;
pub mod modals;
pub mod properties;
pub mod statusbar;
pub mod toolbar;
pub mod widgets;

pub use canvas_area::{view_canvas_pages_bar, view_canvas_title_line, view_sheet_panel};
pub use library::{view_collections_panel, view_unified_library_panel};
pub use menu::{view_dropdown_menu, view_menubar, ActiveMenu};
pub use modals::{
    view_about_modal, view_close_last_page_modal, view_color_modal,
    view_confirm_delete_project_modal, view_export_modal, view_page_properties_modal,
    view_print_modal, view_project_properties_modal, view_settings_modal, view_shortcuts_modal,
    view_text_editor_modal,
};
pub use properties::view_properties_panel;
pub use statusbar::view_statusbar;
pub use toolbar::view_toolbar;
