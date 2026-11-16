//! Native PLC Ladder Logic + FBD simulator (iced 0.13).

use cade_simu_rs::plc::PlcSimuApp;

pub fn main() -> iced::Result {
    iced::application(PlcSimuApp::title, PlcSimuApp::update, PlcSimuApp::view)
        .subscription(PlcSimuApp::subscription)
        .theme(PlcSimuApp::theme)
        .window_size((1280.0, 860.0))
        .run_with(PlcSimuApp::new)
}
