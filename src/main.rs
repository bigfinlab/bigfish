//! BigFish CLI —— 启动器内核的命令行入口
use bigfish::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("versions") | None => {
            match list_versions() {
                Ok(vs) => {
                    let mut counts: std::collections::BTreeMap<&str, usize> = Default::default();
                    for v in &vs {
                        *counts.entry(v.kind.as_str()).or_default() += 1;
                    }
                    println!("版本总数: {}", vs.len());
                    println!("类型统计: {counts:?}");
                    for v in vs.iter().take(15) {
                        println!("  {:<18} {:<10} {}", v.id, v.kind, v.release_time);
                    }
                }
                Err(e) => eprintln!("[error] {e}"),
            }
        }
        Some("download") => {
            let id = args.get(2).cloned().unwrap_or_else(|| "1.20.1".into());
            let dir = args.get(3).cloned().unwrap_or_else(|| "./test-game".into());
            let progress = |m: &str| println!("{m}");
            if let Err(e) = download_version(&id, &dir, &progress) {
                eprintln!("[error] {e}");
                std::process::exit(1);
            }
        }
        Some("java") => {
            let javas = scan_local_javas();
            if javas.is_empty() {
                println!("没有检测到任何 Java");
            }
            for j in &javas {
                println!("Java {:<2}  {:<12} [{}] {}", j.major, j.version, j.source, j.path);
            }
            if let Some(id) = args.get(2) {
                let dir = args.get(3).cloned().unwrap_or_else(|| ".".into());
                let need = java_requirement(id, &dir);
                let hit = find_java_for(need);
                println!("\n版本 {id} 需要 Java {need} -> {:?}", hit.map(|j| j.path));
            }
        }
        Some("install-java") => {
            let major: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(21);
            let progress = |m: &str| println!("{m}");
            match install_java(major, &progress) {
                Ok(info) => println!("✅ Java {} 可用: {}", info.major, info.path),
                Err(e) => eprintln!("[error] {e}"),
            }
        }
        Some("saves") => {
            let saves = scan_saves();
            println!("发现 {} 个存档:", saves.len());
            for s in saves {
                let when = if s.last_played > 0 {
                    format!("{}", s.last_played / 1000)
                } else { "-".into() };
                println!("  [{}] {} | 版本 {} | 最后游玩 {} | {}", s.source, s.display_name, s.version, when, s.path);
            }
        }
        Some("launch") => {
            let id = args.get(2).cloned().unwrap_or_else(|| "1.20.1".into());
            let dir = args.get(3).cloned().unwrap_or_else(|| "./test-game".into());
            let user = args.get(4).cloned().unwrap_or_else(|| "George".into());
            if args.iter().any(|a| a == "--preview") {
                match launch_preview(&id, &dir, &user) {
                    Ok(s) => println!("{s}"),
                    Err(e) => eprintln!("[error] {e}"),
                }
            } else {
                match launch_version(&id, &dir, &user) {
                    Ok(pid) => println!("✅ 游戏已启动 (pid {pid})"),
                    Err(e) => eprintln!("[error] {e}"),
                }
            }
        }
        Some(other) => {
            eprintln!("未知命令: {other}");
            eprintln!("用法:");
            eprintln!("  bigfish versions                      列出所有版本");
            eprintln!("  bigfish download <版本> [目录]        下载完整游戏文件");
            eprintln!("  bigfish launch <版本> [目录] [用户名] 启动游戏");
            eprintln!("  bigfish launch ... --preview          只打印启动命令");
            std::process::exit(2);
        }
    }
}
