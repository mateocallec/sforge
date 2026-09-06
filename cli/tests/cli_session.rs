use assert_cmd::Command;
use predicates::prelude::*;

fn sforge(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("sforge").unwrap();
    cmd.env("SFORGE_DATA_DIR", data_dir);
    cmd
}

#[test]
fn full_session_lifecycle() {
    let data_dir = tempfile::tempdir().unwrap();

    sforge(data_dir.path())
        .args([
            "session",
            "setup",
            "acme-admin",
            "--set-cookie",
            "session_id=abc123; Domain=app.acme.test; Path=/; Secure; HttpOnly",
            "--set-cookie",
            "csrf=xyz789; Domain=acme.test; SameSite=Strict",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("acme-admin"))
        .stdout(predicate::str::contains("2 cookie"));

    sforge(data_dir.path())
        .args(["session", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("acme-admin"));

    sforge(data_dir.path())
        .args(["session", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"cookies\": 2"));

    // Re-running setup with only a header should replace headers but leave
    // the previously-captured cookies untouched (per-category replace).
    sforge(data_dir.path())
        .args(["session", "setup", "acme-admin", "-H", "X-Debug: 1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 header"));

    sforge(data_dir.path())
        .args(["session", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"cookies\": 2"))
        .stdout(predicate::str::contains("\"headers\": 1"));

    sforge(data_dir.path())
        .args(["session", "delete", "acme-admin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    sforge(data_dir.path())
        .args(["session", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No sessions yet"));
}

#[test]
fn deleting_an_unknown_session_fails_with_a_clear_error() {
    let data_dir = tempfile::tempdir().unwrap();

    sforge(data_dir.path())
        .args(["session", "delete", "does-not-exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no session named"));
}

#[test]
fn set_cookie_without_domain_is_rejected() {
    let data_dir = tempfile::tempdir().unwrap();

    sforge(data_dir.path())
        .args(["session", "setup", "broken", "--set-cookie", "a=b"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Domain"));
}

#[test]
fn session_names_with_unusual_characters_round_trip() {
    let data_dir = tempfile::tempdir().unwrap();
    let name = "acme QA \u{2013} client's \"prod\" copy";

    sforge(data_dir.path())
        .args([
            "session",
            "setup",
            name,
            "--set-cookie",
            "a=b; Domain=x.test",
        ])
        .assert()
        .success();

    sforge(data_dir.path())
        .args(["session", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(name));

    sforge(data_dir.path())
        .args(["session", "delete", name])
        .assert()
        .success();
}
