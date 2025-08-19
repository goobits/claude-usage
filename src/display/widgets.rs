//! Custom TUI Widgets for Live Display
//!
//! This module provides custom widgets for the live monitoring interface,
//! including styled blocks, activity lists, and session information displays.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use super::{LiveDisplay, SessionActivity};

/// Style constants for consistent theming
pub struct AppTheme {
    pub primary: Style,
    pub secondary: Style,
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub muted: Style,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            primary: Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            secondary: Style::default().fg(Color::Cyan),
            accent: Style::default().fg(Color::Yellow),
            success: Style::default().fg(Color::Green),
            warning: Style::default().fg(Color::Yellow),
            error: Style::default().fg(Color::Red),
            muted: Style::default().fg(Color::DarkGray),
        }
    }
}

/// Custom widget for displaying the main header with totals
pub struct HeaderWidget<'a> {
    totals_text: &'a str,
    theme: &'a AppTheme,
}

impl<'a> HeaderWidget<'a> {
    pub fn new(totals_text: &'a str, theme: &'a AppTheme) -> Self {
        Self { totals_text, theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let header_block = Block::default()
            .title("Claude Usage Live")
            .title_style(self.theme.primary)
            .borders(Borders::ALL)
            .border_style(self.theme.secondary);

        let header_text = Paragraph::new(self.totals_text)
            .style(self.theme.success)
            .alignment(Alignment::Center)
            .block(header_block);

        frame.render_widget(header_text, area);
    }
}

/// Custom widget for displaying current session information
pub struct SessionWidget<'a> {
    session_info: Option<&'a str>,
    theme: &'a AppTheme,
}

impl<'a> SessionWidget<'a> {
    pub fn new(session_info: Option<&'a str>, theme: &'a AppTheme) -> Self {
        Self { session_info, theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let session_block = Block::default()
            .title("Current Session")
            .title_style(self.theme.primary)
            .borders(Borders::ALL)
            .border_style(self.theme.secondary);

        let session_text = if let Some(info) = self.session_info {
            Text::from(vec![
                Line::from(vec![
                    Span::styled("├─ ", self.theme.muted),
                    Span::styled(info, self.theme.accent),
                ]),
            ])
        } else {
            Text::from(vec![
                Line::from(vec![
                    Span::styled("├─ ", self.theme.muted),
                    Span::styled("No active session", self.theme.muted),
                ]),
            ])
        };

        let session_paragraph = Paragraph::new(session_text)
            .block(session_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(session_paragraph, area);
    }
}

/// Custom widget for displaying recent activity with scrolling
pub struct ActivityWidget<'a> {
    activities: Vec<&'a SessionActivity>,
    scroll_indicator: &'a str,
    theme: &'a AppTheme,
    can_scroll: bool,
}

impl<'a> ActivityWidget<'a> {
    pub fn new(
        activities: Vec<&'a SessionActivity>,
        scroll_indicator: &'a str,
        theme: &'a AppTheme,
        can_scroll: bool,
    ) -> Self {
        Self {
            activities,
            scroll_indicator,
            theme,
            can_scroll,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let title = if self.can_scroll {
            format!("Recent Activity (↑/↓ to scroll){}", self.scroll_indicator)
        } else {
            "Recent Activity".to_string()
        };

        let activity_block = Block::default()
            .title(title)
            .title_style(self.theme.primary)
            .borders(Borders::ALL)
            .border_style(self.theme.secondary);

        if self.activities.is_empty() {
            let empty_text = Paragraph::new("No recent activity")
                .style(self.theme.muted)
                .alignment(Alignment::Center)
                .block(activity_block);

            frame.render_widget(empty_text, area);
            return;
        }

        let items: Vec<ListItem> = self.activities
            .iter()
            .map(|activity| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", activity.time_str),
                        self.theme.muted,
                    ),
                    Span::styled(
                        format!("{}: ", activity.project),
                        self.theme.secondary,
                    ),
                    Span::styled(
                        format!("+{} tokens ", activity.tokens),
                        self.theme.accent,
                    ),
                    Span::styled(
                        format!("(${:.3})", activity.cost),
                        self.theme.success,
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let activity_list = List::new(items)
            .block(activity_block)
            .style(self.theme.primary);

        frame.render_widget(activity_list, area);
    }
}

/// Custom widget for displaying help/status information
pub struct StatusWidget<'a> {
    theme: &'a AppTheme,
}

impl<'a> StatusWidget<'a> {
    pub fn new(theme: &'a AppTheme) -> Self {
        Self { theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            Span::styled("Press ", self.theme.muted),
            Span::styled("Ctrl+C", self.theme.accent),
            Span::styled(" to exit", self.theme.muted),
        ]);

        let help_paragraph = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .style(self.theme.muted);

        frame.render_widget(help_paragraph, area);
    }
}

/// Error overlay widget for displaying connection issues
pub struct ErrorOverlayWidget<'a> {
    error_message: &'a str,
    theme: &'a AppTheme,
}

impl<'a> ErrorOverlayWidget<'a> {
    pub fn new(error_message: &'a str, theme: &'a AppTheme) -> Self {
        Self { error_message, theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Create a centered popup area
        let popup_area = centered_rect(60, 20, area);

        // Clear the background
        frame.render_widget(Clear, popup_area);

        // Create error block
        let error_block = Block::default()
            .title("Error")
            .title_style(self.theme.error)
            .borders(Borders::ALL)
            .border_style(self.theme.error);

        let error_text = Text::from(vec![
            Line::from(vec![
                Span::styled("Connection Error:", self.theme.error),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(self.error_message, self.theme.primary),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Attempting to reconnect...", self.theme.muted),
            ]),
        ]);

        let error_paragraph = Paragraph::new(error_text)
            .block(error_block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(error_paragraph, popup_area);
    }
}

/// Create a layout for the main display
pub fn create_main_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(5), // Current session
            Constraint::Min(8),    // Recent activity (expandable)
            Constraint::Length(1), // Status line
        ])
        .split(area)
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render the complete live display UI
pub fn render_live_display(
    frame: &mut Frame,
    display: &LiveDisplay,
    area: Rect,
    theme: &AppTheme,
    error_message: Option<&str>,
) {
    let chunks = create_main_layout(area);

    // Header with totals
    let totals_text = display.format_totals();
    let header = HeaderWidget::new(&totals_text, theme);
    header.render(frame, chunks[0]);

    // Current session info
    let session_info = display.format_current_session();
    let session = SessionWidget::new(session_info.as_deref(), theme);
    session.render(frame, chunks[1]);

    // Recent activity list
    let activity_area = chunks[2];
    let available_lines = activity_area.height.saturating_sub(2) as usize; // Account for borders
    let visible_activities = display.get_visible_activities(available_lines);
    let scroll_indicator = display.get_scroll_indicator(available_lines);
    let can_scroll = display.can_scroll(available_lines);

    let activity = ActivityWidget::new(
        visible_activities,
        &scroll_indicator,
        theme,
        can_scroll,
    );
    activity.render(frame, activity_area);

    // Status line
    let status = StatusWidget::new(theme);
    status.render(frame, chunks[3]);

    // Error overlay if there's an error
    if let Some(error) = error_message {
        let error_overlay = ErrorOverlayWidget::new(error, theme);
        error_overlay.render(frame, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, area);
        
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 12);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 25);
    }

    #[test]
    fn test_main_layout_constraints() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = create_main_layout(area);
        
        assert_eq!(layout.len(), 4);
        assert_eq!(layout[0].height, 3); // Header
        assert_eq!(layout[1].height, 5); // Session
        assert_eq!(layout[3].height, 1); // Status
        // Activity area should take remaining space
        assert!(layout[2].height >= 8);
    }
}