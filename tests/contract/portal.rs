//! Private test portal; invoked explicitly by scripts/rust-port/dialog-check.sh, never by normal tests.
use ashpd::zbus::{
    self,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};
use std::{collections::HashMap, path::PathBuf};

struct Chooser {
    work: PathBuf,
}

impl Chooser {
    async fn request(
        &self,
        parent: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        connection: &zbus::Connection,
        header: zbus::message::Header<'_>,
        save: bool,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        assert!(
            parent.starts_with("x11:") && parent != "x11:0",
            "missing native parent"
        );
        assert!(
            bool::try_from(&options["modal"]).unwrap(),
            "missing modality"
        );
        if title == "transport" {
            return Err(zbus::fdo::Error::Failed("scripted transport error".into()));
        }
        if title == "options" {
            assert!(!save);
            assert!(bool::try_from(&options["multiple"]).unwrap());
            assert_eq!(
                <&str>::try_from(&options["accept_label"]).unwrap(),
                "Choose"
            );
            let filters: Vec<(String, Vec<(u32, String)>)> =
                options["filters"].try_clone().unwrap().try_into().unwrap();
            assert_eq!(
                filters,
                [
                    (
                        "Images".into(),
                        vec![(0, "*.png".into()), (0, "*.jpg".into())]
                    ),
                    ("All".into(), vec![(0, "*".into())])
                ]
            );
            let folder: Vec<u8> = options["current_folder"]
                .try_clone()
                .unwrap()
                .try_into()
                .unwrap();
            assert_eq!(folder, b"/tmp/\xff\0");
        }
        if title == "directory" {
            assert!(bool::try_from(&options["directory"]).unwrap());
        }
        if title == "savebytes" {
            use std::os::unix::ffi::OsStrExt;
            assert!(save && !options.contains_key("current_name"));
            let file: Vec<u8> = options["current_file"]
                .try_clone()
                .unwrap()
                .try_into()
                .unwrap();
            let mut expected = self.work.as_os_str().as_bytes().to_vec();
            expected.extend_from_slice(b"/\xff\0");
            assert_eq!(file, expected);
        }
        if title == "Save File" {
            assert!(save);
            assert_eq!(
                <&str>::try_from(&options["current_name"]).unwrap(),
                "new.txt"
            );
        }
        let token = <&str>::try_from(&options["handle_token"]).unwrap();
        let sender = header.sender().unwrap().as_str().to_owned();
        let path = OwnedObjectPath::try_from(format!(
            "/org/freedesktop/portal/desktop/request/{}/{}",
            sender.trim_start_matches(':').replace('.', "_"),
            token
        ))
        .unwrap();
        let tag = match title {
            "held" | "restart" | "new" | "exit" | "dup1" | "dup2" => title.to_owned(),
            _ => "ordinary".to_owned(),
        };
        std::fs::write(self.work.join(format!("received-{tag}")), parent).unwrap();
        let connection = connection.clone();
        let work = self.work.clone();
        let request_path = path.clone();
        let title = title.to_owned();
        tokio::spawn(async move {
            if tag != "ordinary" {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
                while !work.join(format!("release-{tag}")).exists() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "fixture release timeout"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
            let code = match title.as_str() {
                "cancel1" => 1u32,
                "cancel2" => 2,
                _ => 0,
            };
            let uris = match title.as_str() {
                "empty" | "cancel1" | "cancel2" => vec![],
                "invalid" => vec![
                    "file:///ok".to_owned(),
                    "https://remote.invalid/not-a-file".into(),
                ],
                "nonutf8" => vec!["file://localhost/tmp/%FF%20%23".into()],
                _ => vec![format!("file:///tmp/{title}")],
            };
            let mut results = HashMap::<&str, Value<'_>>::new();
            results.insert("uris", Value::from(uris));
            connection
                .emit_signal(
                    Some(sender.as_str()),
                    request_path,
                    "org.freedesktop.portal.Request",
                    "Response",
                    &(code, results),
                )
                .await
                .unwrap();
            std::fs::write(work.join(format!("sent-{tag}")), "sent").unwrap();
        });
        Ok(path)
    }
}

#[zbus::interface(name = "org.freedesktop.portal.FileChooser", crate = "ashpd::zbus")]
impl Chooser {
    #[zbus(property)]
    fn version(&self) -> u32 {
        4
    }

    async fn open_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        self.request(parent_window, title, options, connection, header, false)
            .await
    }
    async fn save_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        self.request(parent_window, title, options, connection, header, true)
            .await
    }
}

#[test]
#[ignore = "isolated private bus service; started explicitly by dialog-check.sh"]
fn serve() {
    let work =
        PathBuf::from(std::env::var_os("DIALOG_CHECK_WORK").expect("private fixture workdir"));
    assert_eq!(
        std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap(),
        format!("unix:path={}/bus", work.display())
    );
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _connection = zbus::connection::Builder::session()
                .unwrap()
                .name("org.freedesktop.portal.Desktop")
                .unwrap()
                .serve_at(
                    "/org/freedesktop/portal/desktop",
                    Chooser { work: work.clone() },
                )
                .unwrap()
                .build()
                .await
                .unwrap();
            std::fs::write(work.join("portal-ready"), "ready").unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            panic!("private fixture outlived test deadline");
        });
}
