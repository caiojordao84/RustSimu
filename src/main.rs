use cade_simu_rs::app::CadeSimuApp;

pub fn main() -> iced::Result {
    iced::application("RustSimu", CadeSimuApp::update, CadeSimuApp::view)
        .subscription(CadeSimuApp::subscription)
        .theme(CadeSimuApp::theme)
        .window_size((1420.0, 880.0))
        .run_with(CadeSimuApp::new)
}
