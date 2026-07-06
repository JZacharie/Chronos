pub fn send(summary: &str, body: &str) {
    if let Ok(notif) = notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .appname("Chronos")
        .icon("chronos")
        .show()
    {
        let _ = notif;
    }
}
