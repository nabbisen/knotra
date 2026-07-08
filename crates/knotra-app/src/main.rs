//! knotra — multi-project VCS dashboard

mod app;
mod config;
mod fs_watcher;
mod message;
mod persistence;
mod state;
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
        .run()
}
