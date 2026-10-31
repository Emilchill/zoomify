#!/usr/bin/env python3
"""Build Zoomify as an importable Cinnabar WebAssembly component."""

from pathlib import Path
import os
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    wrapper = shlex.split(os.environ.get("CINNABAR_CARGO_WRAPPER", ""))
    cargo = [*wrapper, "cargo"]
    target = ROOT / "target"
    environment = dict(os.environ, CARGO_TARGET_DIR=str(target))
    subprocess.run(
        [
            *cargo,
            "build",
            "--locked",
            "--release",
            "-p",
            "cinnabar-mod-template",
            "--target",
            "wasm32-unknown-unknown",
        ],
        cwd=ROOT,
        env=environment,
        check=True,
    )
    core = target / "wasm32-unknown-unknown/release/cinnabar_mod_template.wasm"
    output = ROOT / "dist/zoomify.wasm"
    subprocess.run(
        [
            *cargo,
            "run",
            "--locked",
            "--release",
            "-p",
            "cinnabar-mod-template-pack",
            "--",
            str(core),
            str(output),
        ],
        cwd=ROOT,
        env=environment,
        check=True,
    )
    print(f"Built {output}")


if __name__ == "__main__":
    main()
