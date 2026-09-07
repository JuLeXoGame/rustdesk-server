"""Check security floors and the exact reviewed local backports (Python 3.11+)."""
import hashlib
import json
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def toml(path):
    return tomllib.loads(path.read_text(encoding="utf-8"))


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def version(value):
    return tuple(int(n) for n in value.split("+")[0].split("-")[0].split("."))


def local_patch(manifest, lock, name, path, file, digest):
    require(manifest["patch"]["crates-io"][name]["path"] == path, f"{name}: path patch missing")
    directory = (lock.parent / path).resolve()
    package = toml(directory / "Cargo.toml")["package"]
    entries = [p for p in toml(lock)["package"] if p["name"] == name and p["version"] == package["version"]]
    require(len(entries) == 1 and "source" not in entries[0], f"{name}: registry implementation is active")
    actual = hashlib.sha256((directory / file).read_bytes().replace(b"\r\n", b"\n")).hexdigest()
    require(actual == digest, f"{name}: backport changed; review it and update its regression tests")
    require((directory / "LICENSE").is_file(), f"{name}: upstream license missing")
    require(not package.get("build"), f"{name}: unexpected build script")


def check_lock(lock):
    floors = {"openssl": (0, 10, 80), "fuser": (0, 16, 0),
              "bytes": (1, 11, 1), "idna": (1, 0, 0)}
    for package in toml(lock)["package"]:
        name, current = package["name"], version(package["version"])
        if name in floors:
            require(current >= floors[name], f"{lock}: vulnerable {name} {current}")
        if name == "rand":
            safe = current < (0, 7, 0) or (0, 8, 6) <= current < (0, 9, 0) or (0, 9, 3) <= current < (0, 10, 0) or current >= (0, 10, 1)
            require(safe, f"{lock}: vulnerable rand {current}")
        require(name != "rpassword", f"{lock}: obsolete password reader has returned")
        if name == "atty" or (name == "glib" and (0, 15, 0) <= current < (0, 20, 0)):
            require("source" not in package, f"{lock}: unpatched registry {name} is active")


root_manifest = toml(ROOT / "Cargo.toml")
local_patch(root_manifest, ROOT / "Cargo.lock", "atty", "libs/atty-compat", "src/lib.rs",
            "ad54cd2c7eac460bb4501378a4b37c4c892e4d51785686477feaf8328e21ec48")
check_lock(ROOT / "Cargo.lock")

if root_manifest["package"]["name"] == "rustdesk":
    local_patch(root_manifest, ROOT / "Cargo.lock", "glib", "vendor/glib-0.18.5", "src/variant_iter.rs",
                "1bc30729e0bf70af261afa14bfd26592a726e57b9be48eaffb76cc47a4005582")
    for member in ("libs/portable", "libs/virtual_display"):
        require(member in root_manifest["workspace"]["members"], f"{member}: no longer a workspace member")
        require(not (ROOT / member / "Cargo.lock").exists(), f"{member}: obsolete nested lockfile returned")
else:
    manifest = toml(ROOT / "ui/Cargo.toml")
    lock = ROOT / "ui/Cargo.lock"
    check_lock(lock)
    local_patch(manifest, lock, "glib", "../vendor/glib-0.15.12", "src/variant_iter.rs",
                "d1695482580f36aeac2d00e5973780dd3d24d378da2a5428d758925080915da3")
    local_patch(manifest, lock, "phf_generator", "../vendor/phf_generator-0.8.0", "src/lib.rs",
                "1719af6423219c4e979267d2d8f5821dbf82670803d7acfb6ebdd53e799768b2")
    phf = toml(ROOT / "vendor/phf_generator-0.8.0/Cargo.toml")
    require(phf["dependencies"]["rand"]["version"] == "0.8.6", "PHF: rand security floor changed")
    npm = json.loads((ROOT / "ui/html/package-lock.json").read_text(encoding="utf-8"))
    require(version(npm["packages"]["node_modules/vite"]["version"]) >= (6, 4, 3), "Vite security floor not met")

print("Security dependency floors and reviewed local backports: OK")
