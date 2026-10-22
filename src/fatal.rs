//! Best-effort fatal diagnostics; independent of native windows and the chooser connection.
use std::{collections::HashMap, io::Write, time::Duration};

use ashpd::zbus::{self, zvariant::Value};

pub fn report(title: Vec<u8>, message: Vec<u8>) {
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(&title);
    let _ = stderr.write_all(b": ");
    let _ = stderr.write_all(&message);
    let _ = stderr.write_all(b"\n");
    drop(stderr);

    let (Ok(title), Ok(message)) = (String::from_utf8(title), String::from_utf8(message)) else {
        return;
    };
    let Ok(runtime) = crate::services::runtime() else {
        return;
    };
    // One deadline covers connecting, capability lookup and submission, not display lifetime.
    let _ = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(2), notify(title, message)).await
    });
}

async fn notify(title: String, message: String) -> zbus::Result<()> {
    let connection = zbus::Connection::session().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await?;
    let capabilities: Vec<String> = proxy.call("GetCapabilities", &()).await?;
    let body = if capabilities.iter().any(|c| c == "body-markup") {
        escape_markup(&message)
    } else {
        message
    };
    let hints = HashMap::from([
        ("urgency", Value::U8(2)),
        ("desktop-entry", Value::from("com.lite_xl.LiteXL")),
    ]);
    let _: u32 = proxy
        .call(
            "Notify",
            &(
                "Lite XL",
                0u32,
                "com.lite_xl.LiteXL",
                title,
                body,
                Vec::<String>::new(),
                hints,
                0i32,
            ),
        )
        .await?;
    Ok(())
}

fn escape_markup(message: &str) -> String {
    message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    #[test]
    fn diagnostics_are_literal_markup() {
        assert_eq!(
            super::escape_markup("<a href=\"x&y\">'字' &amp;</a>"),
            "&lt;a href=\"x&amp;y\"&gt;'字' &amp;amp;&lt;/a&gt;"
        );
    }
}
