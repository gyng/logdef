/**
 * Stable row-major order in the shared 4x4 item atlas.
 *
 * Both DOM icons and diegetic room stock use this table. Keeping one
 * mapping prevents a content reorder from putting rope on a bamboo shelf.
 */
export const ITEM_CELLS: Readonly<Record<string, number>> = {
  "item.alloy": 0,
  "item.bamboo": 1,
  "item.charge_cells": 2,
  "item.darts": 3,
  "item.fiber": 4,
  "item.hand_lamp": 5,
  "item.meals": 6,
  "item.mechanisms": 7,
  "item.menders_kit": 8,
  "item.poles": 9,
  "item.porters_harness": 10,
  "item.resin_feedstock": 11,
  "item.rope": 12,
  "item.scrap": 13,
  "item.resonator_drums": 14,
};
