use application::ports::{Notification, NotificationService};

/// Best-effort native desktop notifications.
///
/// Notification failures are deliberately non-fatal: game state has already
/// been persisted by the time this adapter is called.
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopNotifications;

impl NotificationService for DesktopNotifications {
    fn notify(&self, notification: Notification) {
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        if let Err(error) = notify_rust::Notification::new()
            .summary(&notification.title)
            .body(&notification.body)
            .show()
        {
            tracing::warn!(%error, "desktop notification could not be delivered");
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        tracing::info!(
            title = %notification.title,
            body = %notification.body,
            "desktop notifications are unavailable on this platform"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implements_notification_service() {
        fn accepts_service(_: &impl NotificationService) {}
        accepts_service(&DesktopNotifications);
    }
}
