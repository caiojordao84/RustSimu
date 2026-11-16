//! Native PLC Ladder Logic + FBD simulator (iced 0.13).
//!
//! Ported from the reference web simulator under the Project store
//! (`internal/plc-reference/`). Clean split: model / scan / demo / ui.

pub mod demo;
pub mod model;
pub mod scan;
pub mod theme;
pub mod ui;

pub use model::{EditorMode, PlcProgram, SimMode, SimState};
pub use ui::PlcSimuApp;
