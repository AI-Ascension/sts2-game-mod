// SPDX-License-Identifier: MIT

//! Command-line wrapper for the minimal seeded-run client.

use std::collections::BTreeSet;

use sts2_seeded_run_client::context::{
    CanonicalContext, Compatibility, IdentityDigest, ProfileBaseline, hex_sha256, is_digest,
};
use sts2_seeded_run_client::request::{StartRequestParams, build_start_request};
use sts2_seeded_run_client::wire::post_seeded_run;

#[derive(Debug)]
struct Args {
    host: String,
    port: u16,
    token: String,
    caller_id: String,
    schema_file: Option<String>,
    schema_digest: Option<String>,
    correlation_id: String,
    instance_id: String,
    session_id: String,
    lease_id: String,
    lease_epoch: u64,
    generation: u64,
    operation_id: String,
    seed: String,
    run_mode: String,
    send: bool,
    context: CanonicalContext,
}

fn usage() -> String {
    [
        "Usage: sts2-seeded-run-client --token T --caller-id C [options] [--send|--print]",
        "  --host H (default 127.0.0.1) --port P (default 15526)",
        "  --schema-file PATH | --schema-digest HEX",
        "  --correlation-id --instance-id --session-id --lease-id --lease-epoch --generation",
        "  --operation-id --seed --run-mode",
        "  --context-id --ascension N --act ID (repeat) --modifier ID (repeat)",
        "  --selection-policy --profile-baseline-kind/-identity/-digest --save-policy",
        "  --game-identity --game-digest --mod-identity --mod-digest",
    ]
    .join("\n")
}

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

fn run(argv: Vec<String>) -> Result<(), String> {
    if argv.iter().any(|value| value == "-h" || value == "--help") {
        println!("{}", usage());
        return Ok(());
    }
    let args = parse(&argv)?;
    let schema_digest = resolve_schema_digest(&args)?;
    let params = StartRequestParams {
        schema_digest,
        correlation_id: args.correlation_id.clone(),
        instance_id: args.instance_id.clone(),
        session_id: args.session_id.clone(),
        lease_id: args.lease_id.clone(),
        lease_epoch: args.lease_epoch,
        generation: args.generation,
        operation_id: args.operation_id.clone(),
        requested_seed: args.seed.clone(),
        run_mode: args.run_mode.clone(),
        context: args.context.clone(),
    };
    let request = build_start_request(&params).map_err(|error| error.to_string())?;
    let body = request.to_json().map_err(|error| error.to_string())?;
    if !args.send {
        println!("{body}");
        return Ok(());
    }
    let headers = [
        ("x-sts2-instance-id", args.instance_id.as_str()),
        ("x-sts2-caller-id", args.caller_id.as_str()),
        ("x-sts2-session-id", args.session_id.as_str()),
        ("x-sts2-lease-id", args.lease_id.as_str()),
        ("x-sts2-lease-epoch", &args.lease_epoch.to_string()),
        ("x-sts2-correlation-id", args.correlation_id.as_str()),
    ];
    let (status, response) = post_seeded_run(&args.host, args.port, &args.token, &headers, &body)
        .map_err(|error| error.to_string())?;
    println!("status={status}\n{response}");
    Ok(())
}

fn resolve_schema_digest(args: &Args) -> Result<String, String> {
    if let Some(digest) = &args.schema_digest {
        if !is_digest(digest) {
            return Err("--schema-digest must be a lowercase SHA-256 hex digest".to_owned());
        }
        return Ok(digest.clone());
    }
    let path = args
        .schema_file
        .as_deref()
        .ok_or("--schema-file or --schema-digest is required")?;
    let bytes = std::fs::read(path).map_err(|error| format!("cannot read schema: {error}"))?;
    Ok(hex_sha256(&bytes))
}

struct Builder {
    args: Args,
    acts: Vec<String>,
    modifiers: BTreeSet<String>,
}

impl Builder {
    fn new() -> Self {
        Self {
            args: Args {
                host: "127.0.0.1".to_owned(),
                port: 15526,
                token: String::new(),
                caller_id: String::new(),
                schema_file: None,
                schema_digest: None,
                correlation_id: "corr-1".to_owned(),
                instance_id: "inst-1".to_owned(),
                session_id: "sess-1".to_owned(),
                lease_id: "lease-1".to_owned(),
                lease_epoch: 0,
                generation: 0,
                operation_id: "op-1".to_owned(),
                seed: "seed-1".to_owned(),
                run_mode: "seeded_training".to_owned(),
                send: false,
                context: CanonicalContext {
                    context_id: "ctx-1".to_owned(),
                    game_mode: "standard".to_owned(),
                    character: "ironclad".to_owned(),
                    ascension: 0,
                    modifiers: Vec::new(),
                    acts: Vec::new(),
                    selection_policy: "standard_default".to_owned(),
                    profile_baseline: ProfileBaseline {
                        kind: "fresh".to_owned(),
                        identity: "prof-1".to_owned(),
                        digest: String::new(),
                    },
                    save_policy: "enabled".to_owned(),
                    compatibility: Compatibility {
                        game: IdentityDigest {
                            identity: String::new(),
                            digest: String::new(),
                        },
                        mod_identity: IdentityDigest {
                            identity: String::new(),
                            digest: String::new(),
                        },
                    },
                },
            },
            acts: Vec::new(),
            modifiers: BTreeSet::new(),
        }
    }
}

fn parse(argv: &[String]) -> Result<Args, String> {
    let mut builder = Builder::new();
    let mut index = 0;
    while index < argv.len() {
        let key = argv[index].as_str();
        let value = argv.get(index + 1).cloned();
        let take = |value: &Option<String>, name: &str| -> Result<String, String> {
            value
                .clone()
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match key {
            "--host" => builder.args.host = take(&value, key)?,
            "--port" => {
                builder.args.port = take(&value, key)?.parse().map_err(|_| "invalid --port")?
            }
            "--token" => builder.args.token = take(&value, key)?,
            "--caller-id" => builder.args.caller_id = take(&value, key)?,
            "--schema-file" => builder.args.schema_file = Some(take(&value, key)?),
            "--schema-digest" => builder.args.schema_digest = Some(take(&value, key)?),
            "--correlation-id" => builder.args.correlation_id = take(&value, key)?,
            "--instance-id" => builder.args.instance_id = take(&value, key)?,
            "--session-id" => builder.args.session_id = take(&value, key)?,
            "--lease-id" => builder.args.lease_id = take(&value, key)?,
            "--lease-epoch" => {
                builder.args.lease_epoch = take(&value, key)?
                    .parse()
                    .map_err(|_| "invalid --lease-epoch")?
            }
            "--generation" => {
                builder.args.generation = take(&value, key)?
                    .parse()
                    .map_err(|_| "invalid --generation")?
            }
            "--operation-id" => builder.args.operation_id = take(&value, key)?,
            "--seed" => builder.args.seed = take(&value, key)?,
            "--run-mode" => builder.args.run_mode = take(&value, key)?,
            "--context-id" => builder.args.context.context_id = take(&value, key)?,
            "--ascension" => {
                builder.args.context.ascension = take(&value, key)?
                    .parse()
                    .map_err(|_| "invalid --ascension")?
            }
            "--act" => builder.acts.push(take(&value, key)?),
            "--modifier" => {
                builder.modifiers.insert(take(&value, key)?);
            }
            "--selection-policy" => builder.args.context.selection_policy = take(&value, key)?,
            "--profile-baseline-kind" => {
                builder.args.context.profile_baseline.kind = take(&value, key)?
            }
            "--profile-baseline-identity" => {
                builder.args.context.profile_baseline.identity = take(&value, key)?
            }
            "--profile-baseline-digest" => {
                builder.args.context.profile_baseline.digest = take(&value, key)?
            }
            "--save-policy" => builder.args.context.save_policy = take(&value, key)?,
            "--game-identity" => {
                builder.args.context.compatibility.game.identity = take(&value, key)?
            }
            "--game-digest" => builder.args.context.compatibility.game.digest = take(&value, key)?,
            "--mod-identity" => {
                builder.args.context.compatibility.mod_identity.identity = take(&value, key)?
            }
            "--mod-digest" => {
                builder.args.context.compatibility.mod_identity.digest = take(&value, key)?
            }
            "--send" => {
                builder.args.send = true;
                index += 1;
                continue;
            }
            "--print" => {
                builder.args.send = false;
                index += 1;
                continue;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 2;
    }
    builder.args.context.acts = builder.acts;
    builder.args.context.modifiers = builder.modifiers.into_iter().collect();
    if builder.args.token.is_empty() && builder.args.send {
        return Err("--token is required with --send".to_owned());
    }
    if builder.args.caller_id.is_empty() {
        return Err("--caller-id is required".to_owned());
    }
    Ok(builder.args)
}
