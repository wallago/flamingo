use ratatui::{Frame, layout::Rect};

use crate::tui::state::State;

pub fn render_splash_screen(state: &mut State, frame: &mut Frame) {
    if !state.logo.is_rendered {
        let area = frame.area();
        let (logo_width, logo_height) = state.logo.get_size();
        if logo_width < area.width && logo_height < area.height {
            frame.render_widget(
                &state.logo,
                Rect::new(
                    area.width / 2 - logo_width / 2,
                    area.height / 2 - logo_height / 2,
                    logo_width,
                    logo_height,
                ),
            );
            state.logo.is_rendered = state.logo.init_time.elapsed().as_millis() > 500;
            return;
        }
    }
}
