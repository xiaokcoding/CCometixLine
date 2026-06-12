use crate::config::{Config, SegmentConfig, StyleMode};
use crate::core::render::palette;
use crate::core::render::phase::RenderPhase;
use crate::core::render::state::{Fragment, RenderState};
use crate::core::segments::SegmentData;

/// Turns each segment into a colored fragment.
pub struct CompositionPhase;

impl RenderPhase for CompositionPhase {
    fn apply(&self, state: &mut RenderState) {
        for (config, data) in &state.segments {
            let body = render_segment(&state.config, config, data);
            if !body.is_empty() {
                state.fragments.push(Fragment {
                    body,
                    background: config.colors.background.clone(),
                });
            }
        }
    }
}

/// Render a single segment: icon, primary and secondary text, each in its
/// configured colors, padded when a background color frames the whole.
pub fn render_segment(style: &Config, config: &SegmentConfig, data: &SegmentData) -> String {
    let icon = data
        .metadata
        .get("dynamic_icon")
        .cloned()
        .unwrap_or_else(|| icon_for(style, config));

    if let Some(bg_color) = &config.colors.background {
        let bg_code = palette::background(bg_color);

        let icon_colored = match &config.colors.icon {
            Some(icon_color) => palette::tinted(&icon, Some(icon_color)).replace("\x1b[0m", ""),
            None => icon.clone(),
        };

        let text_styled = palette::styled(
            &data.primary,
            config.colors.text.as_ref(),
            config.styles.text_bold,
        )
        .replace("\x1b[0m", "");

        let mut content = format!(" {} {} ", icon_colored, text_styled);

        if !data.secondary.is_empty() {
            let secondary_styled = palette::styled(
                &data.secondary,
                config.colors.text.as_ref(),
                config.styles.text_bold,
            )
            .replace("\x1b[0m", "");
            content.push_str(&format!("{} ", secondary_styled));
        }

        format!("{}{}\x1b[49m", bg_code, content)
    } else {
        let icon_colored = palette::tinted(&icon, config.colors.icon.as_ref());
        let text_styled = palette::styled(
            &data.primary,
            config.colors.text.as_ref(),
            config.styles.text_bold,
        );

        let mut body = format!("{} {}", icon_colored, text_styled);

        if !data.secondary.is_empty() {
            body.push_str(&format!(
                " {}",
                palette::styled(
                    &data.secondary,
                    config.colors.text.as_ref(),
                    config.styles.text_bold
                )
            ));
        }

        body
    }
}

fn icon_for(style: &Config, config: &SegmentConfig) -> String {
    match style.style.mode {
        StyleMode::Plain => config.icon.plain.clone(),
        StyleMode::NerdFont | StyleMode::Powerline => config.icon.nerd_font.clone(),
    }
}
