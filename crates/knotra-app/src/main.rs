//! knotra — multi-project VCS dashboard

mod app;
mod atomic_write;
#[cfg(test)]
mod button_label_guard;
mod config;
mod fs_watcher;
mod message;
mod persistence;
mod state;
#[cfg(test)]
mod suppressions_guard;
#[cfg(test)]
mod text_outside_catalog_guard;
#[cfg(test)]
mod text_size_guard;
#[cfg(test)]
mod text_weight_guard;
mod view;

#[cfg(test)]
mod tests;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("knotra=info")),
        )
        .init();

    tracing::info!("knotra v0.2.0 starting");

    iced::application(app::init, app::update, app::view)
        .title(|_: &state::AppState| String::from("knotra"))
        .subscription(app::subscription)
        .theme(|state: &state::AppState| state.theme.base.clone())
        // RFC-071 D11: zoom. `AppConfig.zoom_percent` is always one of
        // `config::ZOOM_STEPS` once loaded (`load_config`'s own
        // `nearest_zoom_step` snap), so no further validation is needed
        // here — this reads live every render, so a change takes effect
        // without a restart.
        .scale_factor(|state: &state::AppState| f32::from(state.config.zoom_percent) / 100.0)
        .font(knotra_ui::widget::icon::FONT_BYTES)
        .window(iced::window::Settings {
            size: state::INITIAL_WINDOW_SIZE,
            min_size: Some(iced::Size::new(800.0, 600.0)),
            ..iced::window::Settings::default()
        })
        .run()
}
