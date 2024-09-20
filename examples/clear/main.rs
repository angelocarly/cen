use kiyo::app::app::{App, AppConfig};
use kiyo::app::draw_orch::{ClearConfig, DrawConfig, ImageConfig};

fn main() {

    let app = App::new(AppConfig {
        width: 1000,
        height: 1000,
        vsync: true,
        log_fps: false,
    });

    // Display a single image cleared to yellow
    let config = DrawConfig {
        images: Vec::from([
            ImageConfig {
                clear: ClearConfig::Color(1.0, 1.0, 0.0)
            },
        ]),
        passes: Vec::from([
        ])
    };

    app.run(config, None);
}