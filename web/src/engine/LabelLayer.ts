/**
 * Text, as absolutely-positioned DOM on top of the canvas.
 *
 * WebGL text means an atlas, a packer, and a kerning bug three weeks
 * from now. The label count here is small and bounded — a few dozen
 * room names and buffer counts — so the browser's text engine is both
 * faster to build and better at rendering than anything hand-rolled.
 *
 * Elements are pooled and only touched when their content or position
 * actually changes, so a steady frame costs no layout at all.
 */

export interface Label {
  key: string;
  text: string;
  x: number;
  y: number;
  /** Style hook: becomes the element's class alongside `label`. */
  variant?: string;
  /** Opacity 0..1. Used to fade labels out rather than pop them. */
  alpha?: number;
  /**
   * Hover text. The precision layer (`DECISIONS.md` §8) — never the
   * first thing a player sees, and never a number on the screen.
   */
  title?: string;
}

interface PooledLabel {
  element: HTMLDivElement;
  text: string;
  title: string;
  x: number;
  y: number;
  variant: string;
  alpha: number;
  seen: boolean;
}

export class LabelLayer {
  private readonly root: HTMLElement;
  private readonly pool = new Map<string, PooledLabel>();

  constructor(root: HTMLElement) {
    this.root = root;
  }

  /** Replace the visible set with `labels`. Unlisted keys are hidden. */
  sync(labels: readonly Label[]): void {
    for (const pooled of this.pool.values()) {
      pooled.seen = false;
    }

    for (const label of labels) {
      const variant = label.variant ?? "";
      const alpha = label.alpha ?? 1;
      let pooled = this.pool.get(label.key);

      if (!pooled) {
        const element = document.createElement("div");
        element.className = variant ? `label ${variant}` : "label";
        element.textContent = label.text;
        if (label.title) element.title = label.title;
        this.root.appendChild(element);
        pooled = {
          element,
          text: label.text,
          title: label.title ?? "",
          x: Number.NaN,
          y: Number.NaN,
          variant,
          alpha: Number.NaN,
          seen: true,
        };
        this.pool.set(label.key, pooled);
      }

      // Only write to the DOM when something actually differs; a
      // no-op assignment to `style.transform` still dirties layout.
      if (pooled.text !== label.text) {
        pooled.element.textContent = label.text;
        pooled.text = label.text;
      }
      if (pooled.variant !== variant) {
        pooled.element.className = variant ? `label ${variant}` : "label";
        pooled.variant = variant;
      }
      const title = label.title ?? "";
      if (pooled.title !== title) {
        pooled.element.title = title;
        pooled.title = title;
      }
      const x = Math.round(label.x);
      const y = Math.round(label.y);
      if (pooled.x !== x || pooled.y !== y) {
        pooled.element.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
        pooled.x = x;
        pooled.y = y;
      }
      if (pooled.alpha !== alpha) {
        pooled.element.style.opacity = alpha === 1 ? "" : String(alpha);
        pooled.alpha = alpha;
      }
      pooled.seen = true;
    }

    for (const [key, pooled] of this.pool) {
      if (pooled.seen) continue;
      pooled.element.remove();
      this.pool.delete(key);
    }
  }

  dispose(): void {
    for (const pooled of this.pool.values()) {
      pooled.element.remove();
    }
    this.pool.clear();
  }
}
