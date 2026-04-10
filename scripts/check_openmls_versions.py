#!/usr/bin/env python3

"""Validate OpenMLS-family Cargo.lock entries against a pinned upstream tag."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
import tomllib
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator

DEFAULT_OPENMLS_REPO = "https://github.com/openmls/openmls"
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


@dataclass(frozen=True)
class UpstreamPackage:
    version: str


@dataclass(frozen=True)
class LockPackage:
    location: str
    package_name: str
    version: str
    source: str | None


def is_openmls_package(name: str) -> bool:
    return name == "ds-lib" or name.startswith("openmls")


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def iter_files(root: Path, filename: str) -> list[Path]:
    return sorted(path for path in root.rglob(filename) if "target" not in path.parts)


def run_git(args: list[str], cwd: Path | None = None) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def load_upstream_packages(openmls_repo_dir: Path) -> dict[str, UpstreamPackage]:
    packages: dict[str, UpstreamPackage] = {}

    for manifest_path in iter_files(openmls_repo_dir, "Cargo.toml"):
        package = load_toml(manifest_path).get("package")
        if not isinstance(package, dict):
            continue

        name = package.get("name")
        version = package.get("version")
        if not isinstance(name, str) or not isinstance(version, str):
            continue
        if not is_openmls_package(name):
            continue

        existing = packages.get(name)
        if existing is not None and existing.version != version:
            raise ValueError(
                f"Found multiple versions for upstream package {name!r}: "
                f"{existing.version} and {version}."
            )

        packages[name] = UpstreamPackage(version=version)

    if not packages:
        raise ValueError(
            f"No OpenMLS-family packages were found in upstream repo {openmls_repo_dir}."
        )

    return packages


def iter_lock_packages(
    secluso_root: Path, upstream_packages: dict[str, UpstreamPackage]
) -> Iterator[LockPackage]:
    for lockfile_path in iter_files(secluso_root, "Cargo.lock"):
        packages = load_toml(lockfile_path).get("package")
        if not isinstance(packages, list):
            continue

        for package in packages:
            if not isinstance(package, dict):
                continue

            package_name = package.get("name")
            version = package.get("version")
            source = package.get("source")
            if not isinstance(package_name, str) or not isinstance(version, str):
                continue
            if source is not None and not isinstance(source, str):
                continue
            if package_name not in upstream_packages:
                continue

            yield LockPackage(
                location=f"{lockfile_path}:{package_name}",
                package_name=package_name,
                version=version,
                source=source,
            )


def compare_lock_package(
    lock_package: LockPackage, upstream_package: UpstreamPackage
) -> list[str]:
    errors: list[str] = []

    if lock_package.version != upstream_package.version:
        errors.append(
            f"{lock_package.location} expected resolved version "
            f"{upstream_package.version}, found {lock_package.version}."
        )

    if lock_package.source is None:
        errors.append(
            f"{lock_package.location} has no source in Cargo.lock; expected {CRATES_IO_SOURCE}."
        )
    elif lock_package.source.startswith("git+"):
        errors.append(
            f"{lock_package.location} uses git source {lock_package.source!r}; "
            f"OpenMLS packages must resolve from crates.io."
        )
    elif lock_package.source != CRATES_IO_SOURCE:
        errors.append(
            f"{lock_package.location} uses unsupported source {lock_package.source!r}; "
            f"expected {CRATES_IO_SOURCE}."
        )

    return errors


@contextmanager
def openmls_repo_checkout(repo_url: str, tag: str) -> Iterator[Path]:
    temp_root = Path(tempfile.mkdtemp(prefix="openmls-tag-check-"))
    clone_dir = temp_root / "openmls"
    try:
        run_git(["clone", "--depth", "1", "--branch", tag, repo_url, str(clone_dir)])
        yield clone_dir
    finally:
        shutil.rmtree(temp_root, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Validate OpenMLS-family Cargo.lock entries in version/core against "
            "package versions from a single upstream OpenMLS tag."
        )
    )
    parser.add_argument(
        "--tag",
        required=True,
        help='OpenMLS git tag to compare against, e.g. "openmls-v0.8.1".',
    )
    parser.add_argument(
        "--expected-commit",
        help=(
            "Expected commit hash for the requested OpenMLS tag. "
            "If provided, the check fails when the tag resolves to a different commit."
        ),
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print every checked Cargo.lock entry, not just failures.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    secluso_root = Path(__file__).resolve().parent.parent
    if not secluso_root.is_dir():
        print(f"error: secluso root does not exist: {secluso_root}", file=sys.stderr)
        return 2

    try:
        with openmls_repo_checkout(DEFAULT_OPENMLS_REPO, args.tag) as repo_dir:
            resolved_commit = run_git(
                ["rev-parse", f"{args.tag}^{{commit}}"], cwd=repo_dir
            )
            if args.expected_commit and resolved_commit != args.expected_commit:
                print(
                    f"error: OpenMLS tag {args.tag} resolved to commit {resolved_commit}, "
                    f"expected {args.expected_commit}.",
                    file=sys.stderr,
                )
                return 1
            upstream_packages = load_upstream_packages(repo_dir)
    except (subprocess.CalledProcessError, ValueError, tomllib.TOMLDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        lock_packages = list(iter_lock_packages(secluso_root, upstream_packages))
    except (ValueError, tomllib.TOMLDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if not lock_packages:
        print(
            f"No OpenMLS-family Cargo.lock entries were found under {secluso_root}.",
            file=sys.stderr,
        )
        return 2

    errors: list[str] = []
    for lock_package in lock_packages:
        upstream_package = upstream_packages[lock_package.package_name]
        if args.verbose:
            print(
                f"checking {lock_package.location} against "
                f"{lock_package.package_name} {upstream_package.version}"
            )
        errors.extend(compare_lock_package(lock_package, upstream_package))

    if errors:
        print(
            f"OpenMLS lockfile check failed for {len(errors)} issue(s) across "
            f"{len(lock_packages)} Cargo.lock entry(s) checked against {args.tag}:",
            file=sys.stderr,
        )
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        f"OpenMLS lockfile check passed for {len(lock_packages)} Cargo.lock entry(s) "
        f"against {args.tag}."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
