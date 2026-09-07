# RelaisDesk server: build and authorization

This is a RustDesk Server Community fork, not the RustDesk Pro product.
`hbbs` handles rendezvous; `hbbr` relays sessions when direct connectivity is
unavailable. The changes are described in [RELAISDESK_FORK.md](../RELAISDESK_FORK.md).

## Corresponding source and build

```sh
git clone --recursive --branch relaisdesk/authorization https://github.com/JuLeXoGame/rustdesk-server.git
cd rustdesk-server
git checkout <server-commit-from-the-release>
git submodule update --init --recursive
git rev-parse HEAD
git submodule status --recursive
docker build -f Dockerfile.relaisdesk -t relaisdesk-server:local .
```

Replace the placeholder with the immutable commit for the deployed binaries.
The [Dockerfile](../Dockerfile.relaisdesk) pins the build environment and lists
native dependencies. The [CI workflow](../.github/workflows/relaisdesk-ci.yml)
also runs `cargo test --locked --lib --bin hbbr` and builds both release
binaries with Rust 1.90.0. Do not substitute upstream `hbb_common` for the
modified, pinned submodule.

Keep the exact server/submodule commits, scripts, tool versions and artifact
SHA-256 with each release. A documentation-only update does not change the
source provenance of an already deployed binary. Source ZIPs without
submodules are incomplete; use a recursive clone or include the full submodule
source. Retain [LICENSE](../LICENSE) and the modification notice.

## Configuration

```text
RELAISDESK_AUTH_REQUIRED=Y
RELAISDESK_AUTH_PUBLIC_KEYS=relaisdesk-1=<base64url-Ed25519-public-key>
SERVER_SOURCE_URL=https://github.com/JuLeXoGame/rustdesk-server/tree/<deployed-commit>
```

The [Compose example](../docker-compose.yml) maps `SERVER_SOURCE_URL` to the
binary's `RELAISDESK_SOURCE_URL`. Use real values, not these placeholders.
Only the **public** authorization key is installed in `hbbs` and `hbbr`.
The API's private signing key, RustDesk transport identity, device proof key
and release-manifest signing key are separate credentials.

Required authorization without valid keys fails at startup. Compatibility
mode requires both `RELAISDESK_AUTH_REQUIRED=N` and an empty public-key list;
setting `N` alone does not disable verification when keys remain configured.
Never expose an unprotected compatibility deployment as a paid access control.

The sample image uses UID 10001 and `/data`; arrange bind-mount permissions
and backups before deployment. It is not a drop-in instruction to overwrite
an existing `/opt/rustdesk` configuration. Ports 21118/21119 (WebSocket) are
not published by this example.

## Protocol and rotation

Tokens are `rd1.<base64url-JSON>.<base64url-signature>`. The Ed25519 signature
covers `rd1.<base64url-JSON>`. Claims include issuer/audience, tenant, role,
subject, device public key, key ID, timestamps and simultaneous-session limit.
The server also verifies a device proof bound to the action, token, timestamp
and nonce. The implementation and exact checks are in
[src/relaisdesk_auth.rs](../src/relaisdesk_auth.rs) and the modified protocol
submodule; clients must implement those checks, not just copy a TOML key.

For rotation, configure old and new public keys on **both** servers:

```text
RELAISDESK_AUTH_PUBLIC_KEYS=relaisdesk-1=<old-public>,relaisdesk-2=<new-public>
```

Restart the servers to load the changed environment, then switch API issuance
to the new key ID. Keep the old public key until every old token has expired,
including configured clock tolerance. Remove it and restart both servers.
Test in staging and plan the restart impact; this is not a claim of seamless
live reload. Do not rotate the RustDesk transport identity to perform this
authorization-key rotation.

The service must offer corresponding source to network users under AGPLv3.
No certification, anonymity guarantee or blanket compliance is implied by
publication. Report vulnerabilities through [SECURITY.md](../SECURITY.md).
