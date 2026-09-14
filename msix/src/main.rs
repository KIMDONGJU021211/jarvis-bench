//! MSIX 점검기 — 패키지 안에서 실행돼 자비스가 기대는 동작이 실제로 되는지 적는다.
//!
//! 자비스가 MSIX 안에서 깨질 수 있는 자리:
//!   1. `%LOCALAPPDATA%\AltureJarvis` 에 **새로** 만드는 토큰 — 가상화되면 다른 앱이 읽는 경로에 없다
//!   2. 이미 있는 다른 앱 설정 파일 **고치기** — 방식(덮어쓰기·기존 열기·임시파일+이름바꾸기)에 따라 갈릴 수 있다
//!   3. Claude Desktop 이 MSIX 면 진짜 설정은 `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude` 에 있다
//!   4. 백엔드(자식 프로세스)가 부모와 같은 파일 보기를 갖는가, 루프백
//! 실제 설정 파일은 **크기만 읽고** 쓰지 않는다. 고치기 시험은 패키지 밖에서 미리 만든 점검 전용 파일로 한다.
//!
//! 실행 파일 이름이 `jarvis_msix_seed.exe` 면 씨앗만 만들고 끝낸다(패키지 밖에서 explorer 로 띄운다).
#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SEED_FILES: [&str; 3] = ["create.json", "open.json", "rename.json"];

fn var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn outcome<T>(result: std::io::Result<T>) -> String {
    match result {
        Ok(_) => quote("ok"),
        Err(error) => quote(&format!("err: {error}")),
    }
}

fn write_new(path: &Path) -> String {
    outcome((|| {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b"probe")
    })())
}

fn cmd(line: String) -> String {
    let status = Command::new("cmd.exe").raw_arg(line).creation_flags(CREATE_NO_WINDOW).status();
    quote(&format!("{:?}", status.map(|s| s.code())))
}

fn fake_package(local: &Path) -> PathBuf {
    local.join("Packages").join("JarvisProbeFakePkg").join("LocalCache").join("Roaming")
}

fn seed(appdata: &Path, local: &Path, home: &Path) {
    for base in [appdata, local] {
        let dir = base.join("JarvisMsixSeed");
        let _ = fs::create_dir_all(&dir);
        for name in SEED_FILES {
            let _ = fs::write(dir.join(name), b"seed");
        }
    }
    let fake = fake_package(local);
    let _ = fs::create_dir_all(&fake);
    let _ = fs::write(fake.join("existing.json"), b"seed");
    let marker = home.join(".JarvisMsixSeed");
    let _ = fs::create_dir_all(&marker);
    let _ = fs::write(marker.join("done.txt"), b"seeded");
}

fn main() {
    let exe = env::current_exe().unwrap_or_default();
    let appdata = PathBuf::from(var("APPDATA"));
    let local = PathBuf::from(var("LOCALAPPDATA"));
    let home = PathBuf::from(var("USERPROFILE"));
    if exe.file_stem().map(|stem| stem == "jarvis_msix_seed").unwrap_or(false) {
        seed(&appdata, &local, &home);
        return;
    }

    // 변형(A/B)마다 결과가 섞이지 않게 레이아웃 폴더 이름을 꼬리표로 쓴다.
    let tag = exe
        .parent()
        .and_then(|dir| dir.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "x".into());
    let folder = format!("JarvisMsixProbe-{tag}");
    let home_dir = home.join(format!(".{folder}"));

    // 패키지 **안에 든** 실행 파일을 자식으로 띄운 경우 — 나중의 파이썬 백엔드가 바로 이 모양이다.
    if exe.file_stem().map(|stem| stem == "jarvis_msix_child").unwrap_or(false) {
        let sees = fs::read(local.join(&folder).join("local.txt")).map(|b| b.len()).unwrap_or(0);
        let _ = fs::write(home_dir.join("pkgchild-sees-parent.txt"), if sees > 0 { "yes" } else { "no" });
        let _ = write_new(&local.join(format!("{folder}-pkgchild")).join("new.txt"));
        return;
    }
    let _ = fs::create_dir_all(&home_dir);
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut push = |key: &str, value: String| rows.push((key.to_string(), value));

    push("tag", quote(&tag));
    push("exe", quote(&exe.display().to_string()));

    // 1) 새로 만들기
    push("new_roaming", write_new(&appdata.join(&folder).join("roaming.txt")));
    push("new_local", write_new(&local.join(&folder).join("local.txt")));
    push("new_home_dotdir", write_new(&home_dir.join("home.txt")));

    // 2) 이미 있는 파일 고치기 — 세 방식
    let body = format!("probe-{tag}");
    for (label, base) in [("roaming", &appdata), ("local", &local)] {
        let dir = base.join("JarvisMsixSeed");
        push(&format!("modify_create_always_{label}"), outcome(fs::write(dir.join("create.json"), &body)));
        push(
            &format!("modify_open_existing_{label}"),
            outcome((|| {
                let mut file = fs::OpenOptions::new().write(true).truncate(true).open(dir.join("open.json"))?;
                file.write_all(body.as_bytes())
            })()),
        );
        let tmp = dir.join(format!("rename.json.{tag}.tmp"));
        push(
            &format!("modify_tmp_rename_{label}"),
            outcome((|| {
                fs::write(&tmp, &body)?;
                fs::rename(&tmp, dir.join("rename.json"))
            })()),
        );
    }

    // 3) 다른 패키지 폴더(Claude Desktop MSIX 설정이 사는 모양)
    let fake = fake_package(&local);
    push("other_package_modify_existing", outcome(fs::write(fake.join("existing.json"), &body)));
    push("other_package_new_file", write_new(&fake.join(format!("new-{tag}.json"))));
    let claude_pkg_config = fs::read_dir(local.join("Packages"))
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .find(|entry| entry.file_name().to_string_lossy().starts_with("Claude_"))
        })
        .map(|entry| entry.path().join("LocalCache").join("Roaming").join("Claude").join("claude_desktop_config.json"));
    let claude_pkg_bytes = claude_pkg_config
        .and_then(|path| fs::metadata(path).ok())
        .map(|meta| meta.len() as i64)
        .unwrap_or(-1);
    push("read_claude_msix_config_bytes", claude_pkg_bytes.to_string());
    let appdata_bytes = fs::metadata(appdata.join("Claude").join("claude_desktop_config.json"))
        .map(|meta| meta.len() as i64)
        .unwrap_or(-1);
    push("read_claude_appdata_config_bytes", appdata_bytes.to_string());
    let projects = fs::read_dir(home.join(".claude").join("projects"))
        .map(|entries| entries.count() as i64)
        .unwrap_or(-1);
    push("read_claude_projects_count", projects.to_string());

    // 4) 자식 프로세스 — 돌기는 하나 / 부모가 만든 가상 파일이 보이나 / 자식이 새로 만든 AppData 는 어디로
    push("child_home_write", cmd(format!("/c echo child> \"{}\"", home_dir.join("child-home.txt").display())));
    push(
        "child_reads_parent_private",
        cmd(format!(
            "/c type \"{}\" > \"{}\"",
            local.join(&folder).join("local.txt").display(),
            home_dir.join("child-sees-parent.txt").display()
        )),
    );
    let child_dir = local.join(format!("{folder}-child"));
    push(
        "child_new_appdata",
        cmd(format!("/c mkdir \"{d}\" 2>nul & echo x> \"{d}\\new.txt\"", d = child_dir.display())),
    );

    let package_child = exe.with_file_name("jarvis_msix_child.exe");
    let package_child_status = Command::new(&package_child).creation_flags(CREATE_NO_WINDOW).status();
    push("package_child_exit", quote(&format!("{:?}", package_child_status.map(|s| s.code()))));

    let self_loop = (|| -> std::io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let _client = TcpStream::connect_timeout(&listener.local_addr()?, Duration::from_secs(2))?;
        listener.accept().map(|_| ())
    })();
    push("loopback_self", outcome(self_loop));
    let outside = (|| -> std::io::Result<String> {
        let mut stream = TcpStream::connect_timeout(&"127.0.0.1:8091".parse().unwrap(), Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")?;
        let mut text = String::new();
        let _ = stream.read_to_string(&mut text);
        Ok(text.lines().next().unwrap_or("").to_string())
    })();
    push(
        "loopback_outside_8091",
        quote(&match outside {
            Ok(line) => line,
            Err(e) => format!("err: {e}"),
        }),
    );

    let json = format!(
        "{{\n{}\n}}\n",
        rows.iter().map(|(k, v)| format!("  \"{k}\": {v}")).collect::<Vec<_>>().join(",\n")
    );
    let _ = fs::write(home_dir.join("result.json"), &json);
}
