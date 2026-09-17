// SPDX-License-Identifier: MIT

use std::env;
use std::path::PathBuf;

use sts2_game_mod::{ExactRestoreCurrentOwner, ExactRestoreError, ExactRestoreOwnerFence};

pub(crate) const PRINCIPAL_ENV: &str = "STS2_CALLER_ID";
pub(crate) const TOKEN_ENV: &str = "STS2_MOD_TOKEN";

#[derive(Clone, Debug)]
pub(crate) struct TransportConfig {
    pub(crate) principal: String,
    pub(crate) token: String,
    pub(crate) instance_id: String,
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) lease_epoch: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Config {
    pub(crate) address: String,
    pub(crate) transport: TransportConfig,
    pub(crate) owner_file: PathBuf,
    pub(crate) store_dir: PathBuf,
    pub(crate) unsupported: bool,
    pub(crate) lookup_unknown_once: bool,
    pub(crate) commit_unknown: bool,
    pub(crate) max_requests: Option<usize>,
}

#[derive(serde::Deserialize)]
struct OwnerSnapshot {
    fence: ExactRestoreOwnerFence,
    observed_at_millis: u64,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self, String> {
        let transport = TransportConfig {
            principal: required(PRINCIPAL_ENV)?,
            token: required(TOKEN_ENV)?,
            instance_id: required("STS2_INSTANCE_ID")?,
            session_id: required("STS2_SESSION_ID")?,
            lease_id: required("STS2_LEASE_ID")?,
            lease_epoch: required("STS2_LEASE_EPOCH")?
                .parse()
                .map_err(|_| String::from("STS2_LEASE_EPOCH must be an integer"))?,
        };
        let owner_file = PathBuf::from(required("STS2_EXACT_OWNER_FILE")?);
        let store_dir = PathBuf::from(required("STS2_EXACT_STORE")?);
        if transport.token.is_empty()
            || transport.token.len() > 256
            || transport.principal.is_empty()
            || transport.instance_id.is_empty()
        {
            return Err(String::from("invalid configured transport identity"));
        }
        Ok(Self {
            address: env::var("STS2_MOD_ADDR").unwrap_or_else(|_| String::from("127.0.0.1:0")),
            transport,
            owner_file,
            store_dir,
            unsupported: flag("STS2_EXACT_NATIVE_UNSUPPORTED"),
            lookup_unknown_once: flag("STS2_EXACT_LOOKUP_UNKNOWN_ONCE"),
            commit_unknown: flag("STS2_EXACT_COMMIT_UNKNOWN"),
            max_requests: env::var("STS2_EXACT_MAX_REQUESTS")
                .ok()
                .map(|value| value.parse::<usize>())
                .transpose()
                .map_err(|_| String::from("STS2_EXACT_MAX_REQUESTS must be an integer"))?,
        })
    }

    pub(crate) fn owner(&self) -> Result<ExactRestoreCurrentOwner, String> {
        let bytes = std::fs::read(&self.owner_file)
            .map_err(|error| format!("read owner fixture: {error}"))?;
        let snapshot: OwnerSnapshot = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode owner fixture: {error}"))?;
        if snapshot.fence.instance_id() != self.transport.instance_id
            || snapshot.fence.session_id() != self.transport.session_id
            || snapshot.fence.lease_id() != self.transport.lease_id
            || snapshot.fence.lease_epoch() != self.transport.lease_epoch
            || env::var("STS2_OWNER_ID")
                .ok()
                .is_some_and(|value| value != snapshot.fence.host_fence_id())
            || env::var("STS2_OWNER_GENERATION")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|value| value != snapshot.fence.host_fence_generation())
        {
            return Err(String::from(
                "owner fixture does not match configured transport identity",
            ));
        }
        Ok(ExactRestoreCurrentOwner {
            fence: snapshot.fence,
            observed_at_millis: snapshot.observed_at_millis,
        })
    }
}

#[allow(dead_code)]
pub(crate) fn write_owner(
    path: &std::path::Path,
    owner: &ExactRestoreCurrentOwner,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "fence": owner.fence,
        "observed_at_millis": owner.observed_at_millis,
    }))
    .map_err(|error| format!("encode owner fixture: {error}"))?;
    std::fs::write(path, bytes).map_err(|error| format!("write owner fixture: {error}"))
}

fn required(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("{name} is required"))
}

fn flag(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

#[allow(dead_code)]
fn _error_type_is_public(_: ExactRestoreError) {}
