#[cfg(target_os = "windows")]
fn configure_windows_sdk_tools() {
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    fn arch_folder(target: &str) -> &'static str {
        if target.starts_with("x86_64-") {
            "x64"
        } else if target.starts_with("aarch64-") {
            "arm64"
        } else {
            "x86"
        }
    }

    fn find_sdk_bin_dir(arch: &str) -> Option<PathBuf> {
        let roots = ["ProgramFiles(x86)", "ProgramFiles"];

        for root in roots {
            let Some(base) = env::var_os(root) else {
                continue;
            };

            let bin_root = Path::new(&base).join("Windows Kits").join("10").join("bin");
            if !bin_root.is_dir() {
                continue;
            }

            let Ok(entries) = fs::read_dir(&bin_root) else {
                continue;
            };

            let mut version_dirs = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect::<Vec<_>>();

            version_dirs.sort_by(|left, right| right.cmp(left));

            for version_dir in version_dirs {
                let candidate = version_dir.join(arch);
                if candidate.join("rc.exe").is_file() && candidate.join("mt.exe").is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows-msvc") {
        return;
    }

    let rc_target_env = format!("RC_{}", target.replace('-', "_"));
    let mt_target_env = format!("MT_{}", target.replace('-', "_"));

    if env::var_os("RC").is_some()
        || env::var_os(&rc_target_env).is_some()
        || env::var_os("MT").is_some()
        || env::var_os(&mt_target_env).is_some()
    {
        return;
    }

    let Some(bin_dir) = find_sdk_bin_dir(arch_folder(&target)) else {
        println!("cargo:warning=Windows SDK tools were not found automatically.");
        return;
    };

    let rc_path = bin_dir.join("rc.exe");
    let mt_path = bin_dir.join("mt.exe");

    env::set_var("RC", &rc_path);
    env::set_var(&rc_target_env, &rc_path);
    env::set_var("MT", &mt_path);
    env::set_var(&mt_target_env, &mt_path);

    let existing_path = env::var_os("PATH").unwrap_or_default();
    let joined = env::join_paths(
        std::iter::once(bin_dir.clone()).chain(env::split_paths(&existing_path)),
    )
    .unwrap_or(existing_path);
    env::set_var("PATH", joined);

    println!(
        "cargo:warning=Using Windows SDK tools from {}",
        bin_dir.display()
    );
}

fn main() {
    #[cfg(target_os = "windows")]
    configure_windows_sdk_tools();

    tauri_build::build()
}
