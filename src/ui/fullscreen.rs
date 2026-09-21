//! The full-screen player: the cover, the lyrics, and controls that hide
//! with the pointer. One view replaces the former full-screen lyrics mode;
//! the words are one of the things it can show.

use std::time::Duration;

use egui::{Align, Color32, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::app::{App, FULLSCREEN_CONTROLS_IDLE};
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, Loadable};
use crate::settings::FullscreenLyricsLayout;
use crate::theme::{self, Icon};

use super::widgets;
use super::{lyrics, player_bar};

/// How long the controls take to appear or dissolve, matching a lyric line.
const CONTROLS_FADE: f32 = 0.22;
/// Room left around the content so it never touches the window edges.
const CONTENT_MARGIN: f32 = 48.0;
/// Below this width the split layout has no room for two columns.
const SPLIT_MIN_WIDTH: f32 = 700.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, window: Rect) {
    let palette = app.palette;
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(palette.window))
        .show(ui, |ui| {
            // The panel has already given the queue its right-hand strip, so
            // this rect is whatever the view itself owns.
            let rect = ui.max_rect();
            background(app, ui, rect);
            let top = theme::titlebar_inset(ui.ctx()) + 24.0;
            let content = Rect::from_min_max(
                pos2(rect.left() + CONTENT_MARGIN, rect.top() + top),
                pos2(
                    (rect.right() - CONTENT_MARGIN).max(rect.left() + CONTENT_MARGIN),
                    rect.bottom() - theme::PLAYER_BAR_HEIGHT - 24.0,
                ),
            );
            if !app.fullscreen_lyrics {
                let mut column = ui.new_child(UiBuilder::new().max_rect(content));
                cover_content(app, &mut column, content);
            } else if app.settings.fullscreen_lyrics_layout == FullscreenLyricsLayout::Split
                && content.width() >= SPLIT_MIN_WIDTH
            {
                let left_width = (content.width() * 0.42).clamp(240.0, 520.0);
                let left = Rect::from_min_max(
                    content.min,
                    pos2(content.left() + left_width, content.bottom()),
                );
                let right =
                    Rect::from_min_max(pos2(left.right() + 40.0, content.top()), content.max);
                let mut column = ui.new_child(UiBuilder::new().max_rect(left));
                cover_content(app, &mut column, left);
                let mut column = ui.new_child(UiBuilder::new().max_rect(right));
                lyrics_column(app, &mut column, false);
            } else {
                let width = fullscreen_content_width(content.width());
                let region = Rect::from_min_max(
                    pos2(content.center().x - width / 2.0, content.top()),
                    pos2(content.center().x + width / 2.0, content.bottom()),
                );
                let mut column = ui.new_child(UiBuilder::new().max_rect(region));
                lyrics_column(app, &mut column, true);
            }
        });
    controls(app, ui.ctx(), window);
}

/// The playing cover with its title and artist, centred in `region`. Shared
/// by the cover-only view and the left half of the split layout.
fn cover_content(app: &mut App, ui: &mut egui::Ui, region: Rect) {
    let palette = app.palette;
    let Some(now) = app.now_playing() else {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Music,
            &gettext(app.locale, "Nothing playing"),
            &gettext(app.locale, "Pick a song, album, or playlist"),
        );
        return;
    };
    let wrap = region.width().min(560.0);
    let title = ui
        .painter()
        .layout(now.title.clone(), theme::semibold(34.0), palette.text, wrap);
    let artist_text = if now.artists.is_empty() {
        now.subtitle.clone()
    } else {
        now.artists
            .iter()
            .map(|artist| artist.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let artist = ui
        .painter()
        .layout(artist_text, theme::regular(18.0), palette.secondary, wrap);
    let cover_side = ((region.height() - title.size().y - artist.size().y - 44.0)
        .min(region.width() * 0.9))
    .clamp(120.0, 560.0);
    let block = cover_side + 28.0 + title.size().y + 6.0 + artist.size().y;
    let top = region.top() + ((region.height() - block) / 2.0).max(0.0);
    let cover_rect = Rect::from_center_size(
        pos2(region.center().x, top + cover_side / 2.0),
        vec2(cover_side, cover_side),
    );
    widgets::paint_cover(
        ui,
        &palette,
        now.art_url.as_deref().or(now.art_small.as_deref()),
        cover_rect,
        14.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let text_top = top + cover_side + 28.0;
    let title_size = title.size();
    let artist_size = artist.size();
    ui.painter().galley(
        pos2(region.center().x - title_size.x / 2.0, text_top),
        title,
        palette.text,
    );
    ui.painter().galley(
        pos2(
            region.center().x - artist_size.x / 2.0,
            text_top + title_size.y + 6.0,
        ),
        artist,
        palette.secondary,
    );
}

/// The words: a header with their controls, optionally the track heading,
/// then the scrolling lines.
fn lyrics_column(app: &mut App, ui: &mut egui::Ui, heading: bool) {
    lyrics_header(app, ui);
    ui.add_space(20.0);
    if heading {
        track_heading(app, ui);
        ui.add_space(16.0);
    }
    lyrics_contents(app, ui);
}

fn lyrics_header(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        theme::text(
            ui,
            gettext(app.locale, "Lyrics"),
            theme::bold(18.0),
            palette.text,
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let loaded = matches!(&app.lyrics, Loadable::Loaded(Some(_)));
            if loaded
                && !app.lyrics_following
                && theme::pill_button(
                    ui,
                    &palette,
                    &pgettext(app.locale, "lyrics", "Follow"),
                    false,
                )
                .clicked()
            {
                app.actions.push(Action::FollowLyrics);
            }
        });
    });
}

fn track_heading(app: &App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let Some(now) = app.now_playing() else {
        return;
    };
    ui.horizontal(|ui| {
        let size = 52.0;
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        widgets::paint_cover(
            ui,
            &palette,
            now.art_small.as_deref().or(now.art_url.as_deref()),
            rect,
            4.0,
            Icon::Music,
            Some(app.backend.art()),
        );
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&now.title)
                        .font(theme::semibold(22.0))
                        .color(palette.text),
                )
                .truncate(),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&now.subtitle)
                        .font(theme::regular(13.0))
                        .color(palette.secondary),
                )
                .truncate(),
            );
        });
    });
}

fn fullscreen_content_width(viewport_width: f32) -> f32 {
    let available = (viewport_width - 48.0).max(0.0);
    (viewport_width * 0.72).clamp(400.0, 960.0).min(available)
}

fn preferred_backdrop_art(small: Option<String>, large: Option<String>) -> Option<String> {
    small.or(large)
}

/// The blurred playing cover behind everything, veiled toward the theme's
/// own tone so words keep their contrast in either theme.
fn background(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let palette = app.palette;
    let art = app
        .now_playing()
        .and_then(|now| preferred_backdrop_art(now.art_small, now.art_url));
    let painter = ui.painter().with_clip_rect(rect);
    if let Some(texture) = app
        .lyrics_backdrop
        .texture(ui.ctx(), app.backend.art(), art.as_deref())
    {
        // Dark rests on a dimmed picture, light on a milky one.
        let tone = if palette.dark {
            Color32::from_gray(180)
        } else {
            Color32::from_gray(232)
        };
        painter.image(
            texture.id(),
            rect,
            cover_uv(rect.size(), texture.size_vec2()),
            tone,
        );
    }
    let veil_alpha = if palette.dark { 130 } else { 178 };
    let veil = |alpha: u8| {
        Color32::from_rgba_unmultiplied(
            palette.window.r(),
            palette.window.g(),
            palette.window.b(),
            alpha,
        )
    };
    painter.rect_filled(rect, 0.0, veil(veil_alpha));
    // Extra depth under where the controls surface.
    let bottom = Rect::from_min_size(
        pos2(rect.left(), rect.bottom() - theme::PLAYER_BAR_HEIGHT * 2.0),
        vec2(rect.width(), theme::PLAYER_BAR_HEIGHT * 2.0),
    );
    widgets::paint_vertical_gradient(ui, bottom, veil(0), veil(veil_alpha));
}

fn cover_uv(view: egui::Vec2, image: egui::Vec2) -> Rect {
    let ratio = (view.x / view.y.max(1.0)) / (image.x / image.y.max(1.0));
    let size = if ratio > 1.0 {
        vec2(1.0, 1.0 / ratio)
    } else {
        vec2(ratio, 1.0)
    };
    Rect::from_center_size(pos2(0.5, 0.5), size)
}

/// The scrolling lyric lines, following the song. The wheel and a dragged
/// pointer take over from following; the Follow button picks the song up.
fn lyrics_contents(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let Some(now) = app.now_playing() else {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Mic,
            &gettext(app.locale, "Nothing playing"),
            &gettext(app.locale, "Play a song to see its lyrics."),
        );
        return;
    };
    let lyrics = match &app.lyrics {
        Loadable::NotLoaded | Loadable::Loading => {
            widgets::loading_row(ui, &palette, app.locale);
            return;
        }
        Loadable::Failed(error) => {
            // Translators: Keep {error} unchanged. It is the original failure detail.
            let message =
                gettext(app.locale, "Couldn't fetch the lyrics: {error}").replace("{error}", error);
            ui.add_space(8.0);
            theme::text(ui, message, theme::regular(13.0), palette.text);
            ui.add_space(8.0);
            if theme::pill_button(ui, &palette, &gettext(app.locale, "Try again"), false).clicked()
            {
                app.actions.push(Action::RetryLyrics);
            }
            return;
        }
        Loadable::Loaded(None) => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Mic,
                &gettext(app.locale, "No lyrics"),
                &gettext(app.locale, "No lyrics found for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) if lyrics.instrumental => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Music,
                &gettext(app.locale, "Instrumental"),
                &gettext(app.locale, "No timed lyrics for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) => lyrics.clone(),
    };

    let active = lyrics.active_line(now.position_ms);
    // A Follow click resets the remembered line after drawing. Other frames
    // record the shown line before any line-click action restores following.
    if !app
        .actions
        .iter()
        .any(|action| matches!(action, Action::FollowLyrics))
    {
        app.actions.push(Action::LyricsLineShown(active));
    }
    let viewport = ui.available_rect_before_wrap();
    let manual_scroll = ui.rect_contains_pointer(viewport)
        && ui.input(|input| {
            input.smooth_scroll_delta.y != 0.0
                || (input.pointer.primary_down() && input.pointer.delta().y != 0.0)
        });
    let following = app.lyrics_following && !manual_scroll;
    let follow = following && app.lyrics_line_shown != Some(active);
    let animation = egui::style::ScrollAnimation::duration(0.45);
    let size = (ui.available_width() * 0.046).clamp(28.0, 42.0);
    // The line being sung brightens; all lines keep the same font metrics
    // so highlighting cannot rewrap the words during a transition.
    // A line takes 300 ms to light up or fade.
    let quiet = palette
        .text
        .gamma_multiply(if palette.dark { 0.68 } else { 0.55 });
    ui.spacing_mut().scroll.fade.strength = 0.0;
    egui::ScrollArea::vertical()
        .id_salt(("fullscreen-lyrics-scroll", &now.uri))
        .auto_shrink([false, false])
        // The words own the screen: the wheel and a dragged pointer scroll,
        // the bar itself has nothing to say here.
        .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            // Before the first line there is nothing to highlight, so the
            // panel sits at the top rather than wherever it was left.
            if follow && lyrics.synced && active.is_none() {
                let top = ui.cursor().min;
                ui.scroll_to_rect_animation(
                    egui::Rect::from_min_size(top, egui::vec2(1.0, 1.0)),
                    Some(Align::Min),
                    animation,
                );
            }
            let padding = if lyrics.synced {
                (viewport.height() * 0.5 - size).max(12.0)
            } else {
                12.0
            };
            ui.add_space(padding);
            for (index, line) in lyrics.lines.iter().enumerate() {
                let is_active = active == Some(index);
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new("lyric-line").with(("fullscreen", &now.uri, index)),
                    is_active,
                    0.3,
                );
                let color = if lyrics.synced {
                    lyrics::blend(quiet, palette.text, lit)
                } else {
                    palette.text
                };
                let font = theme::bold(size);
                // A timed line with no words is the band playing on.
                let text = if line.text.is_empty() && lyrics.synced {
                    "\u{266a}"
                } else {
                    line.text.as_str()
                };
                let sense = if lyrics.synced {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let galley = crate::bidi::layout(
                    ui.painter(),
                    text,
                    font,
                    color,
                    ui.available_width(),
                    usize::MAX,
                    None,
                );
                let center = ui.cursor().top() + galley.size().y * 0.5;
                let edge = ((center - viewport.top()).min(viewport.bottom() - center)
                    / (size * 2.0))
                    .clamp(0.0, 1.0);
                let response = ui
                    .scope(|ui| {
                        ui.multiply_opacity(edge * edge * (3.0 - 2.0 * edge));
                        ui.add(egui::Label::new(galley).sense(sense))
                    })
                    .inner;
                let rect = response.rect;
                if lyrics.synced {
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.clicked()
                        && let Some(at_ms) = line.at_ms
                    {
                        app.actions.push(Action::Seek(at_ms));
                        app.actions.push(Action::FollowLyrics);
                    }
                }
                if is_active && follow {
                    ui.scroll_to_rect_animation(rect, Some(Align::Center), animation);
                }
                ui.add_space(27.0);
            }
            // Words without timing can only be followed by the clock: sit
            // at the part of the text the song is probably at.
            if following && !lyrics.synced && now.duration_ms > 0 {
                let fraction =
                    (f64::from(now.position_ms) / f64::from(now.duration_ms)).clamp(0.0, 1.0);
                let content = ui.min_rect();
                let y = content.top() + content.height() * fraction as f32;
                ui.scroll_to_rect_animation(
                    egui::Rect::from_min_max(
                        egui::pos2(content.left(), y),
                        egui::pos2(content.right(), y + 1.0),
                    ),
                    Some(Align::Center),
                    animation,
                );
            }
            // The controls surface from below; keep the last lines above them.
            ui.add_space(padding.max(60.0) + theme::PLAYER_BAR_HEIGHT);
        });
    // Scrolling by hand means the reader wants to look elsewhere; the
    // Follow button in the header picks the song back up.
    if manual_scroll && app.lyrics_following {
        app.actions.push(Action::PauseLyricsFollow);
    }
    if now.playing
        && lyrics.synced
        && let Some(next) = lyrics
            .lines
            .iter()
            .filter_map(|line| line.at_ms)
            .find(|at| *at > now.position_ms)
    {
        ui.ctx()
            .request_repaint_after(Duration::from_millis(u64::from(next - now.position_ms)));
    }
}

/// The player controls, an overlay that surfaces with the pointer and a key
/// press and dissolves again with the pointer into rest. A dialog holds
/// them up: a hidden bar cannot be behind a window nobody can dismiss.
fn controls(app: &mut App, ctx: &egui::Context, viewport: Rect) {
    let palette = app.palette;
    let idle_for = app.fullscreen_activity.elapsed();
    let awake = idle_for < FULLSCREEN_CONTROLS_IDLE || app.dialog.is_some();
    let alpha =
        ctx.animate_bool_with_time(egui::Id::new("fullscreen-controls"), awake, CONTROLS_FADE);
    if awake {
        ctx.request_repaint_after(FULLSCREEN_CONTROLS_IDLE.saturating_sub(idle_for));
    }
    if (alpha - f32::from(awake)).abs() > 0.001 {
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    if alpha < 0.01 {
        if app.dialog.is_none() {
            // Rest means no chrome and no pointer.
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }
        return;
    }
    // The queue panel keeps the right edge; the controls stop at its side.
    let queue = if app.show_queue_panel {
        app.settings.queue_width
    } else {
        0.0
    };
    let bar = Rect::from_min_max(
        pos2(
            viewport.left(),
            viewport.bottom() - theme::PLAYER_BAR_HEIGHT,
        ),
        pos2(viewport.right() - queue, viewport.bottom()),
    );
    egui::Area::new(egui::Id::new("fullscreen-controls"))
        .order(egui::Order::Foreground)
        .interactable(awake)
        .anchor(egui::Align2::LEFT_BOTTOM, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
            // The bar is placed by hand, like the transport inside it: an
            // explicit rect instead of a frame, so the fading panel is the
            // same panel, not a relayout of one.
            ui.painter().rect_filled(bar, 0.0, palette.panel);
            ui.painter().hline(
                bar.x_range(),
                bar.top() + 0.5,
                egui::Stroke::new(1.0, palette.outline),
            );
            let side = (bar.width() * 0.3).clamp(200.0, 420.0);
            let center = Rect::from_min_max(
                pos2(bar.left() + side, bar.top()),
                pos2(bar.right() - side, bar.bottom()),
            );
            let now = app.now_playing();
            player_bar::transport(app, ui, now.as_ref(), center);
            let right_band = Rect::from_min_size(
                pos2(bar.right() - side, bar.center().y - 15.0),
                vec2(side, 30.0),
            );
            let mut right_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(right_band)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            extras(app, &mut right_ui, now.as_ref());
        });
}

fn extras(app: &mut App, ui: &mut egui::Ui, now: Option<&crate::app::NowPlaying>) {
    let palette = app.palette;
    ui.spacing_mut().item_spacing.x = 6.0;
    if theme::icon_button(
        ui,
        Icon::Shrink,
        18.0,
        palette.text,
        palette.text,
        &gettext(app.locale, "Leave full screen (Esc)"),
    )
    .clicked()
    {
        app.actions.push(Action::SetPlayerFullscreen(false));
    }
    if theme::icon_button(
        ui,
        Icon::ListVideo,
        18.0,
        if app.show_queue_panel {
            palette.accent
        } else {
            palette.secondary
        },
        palette.text,
        &gettext(app.locale, "Queue"),
    )
    .clicked()
    {
        app.actions.push(Action::ToggleQueuePanel);
    }
    if theme::icon_button(
        ui,
        Icon::Mic,
        18.0,
        if app.fullscreen_lyrics {
            palette.accent
        } else {
            palette.secondary
        },
        palette.text,
        &gettext(app.locale, "Lyrics"),
    )
    .clicked()
    {
        app.actions.push(Action::ToggleFullscreenLyrics);
    }
    if let Some(now) = now.filter(|now| !now.is_episode) {
        let saved = app.is_saved(&now.uri).unwrap_or(false);
        let (icon, color, tooltip) = if saved {
            (
                Icon::HeartFilled,
                palette.accent,
                gettext(app.locale, "Remove from Liked Songs"),
            )
        } else {
            (
                Icon::Heart,
                palette.secondary,
                gettext(app.locale, "Save to Liked Songs"),
            )
        };
        if theme::icon_button(ui, icon, 17.0, color, palette.text, &tooltip).clicked() {
            app.actions.push(Action::ToggleSaved(now.uri.clone()));
        }
    }
    player_bar::volume_control(app, ui, now);
}

#[cfg(test)]
mod tests {
    use super::{fullscreen_content_width, preferred_backdrop_art};

    #[test]
    fn fullscreen_backdrop_prefers_small_art_with_large_art_as_fallback() {
        let small = "small".to_string();
        let large = "large".to_string();
        assert_eq!(
            preferred_backdrop_art(Some(small.clone()), Some(large.clone())),
            Some(small)
        );
        assert_eq!(
            preferred_backdrop_art(None, Some(large.clone())),
            Some(large)
        );
        assert_eq!(preferred_backdrop_art(None, None), None);
    }

    #[test]
    fn fullscreen_content_width_never_inverts_a_narrow_viewport() {
        for viewport_width in [0.0, 24.0, 47.0, 48.0, 64.0, 760.0, 2_000.0] {
            let width = fullscreen_content_width(viewport_width);
            assert!(width >= 0.0);
            assert!(width <= (viewport_width - 48.0).max(0.0));
        }
        assert_eq!(fullscreen_content_width(47.0), 0.0);
        assert_eq!(fullscreen_content_width(2_000.0), 960.0);
    }
}
