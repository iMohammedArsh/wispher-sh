use std::process::Command;
use std::env;
use std::path::Path;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        std::process::exit(1);
    }

    let mut cmd_args = args.clone();
    if cmd_args[0] == "rustc" {
        cmd_args[0] = r"C:\Users\Administrator\.cargo\bin\rustc.exe".to_string();
    }

    let status = Command::new(&cmd_args[0])
        .args(&cmd_args[1..])
        .status()
        .unwrap_or_else(|_| std::process::exit(1));

    if status.success() {
        for i in 0..cmd_args.len() {
            if cmd_args[i] == "-o" && i + 1 < cmd_args.len() {
                let out_file = &cmd_args[i + 1];
                let is_build_script = out_file.contains("build_script_build") 
                                   || out_file.contains("build-script-build");
                if is_build_script && out_file.ends_with(".exe") && Path::new(out_file).exists() {
                    let zone_file = format!("{}:Zone.Identifier", out_file);
                    let _ = fs::remove_file(zone_file);
                }
            }
        }
    }

    std::process::exit(status.code().unwrap_or(1));
}
