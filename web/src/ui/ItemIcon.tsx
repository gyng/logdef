import type { CSSProperties } from "react";

import type { ItemInfo } from "../bridge/types";
import { ITEM_CELLS } from "../engine/itemAtlas";

interface Props {
  item: Pick<ItemInfo, "id" | "name"> | null | undefined;
  className?: string;
  /** The surrounding control or adjacent text already names the item. */
  decorative?: boolean;
}

/** A single cell from the shared 4x4 item atlas. */
export function ItemIcon({ item, className = "", decorative = false }: Props) {
  const cell = item ? ITEM_CELLS[item.id] : undefined;
  const classes = `item-icon${cell === undefined ? " missing" : ""}${className ? ` ${className}` : ""}`;
  const style: CSSProperties | undefined =
    cell === undefined
      ? undefined
      : {
          backgroundPosition: `${(cell % 4) * (100 / 3)}% ${Math.floor(cell / 4) * (100 / 3)}%`,
        };

  return (
    <span
      className={classes}
      style={style}
      role={decorative ? undefined : "img"}
      aria-hidden={decorative || undefined}
      aria-label={decorative ? undefined : (item?.name ?? "Unknown material")}
    />
  );
}
