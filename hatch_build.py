from __future__ import annotations

import platform
import shutil
import subprocess
from pathlib import Path
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface
from packaging import tags


# Map OS to the shared library extension.
LIB_EXTENSIONS = {
    "Linux": "so",
    "Windows": "dll",
    "Darwin": "dylib",
}

# Map OS to the shared library prefix.
LIB_PREFIXES = {
    "Linux": "lib",
    "Windows": "",
    "Darwin": "lib",
}


class CustomHook(BuildHookInterface[Any]):
    target_dir = Path("vapoursynth/plugins/fftspectrum_rs")

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        build_data["pure_python"] = False
        build_data["tag"] = f"py3-none-{next(tags.platform_tags())}"

        os_name = platform.system()
        ext = LIB_EXTENSIONS[os_name]
        prefix = LIB_PREFIXES[os_name]
        crate_name = "fftspectrum_rs"
        lib_filename = f"{prefix}{crate_name}.{ext}"

        self.target_dir.mkdir(parents=True, exist_ok=True)

        subprocess.run(["cargo", "build", "--release"], check=True)

        built = Path("target") / "release" / lib_filename

        shutil.copy2(built, self.target_dir / lib_filename)

        manifest = self.target_dir / "manifest.vs"
        manifest.write_text(
            f"[VapourSynth Manifest V1]\n{prefix}{crate_name}\n",
            encoding="utf-8",
        )

    def finalize(self, version: str, build_data: dict[str, Any], artifact_path: str) -> None:
        shutil.rmtree(self.target_dir.parent, ignore_errors=True)
