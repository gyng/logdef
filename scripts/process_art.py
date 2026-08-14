"""Derive shipping art from full-resolution ImageGen sources.

The manifest remains the source of truth for paths and prompt composition. Transparent
sheets are keyed with the imagegen skill's helper before this script is run; this script
only resizes, packs, derives, and records provenance.
"""

from __future__ import annotations

import json
import math
from collections import deque
from datetime import date
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageOps


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "docs" / "art-manifest.json"
SOURCE_DIR = ROOT / "art-src"
OUT_DIR = ROOT / "web" / "public" / "art"

SOURCE_NAMES = {
    "itch-cover": "itch-cover.png",
    "itch-banner": "itch-banner.png",
    "title": "title.png",
    "arrival": "arrival.png",
    "elegy": "elegy.png",
    "paper-grain": "paper-grain.png",
    "wash-overlay": "wash-overlay.png",
    "sky-dawn": "sky-dawn.png",
    "sky-day": "sky-day.png",
    "sky-dusk": "sky-dusk.png",
    "sky-night": "sky-night.png",
    "scenery-sheet": "scenery-sheet.png",
    "crew-sheet": "crew-sheet.png",
    "crew-portraits": "crew-portraits.png",
    "item-icons": "item-icons.png",
    "walker-frame": "walker-frame.png",
    "roof-flora": "roof-flora.png",
    "shaft-components": "shaft-components.png",
    "room-architecture": "room-architecture.png",
    "parallax-canopy": "parallax-canopy.png",
    "parallax-clearing": "parallax-clearing.png",
    "parallax-ruins": "parallax-ruins.png",
    "parallax-drowned": "parallax-drowned.png",
    "parallax-coast": "parallax-coast.png",
    "terrain-doodads": "terrain-doodads.png",
    "weapon-components": "weapon-components.png",
    "combat-fx": "combat-fx.png",
}

WEAPON_COMPONENT_CELLS = [
    "thorn_gun",
    "dart_battery",
    "lantern_mast",
    "root_ward",
    "tanglenet",
    "resonance_array",
]

COMBAT_FX_CELLS = [
    "thorn_projectile",
    "dart_projectile",
    "tanglenet_cast",
    "lantern_pulse",
    "root_ward_pulse",
    "resonance_wave",
    "creature_impact",
    "hull_impact",
    "room_breach",
    "shaft_sever",
    "mechanical_puff",
    "creature_fade",
]

WALKER_FRAME_CELLS = [
    "roof-lip",
    "flank-wall",
    "deck-plate",
    "hull-rib",
    "underside-beam",
    "stair-frame",
    "leg-strut",
    "leg-joint",
    "leg-foot",
    "planter-box",
    "plating-strip",
    "moss-vine-ledge",
]

ROOF_FLORA_CELLS = [
    "long-grass",
    "fern-and-herbs",
    "small-flowering-herb",
    "trailing-vine",
    "low-shrub",
    "seedlings-and-sensor-stakes",
    "groundcover",
    "dry-and-fresh-shoots",
]

WAYPOINT_SOURCE_CELLS = {
    "waypoints-a-keyed.png": [
        "broken_funicular",
        "cloud_cistern",
        "fallen_carrier",
        "field_kitchen",
    ],
    "waypoints-b-keyed.png": [
        "lantern_post",
        "relay_orchard",
        "seep_pool",
        "signal_bridge",
    ],
    "waypoints-c-keyed.png": [
        "snare_thicket",
        "tool_cradle",
        "windfall_rig",
        "wire_tangle",
    ],
}
WAYPOINT_CELLS = [cell for cells in WAYPOINT_SOURCE_CELLS.values() for cell in cells]

CREATURE_SOURCE_CELLS = {
    "creatures-a-keyed.png": [
        "skitter",
        "root_borer",
        "glean_crow",
        "canopy_leaper",
    ],
    "creatures-b-keyed.png": [
        "night_prowler",
        "mire_hulk",
        "thicket_mother",
        "feral_warden",
    ],
}
CREATURE_CELLS = [cell for cells in CREATURE_SOURCE_CELLS.values() for cell in cells]
CREATURE_WIDTHS = {
    "skitter": 44,
    "root_borer": 66,
    "glean_crow": 70,
    "canopy_leaper": 96,
    "night_prowler": 82,
    "mire_hulk": 116,
    "thicket_mother": 108,
    "feral_warden": 116,
}
CREATURE_MOTION_FRAMES = [
    "idle-approach-a",
    "idle-approach-b",
    "attack-a",
    "attack-b",
    "hurt",
    "dying",
    "leaving-a",
    "leaving-b",
]
CREATURE_MOTION_SOURCES = [
    f"creature-motion/{creature_id}-keyed.png" for creature_id in CREATURE_CELLS
]

CREW_MOTION_FRAMES = [
    "idle-a",
    "idle-b",
    "walk-a",
    "walk-b",
    "work-a",
    "work-b",
    "carry-a",
    "carry-b",
    "eat-a",
    "eat-b",
    "sleep-a",
    "sleep-b",
]
CREW_MOTION_SOURCES = [
    "crew-motion/wren-keyed.png",
    "crew-motion/odile-keyed.png",
    "crew-motion/bakri-keyed.png",
    "crew-motion/sena-keyed.png",
    "crew-motion/toma-keyed.png",
    "crew-motion/ilay-keyed.png",
    "crew-motion/rook-keyed.png",
    "crew-motion/mira-keyed.png",
]

# Image generation kept the requested rows but treated the grid as a visual
# guide rather than exact equal cells. These non-overlapping source regions
# isolate the twelve accepted components without slicing a roof rail into the
# wall beside it. They are specific to art-src/walker-frame.png by design;
# changing that source requires re-approving the extraction boxes visually.
WALKER_FRAME_SOURCE_RECTS = [
    (0, 0, 450, 365),
    (450, 0, 730, 365),
    (730, 0, 1270, 365),
    (1270, 0, 1450, 380),
    (0, 365, 455, 745),
    (455, 365, 710, 745),
    (710, 365, 1230, 745),
    (1230, 380, 1450, 745),
    (0, 745, 370, 1085),
    (370, 745, 750, 1085),
    (750, 745, 1160, 1085),
    (1160, 745, 1450, 1085),
]

ROOM_SOURCE_CELLS = {
    "rooms-a-keyed.png": [
        "heartseed",
        "salvage_rig",
        "bunk",
        "burner",
        "canteen",
        "cellwright",
        "cutter_arm",
        "fiber_comb",
    ],
    "rooms-b-keyed.png": [
        "fitter",
        "garden",
        "kitbench",
        "lantern_mast",
        "mill",
        "resonance_array",
        "resonator_works",
        "ropery",
    ],
    "rooms-c-keyed.png": [
        "storeroom",
        "sun_forge",
        "cell_bank",
        "dart_battery",
        "root_ward",
        "tanglenet",
        "thorn_gun",
        "thornwright",
    ],
}

# Room-specific architectural backings use the same destination rectangles as
# ROOM_TARGETS, but are generated in a different grouping. Keeping this map
# separate makes the three prompts manageable without coupling renderer UVs to
# source-sheet order.
ROOM_SHELL_SOURCE_CELLS = {
    "room-character-a-keyed.png": [
        "bunk",
        "burner",
        "canteen",
        "cellwright",
        "cutter_arm",
        "fiber_comb",
        "fitter",
        "garden",
    ],
    "room-character-b-keyed.png": [
        "kitbench",
        "lantern_mast",
        "mill",
        "resonance_array",
        "resonator_works",
        "ropery",
        "storeroom",
        "sun_forge",
    ],
    "room-character-c-keyed.png": [
        "heartseed",
        "salvage_rig",
        "cell_bank",
        "dart_battery",
        "root_ward",
        "tanglenet",
        "thorn_gun",
        "thornwright",
    ],
}
ROOM_SHELL_SOURCE_COLUMNS = 2
ROOM_SHELL_SOURCE_ROWS = 4

ROOM_TARGETS = {
    "heartseed": (0, 0, 384, 128),
    "salvage_rig": (384, 0, 384, 128),
    "bunk": (0, 128, 256, 128),
    "burner": (256, 128, 256, 128),
    "canteen": (512, 128, 256, 128),
    "cellwright": (768, 128, 256, 128),
    "cutter_arm": (0, 256, 256, 128),
    "fiber_comb": (256, 256, 256, 128),
    "fitter": (512, 256, 256, 128),
    "garden": (768, 256, 256, 128),
    "kitbench": (0, 384, 256, 128),
    "lantern_mast": (256, 384, 256, 128),
    "mill": (512, 384, 256, 128),
    "resonance_array": (768, 384, 256, 128),
    "resonator_works": (0, 512, 256, 128),
    "ropery": (256, 512, 256, 128),
    "storeroom": (512, 512, 256, 128),
    "sun_forge": (768, 512, 256, 128),
    "cell_bank": (0, 640, 128, 128),
    "dart_battery": (128, 640, 128, 128),
    "root_ward": (256, 640, 128, 128),
    "tanglenet": (384, 640, 128, 128),
    "thorn_gun": (512, 640, 128, 128),
    "thornwright": (640, 640, 128, 128),
}


def size(value: str) -> tuple[int, int]:
    width, height = value.lower().split("x")
    return int(width), int(height)


def save_webp(source: Path, target: Path, target_size: tuple[int, int], quality: int) -> None:
    image = Image.open(source).convert("RGB")
    ImageOps.fit(image, target_size, Image.Resampling.LANCZOS).save(
        target, "WEBP", quality=quality, method=6
    )


def seamless_paper(source: Path, target: Path) -> None:
    image = Image.open(source).convert("L")
    patch = ImageOps.fit(image, (512, 512), Image.Resampling.LANCZOS)
    tile = Image.new("L", (1024, 1024))
    tile.paste(patch, (0, 0))
    tile.paste(ImageOps.mirror(patch), (512, 0))
    tile.paste(ImageOps.flip(patch), (0, 512))
    tile.paste(ImageOps.flip(ImageOps.mirror(patch)), (512, 512))
    # Watercolour tooth does not need 256 distinct greys; fewer levels preserve the
    # multiplication effect and compress a noisy texture dramatically.
    tile.quantize(colors=48, dither=Image.Dither.FLOYDSTEINBERG).save(target, optimize=True)


def vignette(target: Path) -> None:
    width, height = 1024, 576
    alpha = Image.new("L", (width, height))
    pixels = alpha.load()
    for y in range(height):
        dy = (y - height / 2) / (height / 2)
        for x in range(width):
            dx = (x - width / 2) / (width / 2)
            radius = math.sqrt(dx * dx + dy * dy)
            # Fully clear through the inner 45%; smooth black toward the corners.
            t = max(0.0, min(1.0, (radius - 0.45) / (math.sqrt(2.0) - 0.45)))
            smooth = t * t * (3.0 - 2.0 * t)
            pixels[x, y] = round(220 * smooth)
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    image.putalpha(alpha)
    image.save(target, optimize=True)


def favicon(target: Path) -> None:
    """Raster companion to the hand-authored SVG browser icon.

    Keep this derived instead of maintaining a second visual design. Pillow
    draws the same few tab-scale shapes at high resolution, including the CRT,
    amber screen and the sprout that breaks its silhouette.
    """
    image = Image.new("RGBA", (512, 512), "#12301f")
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((0, 0, 511, 511), radius=112, fill="#12301f")
    draw.line((256, 214, 256, 98), fill="#b7cbb0", width=24)
    draw.polygon(((251, 132), (210, 120), (171, 77), (220, 64), (251, 132)), fill="#b7cbb0")
    draw.polygon(((263, 164), (302, 148), (313, 90), (273, 108), (263, 164)), fill="#b7cbb0")
    draw.rounded_rectangle((211, 422, 301, 461), radius=13, fill="#33453e")
    draw.rounded_rectangle((102, 205, 410, 435), radius=38, fill="#33453e")
    draw.rounded_rectangle((146, 245, 366, 395), radius=24, fill="#d9a25a")
    draw.line((168, 346, 344, 346), fill="#81704d", width=16)
    image.save(target, optimize=True)


def prop_columns(image: Image.Image, row: int) -> list[tuple[int, int]]:
    alpha = image.getchannel("A")
    width, height = image.size
    top, bottom = row * height // 2, (row + 1) * height // 2
    occupancy = []
    for x in range(width):
        column = alpha.crop((x, top, x + 1, bottom))
        histogram = column.histogram()
        occupancy.append(sum(histogram[17:]))

    # Image generators honour five columns approximately rather than mathematically.
    # Find the quietest real gap near each nominal divider instead of slicing through a
    # prop or merging a detached cable into its neighbour.
    boundaries = [0]
    radius = width // 12
    for index in range(1, 5):
        nominal = index * width // 5
        left, right = max(0, nominal - radius), min(width, nominal + radius)
        boundary = min(range(left, right), key=lambda x: (occupancy[x], abs(x - nominal)))
        boundaries.append(boundary)
    boundaries.append(width)
    return list(zip(boundaries, boundaries[1:]))


def pack_scenery(keyed: Path, target: Path) -> None:
    source = Image.open(keyed).convert("RGBA")
    atlas = Image.new("RGBA", (2048, 2048))
    cell_width, cell_height = 2048 // 5, 1024
    for row in range(2):
        row_top, row_bottom = row * source.height // 2, (row + 1) * source.height // 2
        for column, (left, right) in enumerate(prop_columns(source, row)):
            crop = source.crop((left, row_top, right, row_bottom))
            bbox = crop.getchannel("A").getbbox()
            if bbox is None:
                continue
            crop = crop.crop(bbox)
            crop.thumbnail((int(cell_width * 0.9), int(cell_height * 0.78)), Image.Resampling.LANCZOS)
            crop = remove_specks(crop, 300)
            x = column * cell_width + (cell_width - crop.width) // 2
            y = row * cell_height + int(cell_height * 0.9) - crop.height
            atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_rooms(target: Path) -> None:
    """Pack three generated 4x2 sheets into the renderer's width-aware room atlas."""
    atlas = Image.new("RGBA", (1024, 768))
    for filename, room_ids in ROOM_SOURCE_CELLS.items():
        source = Image.open(SOURCE_DIR / filename).convert("RGBA")
        source_cell_w = source.width / 4
        source_cell_h = source.height / 2
        for index, room_id in enumerate(room_ids):
            column, row = index % 4, index // 4
            # Half-pixel generator dimensions occur on non-standard ImageGen sizes;
            # round the shared dividers, never the room silhouette itself.
            left = round(column * source_cell_w)
            top = round(row * source_cell_h)
            right = round((column + 1) * source_cell_w)
            bottom = round((row + 1) * source_cell_h)
            crop = source.crop((left, top, right, bottom))
            bbox = crop.getchannel("A").getbbox()
            if bbox is None:
                raise ValueError(f"room source cell {filename}:{index} ({room_id}) is empty")
            crop = remove_specks(crop.crop(bbox), 160)
            x, y, width, height = ROOM_TARGETS[room_id]
            crop.thumbnail((width - 8, height - 8), Image.Resampling.LANCZOS)
            dest_x = x + (width - crop.width) // 2
            dest_y = y + height - crop.height - 3
            atlas.alpha_composite(crop, (dest_x, dest_y))

    if set(ROOM_TARGETS) != {
        room_id for room_ids in ROOM_SOURCE_CELLS.values() for room_id in room_ids
    }:
        raise ValueError("room source and target maps have drifted")
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def derive_room_wrecks(source_path: Path, target: Path) -> None:
    """Turn authored furnishings into stable collapsed wreck variants.

    The source identity remains legible, but each destination cell is a fixed
    broken composition rather than the renderer cutting an intact room apart
    differently on every draw.
    """
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", source.size)
    for room_index, (room_id, (x, y, width, height)) in enumerate(ROOM_TARGETS.items()):
        cell = source.crop((x, y, x + width, y + height))
        # Collapse the room into three overlapping authored fragments. The
        # arithmetic is id/order-stable and deliberately contains no RNG.
        fragment_width = max(1, width // 3)
        for fragment in range(3):
            left = fragment * fragment_width
            right = width if fragment == 2 else (fragment + 1) * fragment_width
            piece = cell.crop((left, 0, right, height))
            angle = (-9, 6, -4)[(room_index + fragment) % 3]
            piece = piece.rotate(angle, resample=Image.Resampling.BICUBIC, expand=True)
            # Pull colour toward damp, soot-dark wreckage without erasing the
            # room-specific amber/green/blue accents.
            shade = Image.new("RGBA", piece.size, (34, 43, 39, 0))
            shade.putalpha(piece.getchannel("A").point(lambda alpha: alpha * 2 // 5))
            piece = Image.alpha_composite(piece, shade)
            dest_x = x + left + ((room_index * 7 + fragment * 11) % 9) - 4
            dest_y = y + 18 + ((room_index * 13 + fragment * 17) % 24)
            atlas.alpha_composite(piece, (dest_x, dest_y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_room_shells(target: Path) -> None:
    """Pack room-specific full-cell architecture into the room UV layout.

    Each generated source cell is reduced to its connected keyed silhouette,
    then registered to the complete destination rectangle. Unlike furnishing
    sprites, these backings deliberately fill their room footprint: leaving a
    transparent border here would reveal the generic walker wall behind them.
    """
    atlas = Image.new("RGBA", (1024, 768))
    for filename, room_ids in ROOM_SHELL_SOURCE_CELLS.items():
        source = Image.open(SOURCE_DIR / filename).convert("RGBA")
        if source.width >= source.height:
            raise ValueError(
                f"room shell source {filename} must use the portrait 2x4 layout: {source.size}"
            )
        source_cell_w = source.width / ROOM_SHELL_SOURCE_COLUMNS
        source_cell_h = source.height / ROOM_SHELL_SOURCE_ROWS
        for index, room_id in enumerate(room_ids):
            column = index % ROOM_SHELL_SOURCE_COLUMNS
            row = index // ROOM_SHELL_SOURCE_COLUMNS
            crop = source.crop(
                (
                    round(column * source_cell_w),
                    round(row * source_cell_h),
                    round((column + 1) * source_cell_w),
                    round((row + 1) * source_cell_h),
                )
            )
            bbox = crop.getchannel("A").getbbox()
            if bbox is None:
                raise ValueError(
                    f"room shell source cell {filename}:{index} ({room_id}) is empty"
                )
            crop = remove_specks(crop.crop(bbox), 160)
            x, y, width, height = ROOM_TARGETS[room_id]
            atlas.alpha_composite(
                crop.resize((width, height), Image.Resampling.LANCZOS),
                (x, y),
            )

    source_ids = {
        room_id for room_ids in ROOM_SHELL_SOURCE_CELLS.values() for room_id in room_ids
    }
    if set(ROOM_TARGETS) != source_ids:
        raise ValueError("room shell source and target maps have drifted")
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_walker_frame(source_path: Path, target: Path) -> None:
    """Cut the generated 4x3 component sheet into a compact 128px-cell atlas."""
    source = Image.open(source_path).convert("RGBA")
    if source.size != (1450, 1085):
        raise ValueError(f"walker-frame source changed size: {source.size}")
    atlas = Image.new("RGBA", (512, 384))
    for index, (name, source_rect) in enumerate(
        zip(WALKER_FRAME_CELLS, WALKER_FRAME_SOURCE_RECTS, strict=True)
    ):
        column, row = index % 4, index // 4
        crop = source.crop(source_rect)
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"walker-frame source cell {index} ({name}) is empty")
        crop = remove_specks(crop.crop(bbox), 120)
        crop.thumbnail((120, 120), Image.Resampling.LANCZOS)
        x = column * 128 + (128 - crop.width) // 2
        y = row * 128 + (128 - crop.height) // 2
        atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_roof_flora(source_path: Path, target: Path) -> None:
    """Cut the generated 4x2 keyed plant sheet into stable 128px cells."""
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", (512, 256))
    for index, name in enumerate(ROOF_FLORA_CELLS):
        column, row = index % 4, index // 4
        crop = source.crop(
            (
                round(column * source.width / 4),
                round(row * source.height / 2),
                round((column + 1) * source.width / 4),
                round((row + 1) * source.height / 2),
            )
        )
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"roof-flora source cell {index} ({name}) is empty")
        crop = remove_specks(crop.crop(bbox), 100)
        crop.thumbnail((120, 120), Image.Resampling.LANCZOS)
        x = column * 128 + (128 - crop.width) // 2
        y = row * 128 + (128 - crop.height) // 2
        atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_waypoints(target: Path) -> None:
    """Pack three keyed 2x2 beat sheets into stable 256x192 id cells."""
    atlas = Image.new("RGBA", (1024, 576))
    seen: list[str] = []
    for filename, waypoint_ids in WAYPOINT_SOURCE_CELLS.items():
        source = Image.open(SOURCE_DIR / filename).convert("RGBA")
        for source_index, waypoint_id in enumerate(waypoint_ids):
            source_column, source_row = source_index % 2, source_index // 2
            # Image generation occasionally paints a thin visible grid rule
            # exactly on the requested cell boundary. Keep a small gutter out
            # of every source crop so that guide can never enter the atlas.
            gutter = max(4, round(min(source.width, source.height) * 0.008))
            crop = source.crop(
                (
                    round(source_column * source.width / 2) + gutter,
                    round(source_row * source.height / 2) + gutter,
                    round((source_column + 1) * source.width / 2) - gutter,
                    round((source_row + 1) * source.height / 2) - gutter,
                )
            )
            bbox = crop.getchannel("A").getbbox()
            if bbox is None:
                raise ValueError(f"waypoint source cell {filename}:{source_index} is empty")
            crop = remove_specks(crop.crop(bbox), 160)
            crop.thumbnail((244, 180), Image.Resampling.LANCZOS)
            target_index = len(seen)
            target_column, target_row = target_index % 4, target_index // 4
            x = target_column * 256 + (256 - crop.width) // 2
            y = target_row * 192 + 184 - crop.height
            atlas.alpha_composite(crop, (x, y))
            seen.append(waypoint_id)

    if seen != WAYPOINT_CELLS or len(set(seen)) != len(seen):
        raise ValueError("waypoint source sheets have drifted from their atlas order")
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_shaft_components(source_path: Path, target: Path) -> None:
    """Cut the generated 3x2 machinery sheet into six stable 256px cells."""
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", (768, 512))
    for index in range(6):
        column, row = index % 3, index // 3
        left = round(column * source.width / 3)
        top = round(row * source.height / 2)
        right = round((column + 1) * source.width / 3)
        bottom = round((row + 1) * source.height / 2)
        crop = source.crop((left, top, right, bottom))
        # The generated lift-head cables trespass slightly into the
        # counterweight cell's empty left gutter. The counterweight begins
        # well right of this crop, so discard that known source spill.
        if index == 5:
            crop = crop.crop((round(crop.width * 0.2), 0, crop.width, crop.height))
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"shaft-components source cell {index} is empty")
        crop = remove_specks(crop.crop(bbox), 5000)
        crop.thumbnail((244, 244), Image.Resampling.LANCZOS)
        x = column * 256 + (256 - crop.width) // 2
        y = row * 256 + (256 - crop.height) // 2
        atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_room_architecture(source_path: Path, target: Path) -> None:
    """Pack the fixed 4x4 room-shell and crown sheet without moving its artwork.

    Unlike isolated props, the first eight cells are registered architectural
    backings: their ceiling, wall, and deck edges must stay where they were
    authored.  Crop by the shared grid and resize the whole cell; an alpha-bbox
    crop would turn the shell's deliberate gutter into a different room shape.
    """
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", (1024, 1024))
    for index in range(16):
        column, row = index % 4, index // 4
        left = round(column * source.width / 4)
        top = round(row * source.height / 4)
        right = round((column + 1) * source.width / 4)
        bottom = round((row + 1) * source.height / 4)
        crop = source.crop((left, top, right, bottom))
        # Cell sixteen is a deliberate transparent reserve. Image generators
        # often helpfully invent one more component despite the prompt, so the
        # shipping contract clears it rather than trusting source compliance.
        if index == 15:
            continue
        if crop.getchannel("A").getbbox() is None:
            raise ValueError(f"room-architecture source cell {index} is empty")
        atlas.alpha_composite(
            crop.resize((256, 256), Image.Resampling.LANCZOS),
            (column * 256, row * 256),
        )

    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_terrain_doodads(source_path: Path, target: Path) -> None:
    """Cut the exact 4x4 terrain sheet into a compact transparent atlas."""
    source = Image.open(source_path).convert("RGBA")
    if source.width != source.height:
        raise ValueError(f"terrain-doodads source must be square: {source.size}")
    atlas = Image.new("RGBA", (1024, 1024))
    for index in range(16):
        column, row = index % 4, index // 4
        left = round(column * source.width / 4)
        top = round(row * source.height / 4)
        right = round((column + 1) * source.width / 4)
        bottom = round((row + 1) * source.height / 4)
        crop = source.crop((left, top, right, bottom))
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"terrain-doodads source cell {index} is empty")
        crop = remove_specks(crop.crop(bbox), 100)
        crop.thumbnail((244, 244), Image.Resampling.LANCZOS)
        x = column * 256 + (256 - crop.width) // 2
        y = row * 256 + 250 - crop.height
        atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_crew(source_path: Path, target: Path) -> None:
    """Cut the keyed eight-person source into stable 128x384 name cells."""
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", (1024, 384))
    for index in range(8):
        left = round(index * source.width / 8)
        right = round((index + 1) * source.width / 8)
        crop = source.crop((left, 0, right, source.height))
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"crew source cell {index} is empty")
        crop = remove_specks(crop.crop(bbox), 120)
        crop.thumbnail((120, 360), Image.Resampling.LANCZOS)
        x = index * 128 + (128 - crop.width) // 2
        y = 374 - crop.height
        atlas.alpha_composite(crop, (x, y))
    atlas.save(target, optimize=True)


def pack_equal_grid(
    source_path: Path,
    target: Path,
    columns: int,
    rows: int,
    cells: list[str],
) -> None:
    """Register a keyed exact grid into 256px row-major atlas cells."""
    if len(cells) != columns * rows:
        raise ValueError(f"{source_path.name}: {len(cells)} cells do not fill {columns}x{rows}")
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", (columns * 256, rows * 256))
    for index, name in enumerate(cells):
        column = index % columns
        row = index // columns
        crop = source.crop(
            (
                round(column * source.width / columns),
                round(row * source.height / rows),
                round((column + 1) * source.width / columns),
                round((row + 1) * source.height / rows),
            )
        )
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"{source_path.name}:{index} ({name}) is empty")
        crop = remove_specks(crop.crop(bbox), 120)
        crop.thumbnail((244, 244), Image.Resampling.LANCZOS)
        x = column * 256 + (256 - crop.width) // 2
        y = row * 256 + (256 - crop.height) // 2
        atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_crew_motion(target: Path) -> None:
    """Pack authored motion plus derived meal and sleep poses for eight people."""
    atlas = Image.new("RGBA", (1024, len(CREW_MOTION_FRAMES) * 192))
    for identity, filename in enumerate(CREW_MOTION_SOURCES):
        source = Image.open(SOURCE_DIR / filename).convert("RGBA")
        authored: list[Image.Image] = []
        for frame in range(8):
            column = frame % 4
            row = frame // 4
            left = round(column * source.width / 4)
            right = round((column + 1) * source.width / 4)
            top = round(row * source.height / 2)
            bottom = round((row + 1) * source.height / 2)
            crop = source.crop((left, top, right, bottom))
            bbox = crop.getchannel("A").getbbox()
            if bbox is None:
                raise ValueError(f"crew animation source {filename}:{frame} is empty")
            crop = remove_specks(crop.crop(bbox), 120)
            crop.thumbnail((120, 180), Image.Resampling.LANCZOS)
            authored.append(crop.copy())
            x = identity * 128 + (128 - crop.width) // 2
            y = frame * 192 + 188 - crop.height
            atlas.alpha_composite(crop, (x, y))

        # Eating borrows the authored hands-busy frames, settled lower behind
        # the procedural canteen table. Sleeping turns each person's neutral
        # silhouette onto the deck; identity, clothes and headwear survive.
        for frame, basis in enumerate((authored[4], authored[5]), start=8):
            # Preserve adult proportions: the renderer's table hides the lap,
            # so crop the lowest quarter instead of squashing a standing body.
            seated = basis.crop((0, 0, basis.width, round(basis.height * 0.76)))
            x = identity * 128 + (128 - seated.width) // 2
            y = frame * 192 + 188 - seated.height
            atlas.alpha_composite(seated, (x, y))
        for frame, basis in enumerate((authored[0], authored[1]), start=10):
            sleeper = basis.rotate(90, resample=Image.Resampling.BICUBIC, expand=True)
            sleeper.thumbnail((120, 88), Image.Resampling.LANCZOS)
            x = identity * 128 + (128 - sleeper.width) // 2
            y = frame * 192 + 188 - sleeper.height
            atlas.alpha_composite(sleeper, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_crew_portraits(source_path: Path, target: Path) -> None:
    """Cut the keyed 4x2 portrait source into stable 128px name cells."""
    source = Image.open(source_path).convert("RGBA")
    atlas = Image.new("RGBA", (512, 256))
    for index in range(8):
        column = index % 4
        row = index // 4
        left = round(column * source.width / 4)
        right = round((column + 1) * source.width / 4)
        top = round(row * source.height / 2)
        bottom = round((row + 1) * source.height / 2)
        crop = source.crop((left, top, right, bottom))
        bbox = crop.getchannel("A").getbbox()
        if bbox is None:
            raise ValueError(f"crew portrait source cell {index} is empty")
        crop = remove_specks(crop.crop(bbox), 120)
        crop.thumbnail((122, 122), Image.Resampling.LANCZOS)
        x = column * 128 + (128 - crop.width) // 2
        y = row * 128 + (128 - crop.height) // 2
        atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def connected_cutouts(source: Image.Image, count: int) -> list[Image.Image]:
    """Return the largest opaque components as isolated, left-to-right cutouts.

    Creature generations can honour a four-column composition while allowing a tail or
    leg to cross a nominal quarter boundary. Component isolation keeps the complete animal
    without importing any pixels from its neighbour.
    """
    source = source.convert("RGBA")
    width, height = source.size
    alpha = bytearray(source.getchannel("A").tobytes())
    seen = bytearray(width * height)
    components: list[list[int]] = []
    for origin, value in enumerate(alpha):
        if value <= 16 or seen[origin]:
            continue
        seen[origin] = 1
        queue = deque([origin])
        component: list[int] = []
        while queue:
            index = queue.popleft()
            component.append(index)
            x, y = index % width, index // width
            for neighbour in (
                index - 1 if x else -1,
                index + 1 if x + 1 < width else -1,
                index - width if y else -1,
                index + width if y + 1 < height else -1,
            ):
                if neighbour >= 0 and not seen[neighbour] and alpha[neighbour] > 16:
                    seen[neighbour] = 1
                    queue.append(neighbour)
        components.append(component)

    components = sorted(components, key=len, reverse=True)[:count]
    if len(components) != count:
        raise ValueError(f"expected {count} connected cutouts, found {len(components)}")

    cutouts: list[tuple[int, Image.Image]] = []
    pixels = source.load()
    for component in components:
        xs = [index % width for index in component]
        ys = [index // width for index in component]
        left, top, right, bottom = min(xs), min(ys), max(xs) + 1, max(ys) + 1
        cutout = Image.new("RGBA", (right - left, bottom - top))
        cutout_pixels = cutout.load()
        for index in component:
            x, y = index % width, index // width
            cutout_pixels[x - left, y - top] = pixels[x, y]
        cutouts.append((left, cutout))
    return [cutout for _, cutout in sorted(cutouts, key=lambda pair: pair[0])]


def pack_creatures(target: Path) -> None:
    """Pack two keyed four-animal sheets into stable 128x256 enemy cells."""
    atlas = Image.new("RGBA", (1024, 256))
    seen: list[str] = []
    for filename, creature_ids in CREATURE_SOURCE_CELLS.items():
        source = Image.open(SOURCE_DIR / filename).convert("RGBA")
        for creature_id, crop in zip(
            creature_ids, connected_cutouts(source, len(creature_ids)), strict=True
        ):
            crop.thumbnail((CREATURE_WIDTHS[creature_id], 166), Image.Resampling.LANCZOS)
            index = len(seen)
            x = index * 128 + (128 - crop.width) // 2
            # The renderer's sprite origin is 68% down its destination quad. Keep the
            # painted feet on that same line, with ample transparent room below.
            y = 174 - crop.height
            atlas.alpha_composite(crop, (x, y))
            seen.append(creature_id)

    if seen != CREATURE_CELLS or len(set(seen)) != len(seen):
        raise ValueError("creature source sheets have drifted from their atlas order")
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def pack_creature_motion(target: Path) -> None:
    """Pack eight keyed 4x2 species sheets as 8 species by 8 state rows."""
    atlas = Image.new("RGBA", (1024, 2048))
    for species, (creature_id, filename) in enumerate(
        zip(CREATURE_CELLS, CREATURE_MOTION_SOURCES, strict=True)
    ):
        source = Image.open(SOURCE_DIR / filename).convert("RGBA")
        for frame in range(len(CREATURE_MOTION_FRAMES)):
            column = frame % 4
            row = frame // 4
            left = round(column * source.width / 4)
            right = round((column + 1) * source.width / 4)
            top = round(row * source.height / 2)
            bottom = round((row + 1) * source.height / 2)
            crop = source.crop((left, top, right, bottom))
            bbox = crop.getchannel("A").getbbox()
            if bbox is None:
                raise ValueError(f"creature motion source {filename}:{frame} is empty")
            crop = remove_specks(crop.crop(bbox), 120)
            crop.thumbnail((CREATURE_WIDTHS[creature_id], 166), Image.Resampling.LANCZOS)
            x = species * 128 + (128 - crop.width) // 2
            y = frame * 256 + 174 - crop.height
            atlas.alpha_composite(crop, (x, y))
    atlas.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(target, optimize=True)


def remove_specks(image: Image.Image, minimum: int) -> Image.Image:
    """Drop small disconnected remnants introduced where a generated cell crosses a divider."""
    image = image.convert("RGBA")
    width, height = image.size
    alpha = bytearray(image.getchannel("A").tobytes())
    seen = bytearray(width * height)
    for origin, value in enumerate(alpha):
        if value <= 16 or seen[origin]:
            continue
        seen[origin] = 1
        queue = deque([origin])
        component = []
        while queue:
            index = queue.popleft()
            component.append(index)
            x, y = index % width, index // width
            for neighbour in (
                index - 1 if x else -1,
                index + 1 if x + 1 < width else -1,
                index - width if y else -1,
                index + width if y + 1 < height else -1,
            ):
                if neighbour >= 0 and not seen[neighbour] and alpha[neighbour] > 16:
                    seen[neighbour] = 1
                    queue.append(neighbour)
        if len(component) < minimum:
            for index in component:
                alpha[index] = 0
    cleaned = Image.frombytes("L", (width, height), bytes(alpha))
    image.putalpha(cleaned)
    return image


def quantize_atlas(path: Path) -> None:
    if not path.exists():
        return
    image = Image.open(path).convert("RGBA")
    image.quantize(
        colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG
    ).save(path, optimize=True)


def composed_prompt(manifest: dict, asset: dict) -> str:
    style_use = asset.get("style_use", "illustration")
    style = "" if style_use == "none" else manifest["style_sprite" if style_use == "sprite" else "style"]
    canon = manifest["subject_canon"]
    subject = asset.get("subject", "")
    for key, value in canon.items():
        if key.startswith("_"):
            continue
        subject = subject.replace("{" + key + "}", value)
    parts = {
        "style": style,
        "palette": manifest["palette"] if asset.get("palette", False) else "",
        "subject": subject,
        "composition": asset.get("composition", ""),
    }
    return "\n\n".join(parts[name] for name in manifest["compose"] if parts[name])


def write_sidecars(manifest: dict) -> None:
    by_id = {asset["id"]: asset for asset in manifest["assets"]}
    for asset_id, filename in SOURCE_NAMES.items():
        source = SOURCE_DIR / filename
        if not source.exists():
            continue
        record = {
            "model": "built-in imagegen (gpt-image-2)",
            "prompt": composed_prompt(manifest, by_id[asset_id]),
            "seed": None,
            "date": date.today().isoformat(),
            "source": str(source.relative_to(ROOT)).replace("\\", "/"),
        }
        source.with_suffix(".json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")

    room_asset = by_id.get("room-interiors")
    if room_asset is not None:
        room_prompt = composed_prompt(manifest, room_asset)
        for index, (keyed_name, cells) in enumerate(ROOM_SOURCE_CELLS.items(), start=1):
            source = SOURCE_DIR / keyed_name.replace("-keyed", "")
            if not source.exists():
                continue
            record = {
                "model": "built-in imagegen (gpt-image-2)",
                "prompt": (
                    f"{room_prompt}\n\nSource sheet {index} of 3, exact row-major cells: "
                    + ", ".join(cells)
                ),
                "seed": None,
                "date": date.today().isoformat(),
                "source": str(source.relative_to(ROOT)).replace("\\", "/"),
            }
            source.with_suffix(".json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )

    shell_asset = by_id.get("room-shells")
    if shell_asset is not None:
        shell_prompt = composed_prompt(manifest, shell_asset)
        for index, (keyed_name, cells) in enumerate(
            ROOM_SHELL_SOURCE_CELLS.items(), start=1
        ):
            source = SOURCE_DIR / keyed_name.replace("-keyed", "")
            if not source.exists():
                continue
            record = {
                "model": "built-in imagegen (gpt-image-2)",
                "prompt": (
                    f"{shell_prompt}\n\nSource sheet {index} of 3, exact row-major cells: "
                    + ", ".join(cells)
                ),
                "seed": None,
                "date": date.today().isoformat(),
                "source": str(source.relative_to(ROOT)).replace("\\", "/"),
            }
            source.with_suffix(".json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )

    waypoint_asset = by_id.get("waypoint-art")
    if waypoint_asset is not None:
        waypoint_prompt = composed_prompt(manifest, waypoint_asset)
        for index, (keyed_name, cells) in enumerate(WAYPOINT_SOURCE_CELLS.items(), start=1):
            source = SOURCE_DIR / keyed_name.replace("-keyed", "")
            if not source.exists():
                continue
            record = {
                "model": "built-in imagegen (gpt-image-2)",
                "prompt": (
                    f"{waypoint_prompt}\n\nSource sheet {index} of 3, exact row-major cells: "
                    + ", ".join(cells)
                ),
                "seed": None,
                "date": date.today().isoformat(),
                "source": str(source.relative_to(ROOT)).replace("\\", "/"),
            }
            source.with_suffix(".json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )

    creature_asset = by_id.get("creature-sheet")
    if creature_asset is not None:
        creature_prompt = composed_prompt(manifest, creature_asset)
        for index, (keyed_name, cells) in enumerate(
            CREATURE_SOURCE_CELLS.items(), start=1
        ):
            source = SOURCE_DIR / keyed_name.replace("-keyed", "")
            if not source.exists():
                continue
            record = {
                "model": "built-in imagegen (gpt-image-2)",
                "prompt": (
                    f"{creature_prompt}\n\nSource sheet {index} of 2, exact row-major cells: "
                    + ", ".join(cells)
                ),
                "seed": None,
                "date": date.today().isoformat(),
                "source": str(source.relative_to(ROOT)).replace("\\", "/"),
            }
            source.with_suffix(".json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )

    creature_motion_asset = by_id.get("creature-motion")
    if creature_motion_asset is not None:
        creature_motion_prompt = composed_prompt(manifest, creature_motion_asset)
        names = creature_motion_asset["sheet"]["cells"]
        for species, keyed_name in enumerate(CREATURE_MOTION_SOURCES):
            source = SOURCE_DIR / keyed_name.replace("-keyed", "")
            if not source.exists():
                continue
            record = {
                "model": "built-in imagegen (gpt-image-2)",
                "prompt": (
                    f"{creature_motion_prompt}\n\nSpecies sheet {species + 1} of 8: "
                    f"{names[species]}."
                ),
                "seed": None,
                "date": date.today().isoformat(),
                "source": str(source.relative_to(ROOT)).replace("\\", "/"),
            }
            source.with_suffix(".json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )

    crew_asset = by_id.get("crew-motion")
    if crew_asset is not None:
        crew_prompt = composed_prompt(manifest, crew_asset)
        names = crew_asset["sheet"]["cells"]
        for identity, keyed_name in enumerate(CREW_MOTION_SOURCES):
            source = SOURCE_DIR / keyed_name.replace("-keyed", "")
            if not source.exists():
                continue
            record = {
                "model": "built-in imagegen (gpt-image-2)",
                "prompt": (
                    f"{crew_prompt}\n\nIdentity sheet {identity + 1} of 8: {names[identity]}. "
                    "Exact row-major frames: " + ", ".join(CREW_MOTION_FRAMES)
                ),
                "seed": None,
                "date": date.today().isoformat(),
                "source": str(source.relative_to(ROOT)).replace("\\", "/"),
            }
            source.with_suffix(".json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    assets = {asset["id"]: asset for asset in manifest["assets"]}
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    for asset_id in ("itch-cover", "itch-banner", "title", "arrival", "elegy", "wash-overlay"):
        asset = assets[asset_id]
        save_webp(
            SOURCE_DIR / SOURCE_NAMES[asset_id],
            ROOT / asset["out"],
            size(asset["ship"]),
            asset.get("quality", manifest["defaults"]["quality"]),
        )

    cover = Image.open(SOURCE_DIR / SOURCE_NAMES["itch-cover"]).convert("RGB")
    ImageOps.fit(cover, (1200, 630), Image.Resampling.LANCZOS).save(
        OUT_DIR / "og-image.webp", "WEBP", quality=86, method=6
    )

    for asset_id in ("sky-dawn", "sky-day", "sky-dusk", "sky-night"):
        asset = assets[asset_id]
        save_webp(
            SOURCE_DIR / SOURCE_NAMES[asset_id],
            ROOT / asset["out"],
            size(asset["ship"]),
            asset.get("quality", manifest["defaults"]["quality"]),
        )

    for asset_id in (
        "parallax-canopy",
        "parallax-clearing",
        "parallax-ruins",
        "parallax-drowned",
        "parallax-coast",
    ):
        asset = assets[asset_id]
        save_webp(
            SOURCE_DIR / SOURCE_NAMES[asset_id],
            ROOT / asset["out"],
            size(asset["ship"]),
            asset.get("quality", manifest["defaults"]["quality"]),
        )

    seamless_paper(SOURCE_DIR / SOURCE_NAMES["paper-grain"], OUT_DIR / "paper-grain.png")
    vignette(OUT_DIR / "vignette.png")
    favicon(OUT_DIR / "favicon.png")
    keyed_scenery = OUT_DIR / "scenery-keyed.png"
    if keyed_scenery.exists():
        pack_scenery(keyed_scenery, OUT_DIR / "scenery-atlas.png")
    if all((SOURCE_DIR / filename).exists() for filename in ROOM_SOURCE_CELLS):
        pack_rooms(OUT_DIR / "rooms-atlas.png")
        derive_room_wrecks(OUT_DIR / "rooms-atlas.png", OUT_DIR / "room-wrecks-atlas.png")
    if all((SOURCE_DIR / filename).exists() for filename in ROOM_SHELL_SOURCE_CELLS):
        pack_room_shells(OUT_DIR / "room-shells-atlas.png")
    walker_frame = SOURCE_DIR / "walker-frame-keyed.png"
    if walker_frame.exists():
        pack_walker_frame(walker_frame, OUT_DIR / "walker-frame-atlas.png")
    roof_flora = SOURCE_DIR / "roof-flora-keyed.png"
    if roof_flora.exists():
        pack_roof_flora(roof_flora, OUT_DIR / "roof-flora-atlas.png")
    if all((SOURCE_DIR / filename).exists() for filename in WAYPOINT_SOURCE_CELLS):
        pack_waypoints(OUT_DIR / "waypoint-atlas.png")
    shaft_components = SOURCE_DIR / "shaft-components-keyed.png"
    if shaft_components.exists():
        pack_shaft_components(shaft_components, OUT_DIR / "shaft-components-atlas.png")
    room_architecture = SOURCE_DIR / "room-architecture-keyed.png"
    if room_architecture.exists():
        pack_room_architecture(room_architecture, OUT_DIR / "room-architecture-atlas.png")
    terrain_doodads = SOURCE_DIR / "terrain-doodads-keyed.png"
    if terrain_doodads.exists():
        pack_terrain_doodads(terrain_doodads, OUT_DIR / "terrain-doodads-atlas.png")
    weapon_components = SOURCE_DIR / "weapon-components-keyed.png"
    if weapon_components.exists():
        pack_equal_grid(
            weapon_components,
            OUT_DIR / "weapon-components-atlas.png",
            3,
            2,
            WEAPON_COMPONENT_CELLS,
        )
    combat_fx = SOURCE_DIR / "combat-fx-keyed.png"
    if combat_fx.exists():
        pack_equal_grid(combat_fx, OUT_DIR / "combat-fx-atlas.png", 4, 3, COMBAT_FX_CELLS)
    crew_sheet = SOURCE_DIR / "crew-sheet-keyed.png"
    if crew_sheet.exists():
        pack_crew(crew_sheet, OUT_DIR / "crew-atlas.png")
    if all((SOURCE_DIR / filename).exists() for filename in CREW_MOTION_SOURCES):
        pack_crew_motion(OUT_DIR / "crew-motion-atlas.png")
    if all((SOURCE_DIR / filename).exists() for filename in CREATURE_SOURCE_CELLS):
        pack_creatures(OUT_DIR / "creature-atlas.png")
    if all((SOURCE_DIR / filename).exists() for filename in CREATURE_MOTION_SOURCES):
        pack_creature_motion(OUT_DIR / "creature-motion-atlas.png")
    crew_portraits = SOURCE_DIR / "crew-portraits-keyed.png"
    if crew_portraits.exists():
        pack_crew_portraits(crew_portraits, OUT_DIR / "crew-portraits-atlas.png")
    for filename in ("items-atlas.png", "crew-atlas.png", "creature-atlas.png"):
        quantize_atlas(OUT_DIR / filename)
    write_sidecars(manifest)


if __name__ == "__main__":
    main()
