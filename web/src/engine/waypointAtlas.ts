/**
 * Stable row-major UV contract for the painted route-beat atlas.
 *
 * Content indices are assigned after the pack is sorted and are not an art
 * identity. Keep this map keyed by authored id so adding a waypoint cannot
 * silently repaint every beat after it.
 */
export const WAYPOINT_ART_IDS = [
  "waypoint.broken_funicular",
  "waypoint.cloud_cistern",
  "waypoint.fallen_carrier",
  "waypoint.field_kitchen",
  "waypoint.lantern_post",
  "waypoint.relay_orchard",
  "waypoint.seep_pool",
  "waypoint.signal_bridge",
  "waypoint.snare_thicket",
  "waypoint.tool_cradle",
  "waypoint.windfall_rig",
  "waypoint.wire_tangle",
] as const;

export type WaypointAtlasRect = readonly [x: number, y: number, width: number, height: number];

const CELLS = new Map<string, WaypointAtlasRect>(
  WAYPOINT_ART_IDS.map((id, index) => [
    id,
    [(index % 4) * 256, Math.floor(index / 4) * 192, 256, 192],
  ]),
);

export function waypointArtCell(id: string | undefined): WaypointAtlasRect | null {
  return id === undefined ? null : (CELLS.get(id) ?? null);
}
