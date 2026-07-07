use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::thread;
#[cfg(target_os = "linux")]
use std::time::Duration;
#[cfg(target_os = "linux")]
use zbus::zvariant::Value;

#[cfg(target_os = "linux")]
const POLL_INTERVAL_SECS: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockEvent {
    Locked,
    Unlocked,
}

#[cfg(target_os = "linux")]
pub fn spawn_lock_listener(tx: mpsc::Sender<LockEvent>) {
    thread::spawn(move || {
        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("Failed to connect to D-Bus system bus: {e}");
                return;
            }
        };

        let pid = std::process::id();
        let session_path = match get_session_path(&conn, pid) {
            Some(p) => p,
            None => {
                tracing::debug!("Could not determine D-Bus session path");
                return;
            }
        };

        let mut was_locked = false;
        loop {
            let is_locked = check_locked(&conn, &session_path);
            if is_locked && !was_locked {
                tracing::info!("Session locked");
                let _ = tx.send(LockEvent::Locked);
            } else if !is_locked && was_locked {
                tracing::info!("Session unlocked");
                let _ = tx.send(LockEvent::Unlocked);
            }
            was_locked = is_locked;
            thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        }
    });
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_lock_listener(_tx: mpsc::Sender<LockEvent>) {
    // Session lock tracking is currently only supported on Linux via D-Bus
}

#[cfg(target_os = "linux")]
fn get_session_path(
    conn: &zbus::blocking::Connection,
    pid: u32,
) -> Option<zbus::zvariant::OwnedObjectPath> {
    let msg = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "GetSessionByPID",
            &(pid,),
        )
        .ok()?;
    let body = msg.body();
    body.deserialize::<zbus::zvariant::OwnedObjectPath>().ok()
}

#[cfg(target_os = "linux")]
fn check_locked(
    conn: &zbus::blocking::Connection,
    session_path: &zbus::zvariant::OwnedObjectPath,
) -> bool {
    let msg = match conn.call_method(
        Some("org.freedesktop.login1"),
        session_path.as_str(),
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.freedesktop.login1.Session", "LockedHint"),
    ) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let body = msg.body();
    let value: Value = match body.deserialize() {
        Ok(v) => v,
        Err(_) => return false,
    };

    match value {
        Value::Value(box_val) => match *box_val {
            Value::Bool(b) => b,
            _ => false,
        },
        Value::Bool(b) => b,
        _ => false,
    }
}
