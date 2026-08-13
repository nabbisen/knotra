//! knotra — multi-project VCS dashboard

mod app;
mod atomic_write;
mod config;
mod fs_watcher;
mod message;
mod persistence;
mod state;
#[cfg(test)]
mod suppressions_guard;
#[cfg(test)]
mod text_outside_catalog_guard;
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
        .font(knotra_ui::widget::icon::FONT_BYTES)
        .window(iced::window::Settings {
            size: state::INITIAL_WINDOW_SIZE,
            min_size: Some(iced::Size::new(800.0, 600.0)),
            ..iced::window::Settings::default()
        })
        .run()
}
