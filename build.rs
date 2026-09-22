use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=CF_PAGES_COMMIT_SHA");
    println!("cargo:rerun-if-env-changed=CF_COMMIT_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let revision = ["CF_PAGES_COMMIT_SHA", "CF_COMMIT_SHA", "GITHUB_SHA"]
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.is_empty()))
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=8", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_owned())
        })
        .unwrap_or_else(|| "development".to_owned());
    let short_revision: String = revision.chars().take(8).collect();
    println!("cargo:rustc-env=IRONWOOD_BUILD_ID={short_revision}");
}
