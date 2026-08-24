use base64::{decode_config, URL_SAFE_NO_PAD};
use serde::Deserialize;
use sodiumoxide::crypto::sign;
use std::{
    collections::HashMap,
    env, fmt,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const TOKEN_PREFIX: &str = "rd1";
const EXPECTED_ISSUER: &str = "relaisdesk-api";
const EXPECTED_AUDIENCE: &str = "rustdesk-network";
const MAX_TOKEN_BYTES: usize = 8 * 1024;
const MAX_PAYLOAD_BYTES: usize = 4 * 1024;
const MAX_CLOCK_SKEW_SECONDS: i64 = 30;
const MAX_TOKEN_LIFETIME_SECONDS: i64 = 15 * 60;
const MAX_REPLAY_ENTRIES: usize = 100_000;
const MAX_REPLAY_ENTRIES_PER_DEVICE: usize = 512;
const REPLAY_CLEANUP_INTERVAL_SECONDS: i64 = 10;
const MAX_ACTIVE_REGISTRATIONS: usize = 100_000;
const ACTIVE_REGISTRATION_TTL_SECONDS: i64 = 35;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Claims {
    pub(crate) iss: String,
    pub(crate) aud: String,
    pub(crate) sub: String,
    pub(crate) tenant: String,
    pub(crate) role: String,
    pub(crate) jti: String,
    pub(crate) device_public_key: String,
    pub(crate) kid: String,
    pub(crate) iat: i64,
    pub(crate) nbf: i64,
    pub(crate) exp: i64,
    pub(crate) max_sessions: u32,
}

impl Claims {
    pub(crate) fn is_technician(&self) -> bool {
        self.role == "technician"
    }

    pub(crate) fn is_viewer(&self) -> bool {
        self.role == "viewer"
    }

    pub(crate) fn device_key(&self) -> Option<sign::PublicKey> {
        let bytes = decode_config(&self.device_public_key, URL_SAFE_NO_PAD).ok()?;
        sign::PublicKey::from_slice(&bytes)
    }

    pub(crate) fn is_current(&self) -> bool {
        now_seconds()
            .map(|now| now + MAX_CLOCK_SKEW_SECONDS >= self.nbf && now < self.exp)
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub(crate) struct AuthError(&'static str);

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

struct AuthConfig {
    required: bool,
    keys: HashMap<String, sign::PublicKey>,
}

#[derive(Default)]
struct ReplayCache {
    by_device: HashMap<String, HashMap<String, i64>>,
    total_entries: usize,
    last_cleanup: i64,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct RegistrationKey {
    role: String,
    scope: String,
    device_public_key: String,
}

struct ActiveRegistration {
    expires_at: i64,
    sequence: u64,
}

#[derive(Default)]
struct RegistrationCache {
    entries: HashMap<RegistrationKey, ActiveRegistration>,
    next_sequence: u64,
}

impl RegistrationCache {
    fn reserve(&mut self, claims: &Claims, now: i64) -> Result<(), AuthError> {
        self.entries.retain(|_, entry| entry.expires_at > now);

        let scope = if claims.is_technician() {
            &claims.tenant
        } else if claims.is_viewer() {
            &claims.sub
        } else {
            return Err(AuthError("invalid authorization role"));
        };
        let key = RegistrationKey {
            role: claims.role.clone(),
            scope: scope.clone(),
            device_public_key: claims.device_public_key.clone(),
        };

        if claims.is_technician() {
            let mut active: Vec<_> = self
                .entries
                .iter()
                .filter(|(existing, _)| {
                    existing.role == claims.role && existing.scope == claims.tenant
                })
                .map(|(existing, entry)| {
                    (
                        entry.sequence,
                        existing.device_public_key.as_str(),
                        existing == &key,
                    )
                })
                .collect();
            active.sort_unstable();
            if let Some(position) = active.iter().position(|entry| entry.2) {
                if position >= claims.max_sessions as usize {
                    return Err(AuthError("authorization device limit reached"));
                }
            } else if active.len() >= claims.max_sessions as usize {
                return Err(AuthError("authorization device limit reached"));
            }
        } else if self.entries.keys().any(|existing| {
            existing.role == claims.role
                && existing.scope == claims.sub
                && existing.device_public_key != claims.device_public_key
        }) {
            return Err(AuthError("viewer authorization is already in use"));
        }

        let expires_at = claims
            .exp
            .min(now.saturating_add(ACTIVE_REGISTRATION_TTL_SECONDS));
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.expires_at = expires_at;
            return Ok(());
        }
        if self.entries.len() >= MAX_ACTIVE_REGISTRATIONS {
            return Err(AuthError("authorization registration cache is full"));
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.entries.insert(
            key,
            ActiveRegistration {
                expires_at,
                sequence,
            },
        );
        Ok(())
    }
}

impl AuthConfig {
    fn from_env() -> Result<Self, AuthError> {
        sodiumoxide::init().map_err(|_| AuthError("cryptographic initialization failed"))?;
        let required = env_truthy("RELAISDESK_AUTH_REQUIRED");
        let mut keys = HashMap::new();

        if let Ok(value) = env::var("RELAISDESK_AUTH_PUBLIC_KEYS") {
            for entry in value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
            {
                let (kid, encoded) = entry
                    .split_once('=')
                    .ok_or(AuthError("invalid authorization public key list"))?;
                insert_public_key(&mut keys, kid.trim(), encoded.trim())?;
            }
        }

        if let Ok(encoded) = env::var("RELAISDESK_AUTH_PUBLIC_KEY") {
            if !encoded.trim().is_empty() {
                let kid = env::var("RELAISDESK_AUTH_KEY_ID")
                    .unwrap_or_else(|_| "relaisdesk-1".to_owned());
                insert_public_key(&mut keys, kid.trim(), encoded.trim())?;
            }
        }

        if required && keys.is_empty() {
            return Err(AuthError(
                "authorization is required but no public key is configured",
            ));
        }
        Ok(Self { required, keys })
    }

    fn enabled(&self) -> bool {
        !self.keys.is_empty()
    }
}

lazy_static::lazy_static! {
    static ref AUTH_CONFIG: Result<AuthConfig, AuthError> = AuthConfig::from_env();
    static ref REPLAY_NONCES: Mutex<ReplayCache> = Mutex::new(ReplayCache::default());
    static ref ACTIVE_REGISTRATIONS: Mutex<RegistrationCache> =
        Mutex::new(RegistrationCache::default());
}

pub(crate) fn validate_configuration() -> Result<bool, AuthError> {
    match AUTH_CONFIG.as_ref() {
        Ok(config) => Ok(config.enabled()),
        Err(err) => Err(AuthError(err.0)),
    }
}

pub(crate) fn reserve_registration(claims: &Option<Claims>) -> Result<(), AuthError> {
    let Some(claims) = claims else {
        return Ok(());
    };
    let now = now_seconds()?;
    ACTIVE_REGISTRATIONS
        .lock()
        .map_err(|_| AuthError("authorization registration cache unavailable"))?
        .reserve(claims, now)
}

pub(crate) fn verify_request(
    token: &str,
    timestamp: i64,
    nonce: &str,
    signature: &[u8],
    action: &str,
) -> Result<Option<Claims>, AuthError> {
    let config = AUTH_CONFIG.as_ref().map_err(|err| AuthError(err.0))?;
    if !config.enabled() {
        if config.required {
            return Err(AuthError("authorization is unavailable"));
        }
        return Ok(None);
    }
    if token.is_empty()
        || token.len() > MAX_TOKEN_BYTES
        || action.is_empty()
        || action.len() > 512
        || action.contains('\n')
        || nonce.len() < 16
        || nonce.len() > 64
        || !nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(AuthError("invalid authorization request"));
    }

    let now = now_seconds()?;
    if timestamp < now - MAX_CLOCK_SKEW_SECONDS || timestamp > now + MAX_CLOCK_SKEW_SECONDS {
        return Err(AuthError("authorization proof timestamp is invalid"));
    }

    let mut segments = token.split('.');
    let prefix = segments
        .next()
        .ok_or(AuthError("invalid authorization token"))?;
    let payload_segment = segments
        .next()
        .ok_or(AuthError("invalid authorization token"))?;
    let token_signature_segment = segments
        .next()
        .ok_or(AuthError("invalid authorization token"))?;
    if prefix != TOKEN_PREFIX || segments.next().is_some() {
        return Err(AuthError("invalid authorization token"));
    }

    let payload = decode_config(payload_segment, URL_SAFE_NO_PAD)
        .map_err(|_| AuthError("invalid authorization token"))?;
    if payload.is_empty() || payload.len() > MAX_PAYLOAD_BYTES {
        return Err(AuthError("invalid authorization token"));
    }
    let claims: Claims =
        serde_json::from_slice(&payload).map_err(|_| AuthError("invalid authorization token"))?;
    let token_signature_bytes = decode_config(token_signature_segment, URL_SAFE_NO_PAD)
        .map_err(|_| AuthError("invalid authorization token"))?;
    let token_signature = sign::Signature::from_bytes(&token_signature_bytes)
        .map_err(|_| AuthError("invalid authorization token"))?;
    let public_key = config
        .keys
        .get(&claims.kid)
        .ok_or(AuthError("unknown authorization key"))?;
    let signed_token = format!("{TOKEN_PREFIX}.{payload_segment}");
    if !sign::verify_detached(&token_signature, signed_token.as_bytes(), public_key) {
        return Err(AuthError("invalid authorization token signature"));
    }

    validate_claims(&claims, now)?;
    let device_key = claims
        .device_key()
        .ok_or(AuthError("invalid device public key"))?;
    let proof_signature = sign::Signature::from_bytes(signature)
        .map_err(|_| AuthError("invalid authorization proof"))?;
    let proof = proof_message(action, token, timestamp, nonce);
    if !sign::verify_detached(&proof_signature, proof.as_bytes(), &device_key) {
        return Err(AuthError("invalid authorization proof"));
    }

    remember_nonce(&claims.device_public_key, nonce, claims.exp, now)?;
    Ok(Some(claims))
}

fn validate_claims(claims: &Claims, now: i64) -> Result<(), AuthError> {
    if claims.iss != EXPECTED_ISSUER
        || claims.aud != EXPECTED_AUDIENCE
        || (claims.role != "technician" && claims.role != "viewer")
        || claims.sub.is_empty()
        || claims.sub.len() > 128
        || claims.tenant.is_empty()
        || claims.tenant.len() > 128
        || claims.jti.len() < 16
        || claims.jti.len() > 128
        || claims.kid.is_empty()
        || claims.kid.len() > 64
        || claims.max_sessions == 0
        || claims.max_sessions > 10_000
        || claims.iat > claims.nbf
        || claims.nbf > claims.exp
        || claims.exp - claims.iat > MAX_TOKEN_LIFETIME_SECONDS
        || now + MAX_CLOCK_SKEW_SECONDS < claims.nbf
        || now >= claims.exp
    {
        return Err(AuthError("invalid authorization claims"));
    }
    Ok(())
}

fn remember_nonce(
    device_public_key: &str,
    nonce: &str,
    expires_at: i64,
    now: i64,
) -> Result<(), AuthError> {
    let mut cache = REPLAY_NONCES
        .lock()
        .map_err(|_| AuthError("authorization replay cache unavailable"))?;
    if now - cache.last_cleanup >= REPLAY_CLEANUP_INTERVAL_SECONDS {
        let mut total_entries = 0usize;
        cache.by_device.retain(|_, nonces| {
            nonces.retain(|_, expiry| *expiry > now);
            total_entries = total_entries.saturating_add(nonces.len());
            !nonces.is_empty()
        });
        cache.total_entries = total_entries;
        cache.last_cleanup = now;
    }
    if cache.total_entries >= MAX_REPLAY_ENTRIES {
        return Err(AuthError("authorization replay cache is full"));
    }
    let device_nonces = cache
        .by_device
        .entry(device_public_key.to_owned())
        .or_default();
    if device_nonces.len() >= MAX_REPLAY_ENTRIES_PER_DEVICE {
        return Err(AuthError("device authorization replay cache is full"));
    }
    if device_nonces.insert(nonce.to_owned(), expires_at).is_some() {
        return Err(AuthError("authorization proof was replayed"));
    }
    cache.total_entries = cache.total_entries.saturating_add(1);
    Ok(())
}

fn insert_public_key(
    keys: &mut HashMap<String, sign::PublicKey>,
    kid: &str,
    encoded: &str,
) -> Result<(), AuthError> {
    if kid.is_empty()
        || kid.len() > 64
        || !kid
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(AuthError("invalid authorization key id"));
    }
    let bytes = decode_config(encoded, URL_SAFE_NO_PAD)
        .map_err(|_| AuthError("invalid authorization public key encoding"))?;
    let public_key = sign::PublicKey::from_slice(&bytes)
        .ok_or(AuthError("invalid authorization public key length"))?;
    if keys.insert(kid.to_owned(), public_key).is_some() {
        return Err(AuthError("duplicate authorization key id"));
    }
    Ok(())
}

fn proof_message(action: &str, token: &str, timestamp: i64, nonce: &str) -> String {
    format!("relaisdesk-proof-v1\n{action}\n{timestamp}\n{nonce}\n{token}")
}

fn now_seconds() -> Result<i64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|_| AuthError("system clock is invalid"))
}

fn env_truthy(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{now_seconds, proof_message, verify_request, Claims, RegistrationCache};
    use base64::{encode_config, URL_SAFE_NO_PAD};
    use serde_json::json;
    use sodiumoxide::crypto::sign;
    use std::env;

    fn claims(role: &str, subject: &str, tenant: &str, device: &str, max: u32) -> Claims {
        Claims {
            iss: "relaisdesk-api".to_owned(),
            aud: "rustdesk-network".to_owned(),
            sub: subject.to_owned(),
            tenant: tenant.to_owned(),
            role: role.to_owned(),
            jti: "0123456789abcdef".to_owned(),
            device_public_key: device.to_owned(),
            kid: "test-1".to_owned(),
            iat: 100,
            nbf: 100,
            exp: 1_000,
            max_sessions: max,
        }
    }

    #[test]
    fn proof_format_matches_client() {
        assert_eq!(
            proof_message("relay:abc", "rd1.payload.signature", 42, "nonce"),
            "relaisdesk-proof-v1\nrelay:abc\n42\nnonce\nrd1.payload.signature"
        );
    }

    #[test]
    fn signed_token_proof_and_replay_protection_work_together() {
        sodiumoxide::init().expect("sodiumoxide initialization");
        let (issuer_public, issuer_private) = sign::gen_keypair();
        let (device_public, device_private) = sign::gen_keypair();
        env::set_var("RELAISDESK_AUTH_REQUIRED", "Y");
        env::set_var(
            "RELAISDESK_AUTH_PUBLIC_KEYS",
            format!("test-1={}", encode_config(issuer_public.0, URL_SAFE_NO_PAD)),
        );

        let now = now_seconds().expect("valid clock");
        let payload = json!({
            "iss": "relaisdesk-api",
            "aud": "rustdesk-network",
            "sub": "LIC-TEST",
            "tenant": "LIC-TEST",
            "role": "technician",
            "jti": "0123456789abcdef",
            "device_public_key": encode_config(device_public.0, URL_SAFE_NO_PAD),
            "kid": "test-1",
            "iat": now,
            "nbf": now,
            "exp": now + 300,
            "max_sessions": 2
        });
        let payload_segment = encode_config(payload.to_string(), URL_SAFE_NO_PAD);
        let signed_token = format!("rd1.{payload_segment}");
        let token_signature = sign::sign_detached(signed_token.as_bytes(), &issuer_private);
        let token = format!(
            "{signed_token}.{}",
            encode_config(token_signature.to_bytes(), URL_SAFE_NO_PAD)
        );
        let nonce = "0123456789abcdef0123456789abcdef";
        let action = "punch:123456789";
        let proof = proof_message(action, &token, now, nonce);
        let signature = sign::sign_detached(proof.as_bytes(), &device_private);

        let verified = verify_request(&token, now, nonce, &signature.to_bytes(), action)
            .expect("valid authorization")
            .expect("authorization enabled");
        assert_eq!(verified.tenant, "LIC-TEST");
        assert!(verify_request(&token, now, nonce, &signature.to_bytes(), action).is_err());
    }

    #[test]
    fn registration_reservations_enforce_quotas_atomically() {
        let mut cache = RegistrationCache::default();
        cache
            .reserve(
                &claims("technician", "tech-a", "tenant-a", "device-a", 2),
                100,
            )
            .expect("first technician device");
        cache
            .reserve(
                &claims("technician", "tech-a", "tenant-a", "device-b", 2),
                100,
            )
            .expect("second technician device");
        assert!(cache
            .reserve(
                &claims("technician", "tech-a", "tenant-a", "device-c", 2),
                100
            )
            .is_err());

        assert!(cache
            .reserve(
                &claims("technician", "tech-a", "tenant-a", "device-b", 1),
                101
            )
            .is_err());
        cache
            .reserve(
                &claims("technician", "tech-a", "tenant-b", "device-c", 1),
                101,
            )
            .expect("a separate tenant has its own quota");

        cache
            .reserve(
                &claims("viewer", "viewer-a", "tenant-a", "viewer-device-a", 1),
                101,
            )
            .expect("first viewer device");
        assert!(cache
            .reserve(
                &claims("viewer", "viewer-a", "tenant-a", "viewer-device-b", 1),
                101
            )
            .is_err());
        cache
            .reserve(
                &claims("viewer", "viewer-b", "tenant-a", "viewer-device-b", 1),
                101,
            )
            .expect("another viewer has an independent reservation");
    }
}
