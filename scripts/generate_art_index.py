"""Generate the human-readable shipped-art quick reference.

The manifest owns intent and declared layout, process_art owns source-to-output
packing, and ImageGen sidecars own generation provenance. This script joins
those facts with the files that actually exist; it does not introduce another
hand-maintained art manifest.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image

from process_art import (
    CREATURE_MOTION_SOURCES,
    CREATURE_SOURCE_CELLS,
    CREW_MOTION_SOURCES,
    ROOM_SHELL_SOURCE_CELLS,
    ROOM_SOURCE_CELLS,
    SOURCE_NAMES,
    WAYPOINT_SOURCE_CELLS,
)


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "art-manifest.json"
OUTPUT = ROOT / "docs" / "GENERATED-ART.md"

# A checkpoint proves the asset in its actual composited context, not merely
# that an output file exists. Keep this small: several assets can share the
# same whole-scene checkpoint where that scene genuinely exercises them.
CHECKPOINTS: dict[str, list[str]] = {
    "arrival": ["web/capture/ui-surfaces-arrival.png"],
    "paper-grain": ["web/capture/tower-early.png"],
    "wash-overlay": ["web/capture/tower-early.png"],
    "vignette": ["web/capture/tower-early.png"],
    "sky-dawn": ["web/capture/tower-early.png"],
    "sky-day": ["web/capture/tower-running.png"],
    "sky-night": ["web/capture/tower-night.png"],
    "scenery-sheet": ["web/capture/tower-early.png"],
    "parallax-canopy": ["web/capture/tower-early.png"],
    "parallax-ruins": ["web/capture/ruin-rich.png"],
    "parallax-drowned": ["web/capture/fx-water.png"],
    "parallax-coast": ["web/capture/ui-surfaces-arrival.png"],
    "terrain-doodads": ["web/capture/ruin-rich.png", "web/capture/ruin-stripped.png"],
    "room-interiors": ["web/capture/fx-room-states-unlabelled.png"],
    "room-shells": ["web/capture/fx-room-states-unlabelled.png"],
    "room-architecture": ["web/capture/fx-room-states-unlabelled.png"],
    "walker-frame": ["web/capture/fx-walker-gait-a.png", "web/capture/fx-walker-gait-b.png"],
    "roof-flora": ["web/capture/tower-early.png"],
    "waypoint-art": [
        "web/capture/waypoint-field_kitchen.png",
        "web/capture/waypoint-signal_bridge.png",
        "web/capture/waypoint-wire_tangle.png",
    ],
    "shaft-components": ["web/capture/fx-stairs.png", "web/capture/fx-elevator.png"],
    "crew-sheet": ["web/capture/crew-badges.png"],
    "crew-motion": ["web/capture/crew-badges.png", "web/capture/crew-motion-b.png"],
    "crew-portraits": [
        "web/capture/crew-portraits-1600x900.png",
        "web/capture/crew-portraits-1200x760.png",
    ],
    "creature-sheet": [
        "web/capture/creature-gallery.png",
        "web/capture/creatures-live-day.png",
    ],
    "item-icons": ["web/capture/ui-surfaces-chain.png"],
}


def rel_link(path: str) -> str:
    """A link from docs/ to a repository path, with readable slash form."""
    return "../" + path.replace("\\", "/")


def source_paths(asset: dict) -> list[str]:
    asset_id = asset["id"]
    if asset_id == "room-interiors":
        return [f"art-src/{name.replace('-keyed', '')}" for name in ROOM_SOURCE_CELLS]
    if asset_id == "room-shells":
        return [f"art-src/{name.replace('-keyed', '')}" for name in ROOM_SHELL_SOURCE_CELLS]
    if asset_id == "waypoint-art":
        return [
            f"art-src/{name.replace('-keyed', '')}" for name in WAYPOINT_SOURCE_CELLS
        ]
    if asset_id == "crew-motion":
        return [
            f"art-src/{name.replace('-keyed', '')}" for name in CREW_MOTION_SOURCES
        ]
    if asset_id == "creature-sheet":
        return [
            f"art-src/{name.replace('-keyed', '')}"
            for name in CREATURE_SOURCE_CELLS
        ]
    if asset_id == "creature-motion":
        return [
            f"art-src/{name.replace('-keyed', '')}"
            for name in CREATURE_MOTION_SOURCES
        ]
    name = SOURCE_NAMES.get(asset_id)
    return [f"art-src/{name}"] if name else []


def dimensions(path: Path) -> str:
    if not path.exists():
        return "missing"
    try:
        with Image.open(path) as image:
            return f"{image.width}×{image.height}"
    except OSError:
        return "non-raster"


def provenance(sources: list[str]) -> tuple[list[str], set[str], set[str]]:
    sidecars: list[str] = []
    models: set[str] = set()
    dates: set[str] = set()
    for source in sources:
        sidecar = str(Path(source).with_suffix(".json")).replace("\\", "/")
        path = ROOT / sidecar
        if not path.exists():
            continue
        sidecars.append(sidecar)
        record = json.loads(path.read_text(encoding="utf-8"))
        if record.get("model"):
            models.add(record["model"])
        if record.get("date"):
            dates.add(record["date"])
    return sidecars, models, dates


def linked(paths: list[str], label: str | None = None) -> str:
    if not paths:
        return "—"
    links = []
    for index, path in enumerate(paths, start=1):
        text = label if len(paths) == 1 and label else str(index)
        links.append(f"[{text}]({rel_link(path)})")
    return ", ".join(links)


def atlas_order(asset: dict) -> str:
    sheet = asset.get("sheet")
    if not sheet:
        return "—"
    cells = sheet.get("cells", [])
    layout = sheet.get("layout", "atlas")
    return f"{layout}: " + " · ".join(str(cell) for cell in cells)


def render() -> str:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    lines = [
        "# Generated art quick reference",
        "",
        "<!-- Generated by scripts/generate_art_index.py. Do not edit by hand. -->",
        "",
        "This is a joined view of the authoritative [art manifest](art-manifest.json), "
        "the ImageGen provenance sidecars in [`art-src`](../art-src), the packing rules in "
        "[`process_art.py`](../scripts/process_art.py), and the files actually shipped from "
        "[`web/public/art`](../web/public/art). A checkpoint is a live composited capture; “—” "
        "means the asset has no dedicated in-game proof yet.",
        "",
        "Regenerate with `python scripts/generate_art_index.py`; CI-style verification is "
        "`python scripts/generate_art_index.py --check`.",
        "",
        "## Asset inventory",
        "",
        "| Asset | Use | Source → shipped output | Actual dimensions / format | Prompt provenance | Live checkpoint |",
        "|---|---|---|---|---|---|",
    ]

    for asset in manifest["assets"]:
        asset_id = asset["id"]
        sources = source_paths(asset)
        out = asset.get("out")
        derived = asset.get("derive")
        source_text = linked(sources, "source")
        if derived:
            source_text = f"derived from `{derived['from']}` ({derived['op']})"
        output_text = f"[`{out}`]({rel_link(out)})" if out and (ROOT / out).exists() else f"`{out}` (missing)"
        source_output = f"{source_text} → {output_text}"

        source_dimensions = [dimensions(ROOT / source) for source in sources]
        source_actual = " + ".join(source_dimensions) if source_dimensions else "derived"
        actual = dimensions(ROOT / out) if out else "missing"
        suffix = Path(out).suffix.lstrip(".").upper() if out else str(asset.get("format", "?"))
        declared = asset.get("ship", "—")
        dimension_text = f"source {source_actual} → output {actual} {suffix} (declared {declared})"

        sidecars, models, dates = provenance(sources)
        prompt = linked(sidecars, "prompt sidecar")
        if sidecars:
            prompt += f"<br>{', '.join(sorted(models))}<br>{', '.join(sorted(dates))}"
        elif derived:
            prompt = "derived; see source asset provenance"
        else:
            prompt = "procedural or hand-authored; no ImageGen sidecar"

        checkpoints = [path for path in CHECKPOINTS.get(asset_id, []) if (ROOT / path).exists()]
        checkpoint_text = linked(checkpoints, "capture")
        use = str(asset.get("usedBy", "—")).replace("|", "\\|")
        lines.append(
            f"| `{asset_id}` | {use} | {source_output} | {dimension_text} | {prompt} | {checkpoint_text} |"
        )

    lines.extend(["", "## Atlas cell order", ""])
    for asset in manifest["assets"]:
        if asset.get("sheet"):
            lines.extend([f"### `{asset['id']}`", "", atlas_order(asset), ""])

    shipped = sorted(path.name for path in (ROOT / "web/public/art").iterdir() if path.is_file())
    declared = {Path(asset["out"]).name for asset in manifest["assets"] if asset.get("out")}
    extras = [name for name in shipped if name not in declared]
    lines.extend(
        [
            "## Audit notes",
            "",
            f"- Manifest assets: {len(manifest['assets'])}.",
            f"- Shipped files in `web/public/art`: {len(shipped)}.",
            "- Shipped outputs not directly declared by an asset: "
            + (", ".join(f"`{name}`" for name in extras) if extras else "none")
            + ". These should be intentional pipeline intermediates or derived files.",
            "- Actual dimensions are read from the current output files, so a mismatch against the declared ship size is visible here.",
            "- Full prompts live only in sidecars; this index links them instead of duplicating prompt bodies.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if the generated index is stale")
    args = parser.parse_args()
    rendered = render()
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            raise SystemExit(f"{OUTPUT.relative_to(ROOT)} is stale; regenerate it")
        return
    OUTPUT.write_text(rendered, encoding="utf-8")


if __name__ == "__main__":
    main()
