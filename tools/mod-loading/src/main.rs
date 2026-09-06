// SPDX-License-Identifier: MIT

use std::path::PathBuf;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut settings = None;
    let mut mods = None;
    let mut backups = None;
    let mut expected = None;
    let mut apply = false;
    while let Some(arg) = args.next() {
        if arg == "--apply" {
            if apply {
                return Err("duplicate apply option".into());
            }
            apply = true;
            continue;
        }
        if arg == "--help" {
            println!(
                "sts2-game-mod-loading --settings-file ABSOLUTE_PATH --mods-dir ABSOLUTE_PATH\n\
                [--apply --backup-dir ABSOLUTE_PATH --expected-sha256 DIGEST]\n\
                Offline preparation only: the launch owner must keep the selected game stopped.\n\
                Plan mode is read-only. Apply enables only AIAscensionSTS2GameMod in a validated directory."
            );
            return Ok(());
        }
        let value = args.next().ok_or("option requires a value")?;
        let target = match arg.as_str() {
            "--settings-file" => &mut settings,
            "--mods-dir" => &mut mods,
            "--backup-dir" => &mut backups,
            "--expected-sha256" => &mut expected,
            _ => return Err("unknown option".into()),
        };
        if target.replace(value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let settings = PathBuf::from(settings.ok_or("settings-file is required")?);
    let mods = PathBuf::from(mods.ok_or("mods-dir is required")?);
    let plan = sts2_game_mod_loading::inspect(&settings, &mods)?;
    let backup_label = if apply {
        let backups = PathBuf::from(backups.ok_or("apply requires backup-dir")?);
        let expected = expected.ok_or("apply requires expected-sha256")?;
        if plan.before_sha256 != expected {
            return Err("settings changed since review".into());
        }
        sts2_game_mod_loading::apply(&settings, &mods, &backups, &expected)?
    } else {
        if backups.is_some() || expected.is_some() {
            return Err("apply-only options require --apply".into());
        }
        None
    };
    println!(
        "{}",
        serde_json::json!({"addon_id":sts2_game_mod_loading::ADDON_ID,
        "mode":if apply {"applied"} else {"plan"},"changed":plan.changed,
        "before_sha256":plan.before_sha256,"after_sha256":plan.after_sha256,"backup_label":backup_label})
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("mod-loading preparation failed: {error}");
        std::process::exit(2);
    }
}
