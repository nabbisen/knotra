//! Integration-level tests for knotra-app.

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::state::{AppState, Screen};
    use crate::message::{Message, FilterMessage};

    fn make_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    #[test]
    fn initial_screen_is_dashboard() {
        let state = make_state();
        assert_eq!(state.screen, Screen::Dashboard);
    }

    #[test]
    fn filter_search_updates_state() {
        let mut state = make_state();
        state.apply_filter(FilterMessage::SearchChanged("api".to_owned()));
        assert_eq!(state.filter.search_text, "api");
    }

    #[test]
    fn filter_toggle_adds_and_removes() {
        use crate::message::StatusFilter;
        let mut state = make_state();

        state.apply_filter(FilterMessage::StatusFilterToggled(StatusFilter::Behind));
        assert_eq!(state.filter.status_filters.len(), 1);

        state.apply_filter(FilterMessage::StatusFilterToggled(StatusFilter::Behind));
        assert_eq!(state.filter.status_filters.len(), 0);
    }

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = AppConfig::default();
        assert!(cfg.max_concurrent_reads > 0);
        assert!(cfg.max_log_entries > 0);
    }
}
