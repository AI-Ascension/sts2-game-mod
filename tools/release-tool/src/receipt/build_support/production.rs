// SPDX-License-Identifier: MIT

use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde_json::Map;

mod tool;

use super::{Args, Inputs, host_records};
use crate::common::{MANAGED_NAME, fail, regular_file_path, run_logged};
use tool::{command_output, dotnet_path, resolve_dotnet};

pub(crate) fn production_inputs(
    repo: &Path,
    host: &Path,
    args: &Args,
    commit: &str,
    scratch: &Path,
    logs: &Path,
) -> Result<Inputs, String> {
    let tool = resolve_dotnet(&args.dotnet)?;
    let mut envs: Vec<(String, String)> = env::vars()
        .filter(|(key, _)| key != "RUSTFLAGS" && key != "CARGO_ENCODED_RUSTFLAGS")
        .collect();
    envs.extend([
        ("DOTNET_CLI_TELEMETRY_OPTOUT".into(), "1".into()),
        ("DOTNET_SKIP_FIRST_TIME_EXPERIENCE".into(), "1".into()),
    ]);
    let dotnet_home = scratch.join("dotnet-home");
    fs::create_dir(&dotnet_home).map_err(|error| format!("cannot create dotnet home: {error}"))?;
    envs.push((
        "DOTNET_CLI_HOME".into(),
        dotnet_path(&tool, &dotnet_home, "DOTNET_CLI_HOME")?,
    ));
    let rustc = command_output(
        Command::new("rustc")
            .arg("--version")
            .current_dir(repo)
            .envs(envs.clone())
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS"),
        "rustc version",
    )?;
    let cargo = command_output(
        Command::new("cargo")
            .arg("--version")
            .current_dir(repo)
            .envs(envs.clone())
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS"),
        "cargo version",
    )?;
    let dotnet_version = command_output(
        Command::new(&tool.path)
            .arg("--version")
            .current_dir(repo)
            .envs(envs.clone())
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS"),
        "dotnet version",
    )?;
    let native_target = scratch.join("native-target");
    fs::create_dir(&native_target)
        .map_err(|error| format!("cannot create native target: {error}"))?;
    let platform = &args.platform;
    let (triple, built_name) = match platform.as_str() {
        "windows-x86_64" => (
            "x86_64-pc-windows-gnu",
            "ai_ascension_sts2_game_mod_native.dll",
        ),
        "linux-x86_64" => (
            "x86_64-unknown-linux-gnu",
            "libai_ascension_sts2_game_mod_native.so",
        ),
        _ => return fail(format!("unsupported platform: {platform}")),
    };
    let helper = regular_file_path(
        &repo.join("experiments/managed-rust-interop/build-native-release.sh"),
        "native build helper",
    )?;
    if helper
        .metadata()
        .map_err(|error| error.to_string())?
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        return fail("native build helper is not executable");
    }
    let native_log = logs.join("native-build.log");
    run_logged(
        Command::new(&helper)
            .arg(platform)
            .current_dir(repo)
            .envs(envs.clone())
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env("CARGO_TARGET_DIR", &native_target),
        &native_log,
        "native build",
    )?;
    let native_output = regular_file_path(
        &native_target.join(triple).join("release").join(built_name),
        "native build output",
    )?;
    let managed_output = scratch.join("managed-output");
    let managed_intermediate = scratch.join("managed-intermediate");
    fs::create_dir(&managed_output)
        .map_err(|error| format!("cannot create managed output: {error}"))?;
    fs::create_dir(&managed_intermediate)
        .map_err(|error| format!("cannot create managed intermediate: {error}"))?;
    let project = repo.join("experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj");
    let separator = if tool.style == "windows" { "\\" } else { "/" };
    let output_arg = format!(
        "{}{}",
        dotnet_path(&tool, &managed_output, "managed output")?,
        separator
    );
    let intermediate_arg = format!(
        "{}{}",
        dotnet_path(&tool, &managed_intermediate, "managed intermediate")?,
        separator
    );
    let mut managed = Command::new(&tool.path);
    managed
        .args(["restore", &dotnet_path(&tool, &project, "managed project")?])
        .arg(format!(
            "-p:STS2GameDataDir={}",
            dotnet_path(&tool, host, "host data directory")?
        ))
        .args([
            format!("-p:SourceRevisionId={commit}"),
            format!("-p:Version={}", args.package_version),
            format!("-p:PackageVersion={}", args.package_version),
            format!("-p:OutputPath={output_arg}"),
            format!("-p:BaseIntermediateOutputPath={intermediate_arg}"),
        ])
        .envs(envs.clone())
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .current_dir(repo);
    for name in [
        "EnableCombatDemoProbe",
        "EnableVideoMenuProbe",
        "EnableHandChoiceProbe",
        "EnableTerminalProbe",
        "EnableNativeCoopProbe",
    ] {
        managed.arg(format!("-p:{name}=false"));
    }
    run_logged(
        &mut managed,
        &logs.join("managed-restore.log"),
        "managed restore",
    )?;
    let mut build_args: Vec<String> = managed
        .get_args()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();
    build_args[0] = "build".into();
    build_args.extend([
        "--configuration".into(),
        "Release".into(),
        "--no-restore".into(),
    ]);
    let mut build = Command::new(&tool.path);
    build
        .args(&build_args)
        .envs(envs)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .current_dir(repo);
    run_logged(&mut build, &logs.join("managed-build.log"), "managed build")?;
    let managed_output_file =
        regular_file_path(&managed_output.join(MANAGED_NAME), "managed build output")?;
    let hosts = host_records(host)?;
    Ok(Inputs {
        managed: managed_output_file,
        native: native_output,
        toolchain: Map::from_iter([
            (
                "dotnet_executable".into(),
                tool.path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
                    .into(),
            ),
            ("dotnet_path_style".into(), tool.style.into()),
            ("dotnet_version".into(), dotnet_version.into()),
            ("rustc_version".into(), rustc.into()),
            ("cargo_version".into(), cargo.into()),
        ]),
        hosts: hosts.records,
        host_values: hosts.values,
    })
}
