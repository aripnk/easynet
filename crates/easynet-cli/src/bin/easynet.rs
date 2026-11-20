use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--help") | Some("-h") | None => {
            println!("easynet");
        }
        Some("config") => config_cmd(args),
        Some("stun") => stun_cmd(args),
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

fn stun_cmd(mut args: impl Iterator<Item = String>) {
    let mut bind = None;
    let mut server = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("easynet");
                return;
            }
            "--bind" => bind = args.next(),
            "--server" => server = args.next(),
            other => {
                eprintln!("easynet: unexpected argument {other}");
                std::process::exit(2);
            }
        }
    }
    let Some(bind) = bind else {
        eprintln!("easynet: --bind is required");
        std::process::exit(2);
    };
    let Some(server) = server else {
        eprintln!("easynet: --server is required");
        std::process::exit(2);
    };
    let bind = match bind.parse() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("easynet: --bind is not IP:PORT");
            std::process::exit(2);
        }
    };
    let server = match server.parse() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("easynet: --server is not IP:PORT");
            std::process::exit(2);
        }
    };
    match easynet_net::binding_reflexive_blocking(bind, server) {
        Ok(addr) => println!("reflexive={addr}"),
        Err(err) => {
            eprintln!("easynet: {err}");
            std::process::exit(1);
        }
    }
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
