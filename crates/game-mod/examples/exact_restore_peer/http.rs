// SPDX-License-Identifier: MIT

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use sts2_game_mod::{
    EXACT_RESTORE_MAX_FRAME_BYTES, ExactRestoreAuthorization, ExactRestoreCurrentOwner,
    ExactRestoreEngine, ExactRestoreError, ExactRestoreOwnerFence, RestoreOwnerProvider,
    SecureExactRestoreStore,
};

use super::config::Config;
use super::fixture::PeerApplier;
use super::http_wire::{MAX_RECOVERY_FRAME, Request, read_request, write_response};
use super::recovery::{RECOVERY_PATH, lease_response, recovery_response, valid_lease_request};

const EXACT_PREFIX: &str = "/v1/exact-restore/";

#[derive(Clone)]
struct SharedOwner(Arc<Mutex<ExactRestoreCurrentOwner>>);

impl RestoreOwnerProvider for SharedOwner {
    fn current_owner(&mut self) -> Result<ExactRestoreCurrentOwner, ExactRestoreError> {
        self.0
            .lock()
            .map(|owner| owner.clone())
            .map_err(|_| ExactRestoreError::OwnerUnavailable)
    }
}

struct PeerState {
    engine: ExactRestoreEngine<SecureExactRestoreStore, SharedOwner, PeerApplier>,
    owner: Arc<Mutex<ExactRestoreCurrentOwner>>,
    config: Config,
    lookup_unknown_left: bool,
    requests: usize,
}

pub(crate) fn run() -> Result<(), String> {
    let config = Config::from_env()?;
    let owner = config.owner()?;
    let owner_ref = Arc::new(Mutex::new(owner));
    std::fs::create_dir_all(&config.store_dir)
        .map_err(|error| format!("create exact store: {error}"))?;
    let store = SecureExactRestoreStore::open(&config.store_dir)
        .map_err(|error| format!("open exact store: {error}"))?;
    let output = config.store_dir.join("synthetic-output");
    let applier = if config.unsupported {
        PeerApplier::Unsupported
    } else {
        PeerApplier::synthetic(output, config.commit_unknown)
    };
    let engine = ExactRestoreEngine::new(
        store,
        SharedOwner(owner_ref.clone()),
        applier,
        config.transport.principal.clone(),
    )
    .map_err(|error| format!("create exact engine: {error}"))?;
    let listener = TcpListener::bind(&config.address)
        .map_err(|error| format!("bind {}: {error}", config.address))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("read bound address: {error}"))?;
    println!("READY {address}");
    std::io::stdout()
        .flush()
        .map_err(|error| error.to_string())?;
    let mut state = PeerState {
        engine,
        owner: owner_ref,
        config: config.clone(),
        lookup_unknown_left: config.lookup_unknown_once,
        requests: 0,
    };
    for stream in listener.incoming() {
        let stream = stream.map_err(|error| format!("accept: {error}"))?;
        state.requests += 1;
        let _ = serve_connection(stream, &mut state);
        if state
            .config
            .max_requests
            .is_some_and(|limit| state.requests >= limit)
        {
            break;
        }
    }
    Ok(())
}

fn serve_connection(mut stream: TcpStream, state: &mut PeerState) -> Result<(), String> {
    let request = read_request(&mut stream)?;
    let response = state.handle(&request);
    write_response(&mut stream, response.0, &response.1)?;
    Ok(())
}

impl PeerState {
    fn handle(&mut self, request: &Request) -> (u16, Vec<u8>) {
        if request.method != "POST" {
            return (405, Vec::new());
        }
        if !self.authenticated(request, request.path == RECOVERY_PATH) {
            return (401, Vec::new());
        }
        if request.path == RECOVERY_PATH {
            return self.handle_recovery(&request.body);
        }
        let Some(kind) = request.path.strip_prefix(EXACT_PREFIX) else {
            return (404, Vec::new());
        };
        let kind = match kind {
            "begin" => "exact_restore_begin_request",
            "chunk" => "exact_restore_chunk_request",
            "finish" => "exact_restore_finish_blob_request",
            "commit" => "exact_restore_commit_request",
            "lookup" => "exact_restore_lookup_request",
            _ => return (404, Vec::new()),
        };
        if request.body.len() > EXACT_RESTORE_MAX_FRAME_BYTES {
            return (413, Vec::new());
        }
        let Ok(frame) = serde_json::from_slice::<Value>(&request.body) else {
            return (400, Vec::new());
        };
        if frame["kind"].as_str() != Some(kind) {
            return (400, Vec::new());
        }
        let Some(correlation) = frame["correlation_id"].as_str() else {
            return (400, Vec::new());
        };
        let auth = match self.exact_authorization(&frame, correlation, request) {
            Ok(auth) => auth,
            Err(_) => return (403, Vec::new()),
        };
        let response = match self.engine.handle(&request.body, &auth) {
            Ok(response) => response,
            Err(_) => return (503, Vec::new()),
        };
        let mut body = response.body;
        if kind == "exact_restore_lookup_request" && self.lookup_unknown_left {
            self.lookup_unknown_left = false;
            body = lookup_unknown_body(body);
        }
        (response.status, body)
    }

    fn authenticated(&self, request: &Request, recovery: bool) -> bool {
        let bearer = request
            .headers
            .get("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|token| token == self.config.transport.token);
        if recovery {
            return bearer;
        }
        let caller = request
            .headers
            .get("x-sts2-caller-id")
            .is_some_and(|caller| caller == &self.config.transport.principal);
        bearer && caller
    }

    fn exact_authorization(
        &self,
        frame: &Value,
        correlation: &str,
        request: &Request,
    ) -> Result<ExactRestoreAuthorization, ExactRestoreError> {
        let header = |name: &str| request.headers.get(name).cloned().unwrap_or_default();
        let instance_id = header("x-sts2-instance-id");
        let session_id = header("x-sts2-session-id");
        let lease_id = header("x-sts2-lease-id");
        let lease_epoch = header("x-sts2-lease-epoch")
            .parse()
            .map_err(|_| ExactRestoreError::Unauthorized)?;
        let current = self.current_owner();
        if instance_id != current.fence.instance_id()
            || session_id != current.fence.session_id()
            || lease_id != current.fence.lease_id()
            || lease_epoch != current.fence.lease_epoch()
            || frame["correlation_id"].as_str() != Some(correlation)
        {
            return Err(ExactRestoreError::Unauthorized);
        }
        ExactRestoreAuthorization::new(
            self.config.transport.principal.clone(),
            correlation.to_owned(),
            instance_id,
            session_id,
            lease_id,
            lease_epoch,
        )
    }

    fn handle_recovery(&mut self, body: &[u8]) -> (u16, Vec<u8>) {
        if body.len() > MAX_RECOVERY_FRAME {
            return (413, Vec::new());
        }
        let Ok(frame) = serde_json::from_slice::<Value>(body) else {
            return (400, Vec::new());
        };
        let kind = frame["kind"].as_str().unwrap_or_default();
        if kind == "host_fence_request" {
            return (200, recovery_response(&frame, &self.current_owner()));
        }
        if matches!(
            kind,
            "lease_install_request" | "lease_renew_request" | "lease_revoke_request"
        ) {
            if !valid_lease_request(&frame) {
                return (403, Vec::new());
            }
            let response = lease_response(&frame, &self.current_owner());
            if kind == "lease_install_request" {
                self.update_owner_from_grant(&frame["payload"]["grant"]);
            }
            return (200, response);
        }
        (400, Vec::new())
    }

    fn current_owner(&self) -> ExactRestoreCurrentOwner {
        self.owner.lock().map_or_else(
            |poisoned| poisoned.into_inner().clone(),
            |owner| owner.clone(),
        )
    }

    fn update_owner_from_grant(&self, grant: &Value) {
        let boot = &grant["boot"];
        let fence = &grant["fence"];
        let lease = &grant["lease"];
        let gateway = &grant["gateway"];
        let Some(expires) = lease["expires_at"]
            .as_str()
            .and_then(parse_timestamp_millis)
        else {
            return;
        };
        let Ok(owner) = ExactRestoreOwnerFence::new(
            boot["deployment_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            boot["instance_id"].as_str().unwrap_or_default().to_owned(),
            boot["instance_incarnation"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            boot["boot_id"].as_str().unwrap_or_default().to_owned(),
            boot["authority_generation"].as_u64().unwrap_or_default(),
            fence["host_fence_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            fence["fence_generation"].as_u64().unwrap_or_default(),
            lease["lease_id"].as_str().unwrap_or_default().to_owned(),
            lease["lease_epoch"].as_u64().unwrap_or_default(),
            gateway["session_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            expires,
        ) else {
            return;
        };
        if let Ok(mut current) = self.owner.lock() {
            *current = ExactRestoreCurrentOwner {
                fence: owner,
                observed_at_millis: current_millis(),
            };
        }
    }
}

fn lookup_unknown_body(body: Vec<u8>) -> Vec<u8> {
    let Ok(mut value) = serde_json::from_slice::<Value>(&body) else {
        return body;
    };
    value["payload"]["result"] = Value::String(String::from("UNKNOWN"));
    value["payload"]["state"] = Value::String(String::from("UNKNOWN"));
    if let Some(payload) = value["payload"].as_object_mut() {
        payload.remove("receipt");
    }
    serde_json::to_vec(&value).unwrap_or(body)
}

fn parse_timestamp_millis(value: &str) -> Option<u64> {
    let year: u64 = value.get(0..4)?.parse().ok()?;
    let month: u64 = value.get(5..7)?.parse().ok()?;
    let day: u64 = value.get(8..10)?.parse().ok()?;
    let hour: u64 = value.get(11..13)?.parse().ok()?;
    let minute: u64 = value.get(14..16)?.parse().ok()?;
    let second: u64 = value.get(17..19)?.parse().ok()?;
    let millis = value
        .get(20..value.len().saturating_sub(1))
        .unwrap_or("")
        .parse::<u64>()
        .unwrap_or(0);
    let y = i64::try_from(year)
        .ok()?
        .saturating_sub(i64::from(month <= 2));
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let month = i64::try_from(month).ok()?;
    let day = i64::try_from(day).ok()?;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = u64::try_from(era * 146_097 + day_of_era - 719_468).ok()?;
    Some(
        days.saturating_mul(86_400_000)
            .saturating_add(hour * 3_600_000)
            .saturating_add(minute * 60_000)
            .saturating_add(second * 1_000)
            .saturating_add(millis),
    )
}

fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::{lookup_unknown_body, parse_timestamp_millis};

    #[test]
    fn timestamp_parser_handles_millisecond_precision() {
        assert_eq!(
            parse_timestamp_millis("2026-09-17T00:00:00.123Z"),
            Some(1_789_603_200_123)
        );
    }

    #[test]
    fn lookup_unknown_clears_restore_receipt() {
        let body = serde_json::to_vec(&serde_json::json!({
            "payload": {
                "result": "RESTORE_VERIFIED",
                "state": "RESTORE_VERIFIED",
                "receipt": {"operation_id": "receipt"}
            }
        }))
        .expect("fixture");
        let value: serde_json::Value =
            serde_json::from_slice(&lookup_unknown_body(body)).expect("response");
        assert_eq!(value["payload"]["state"], "UNKNOWN");
        assert!(value["payload"].get("receipt").is_none());
    }
}
