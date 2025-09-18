use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub created_at: Instant,
    pub duration: Duration,
    pub notification_type: NotificationType,
}

#[derive(Debug, Clone)]
pub enum NotificationType {
    Error,
    Warning,
    Info,
    Success,
}

impl Notification {
    pub fn new_error(message: String) -> Self {
        Self {
            message,
            created_at: Instant::now(),
            duration: Duration::from_secs(1), // Auto-disappear after 1 second
            notification_type: NotificationType::Error,
        }
    }

    pub fn new_warning(message: String) -> Self {
        Self {
            message,
            created_at: Instant::now(),
            duration: Duration::from_secs(1),
            notification_type: NotificationType::Warning,
        }
    }

    pub fn new_info(message: String) -> Self {
        Self {
            message,
            created_at: Instant::now(),
            duration: Duration::from_secs(1),
            notification_type: NotificationType::Info,
        }
    }

    pub fn new_success(message: String) -> Self {
        Self {
            message,
            created_at: Instant::now(),
            duration: Duration::from_secs(1),
            notification_type: NotificationType::Success,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.duration
    }
}

pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn add_error(&mut self, message: String) {
        self.add_notification(Notification::new_error(message));
    }

    pub fn add_warning(&mut self, message: String) {
        self.add_notification(Notification::new_warning(message));
    }

    pub fn add_info(&mut self, message: String) {
        self.add_notification(Notification::new_info(message));
    }

    pub fn add_success(&mut self, message: String) {
        self.add_notification(Notification::new_success(message));
    }

    pub fn update(&mut self) {
        // Remove expired notifications
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn has_notifications(&self) -> bool {
        !self.notifications.is_empty()
    }

    pub fn get_latest(&self) -> Option<&Notification> {
        self.notifications.last()
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn bottom_right_popup_area(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.right().saturating_sub(width + 1); // +1 for border spacing
    let y = area.bottom().saturating_sub(height + 1); // +1 for border spacing

    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

pub fn render_notifications(frame: &mut Frame, notifications: &NotificationManager, area: Rect) {
    if let Some(notification) = notifications.get_latest() {
        let notification_width = 50;
        let notification_height = 3;

        let popup_area = bottom_right_popup_area(area, notification_width, notification_height);

        // Clear the background first
        frame.render_widget(Clear, popup_area);

        // Style based on notification type
        let (border_color, text_color, title) = match notification.notification_type {
            NotificationType::Error => (Color::Red, Color::White, "Error"),
            NotificationType::Warning => (Color::Yellow, Color::Black, "Warning"),
            NotificationType::Info => (Color::Blue, Color::White, "Info"),
            NotificationType::Success => (Color::Green, Color::White, "Success"),
        };

        // Create the notification widget
        let notification_widget = Paragraph::new(notification.message.as_str())
            .style(Style::default().fg(text_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(title)
                    .title_style(Style::default().fg(border_color)),
            );

        frame.render_widget(notification_widget, popup_area);
    }
}
