use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--help") | Some("-h") | None => {
            println!("easynet");
        }
        Some("config") => config_cmd(args),
        Some(_) => {
            println!("easynet");
        }
    }
}

fn config_cmd(mut args: impl Iterator<Item = String>) {
    match args.next().as_deref() {
        Some("--help") | Some("-h") | None => {
            println!("easynet");
        }
        Some("init") => config_init(args),
        Some(_) => {
            println!("easynet");
        }
    }
}

fn config_init(mut args: impl Iterator<Item = String>) {
    let path = match args.next().as_deref() {
        Some("--help") | Some("-h") => {
            println!("easynet");
            return;
        }
        Some("--path") => match args.next() {
            Some(path) => PathBuf::from(path),
            None => {
                eprintln!("easynet: --path requires a file");
                std::process::exit(2);
            }
        },
        None => user_config_path(),
        Some(other) => {
            eprintln!("easynet: unexpected argument {other}");
            std::process::exit(2);
        }
    };
    if args.next().is_some() {
        eprintln!("easynet: unexpected extra argument");
        std::process::exit(2);
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                eprintln!("easynet: {err}");
                std::process::exit(1);
            }
        }
    }
    if !path.exists() {
        if let Err(err) = std::fs::write(&path, easynet_core::default_toml()) {
            eprintln!("easynet: {err}");
            std::process::exit(1);
        }
    }
    println!("{}", path.display());
}

/// Spec §9 user path. Does not require root and does not write `/etc`.
fn user_config_path() -> PathBuf {
    if cfg!(windows) {
        let root = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        return PathBuf::from(root).join("EasyNet").join("easynet.toml");
    }
    let home = match std::env::var("HOME") {
        Ok(home) => home,
        Err(_) => {
            eprintln!("easynet: HOME is not set");
            std::process::exit(1);
        }
    };
    PathBuf::from(home)
        .join(".config")
        .join("easynet")
        .join("easynet.toml")
}
