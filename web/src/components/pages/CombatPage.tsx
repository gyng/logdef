import { useCallback, useEffect, useRef, useState } from "react";
import { getBridge } from "../../bridge";
import type {
  EncounterSnapshot,
  HeroSnapshot,
  HudSnapshot,
  SoundEvent,
  TowerSnapshot,
} from "../../bridge/types";
import { t } from "../../i18n";
import { AudioManager } from "../../audio/AudioManager";
import { TowerViz } from "../organisms/TowerViz";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
  refreshPhase: () => void;
}

const CANVAS_W = 800;
const CANVAS_H = 400;
const TOWER_W = 60;
const SIM_FLOOR_HEIGHT = 80;
/** Time in ms to reach a full bow draw (1.0). Holding longer caps. */
const MAX_DRAW_MS = 900;
const GROUND_SCREEN_Y = CANVAS_H * 0.75;

export function CombatPage({ sendCommand, refreshPhase }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [hud, setHud] = useState<HudSnapshot | null>(null);
  const [tower, setTower] = useState<TowerSnapshot | null>(null);
  const [heroPosition, setHeroPosition] = useState<number>(0);
  // Refs mirror the latest state into the rAF loop without re-binding it.
  const towerRef = useRef<TowerSnapshot | null>(null);
  const heroPositionRef = useRef<number>(0);
  const aimWorldRef = useRef<{ x: number; y: number } | null>(null);
  const rafRef = useRef<number>(0);
  const lastTimeRef = useRef<number>(0);
  const tickRef = useRef<(() => void) | null>(null);

  const refreshPhaseRef = useRef(refreshPhase);
  useEffect(() => {
    refreshPhaseRef.current = refreshPhase;
  }, [refreshPhase]);

  const audioRef = useRef<AudioManager | null>(null);
  if (audioRef.current === null) {
    audioRef.current = new AudioManager();
  }

  useEffect(() => {
    lastTimeRef.current = performance.now();
    audioRef.current?.init();

    const gameLoop = () => {
      const bridge = getBridge();
      const now = performance.now();
      const dt = (now - lastTimeRef.current) / 1000;
      lastTimeRef.current = now;

      const soundsJson = bridge.tick(dt);
      if (audioRef.current) {
        try {
          const sounds = JSON.parse(soundsJson) as SoundEvent[];
          if (Array.isArray(sounds) && sounds.length > 0) {
            audioRef.current.play(sounds);
          }
        } catch {
          // ignore audio errors
        }
      }

      const hudData: HudSnapshot = JSON.parse(bridge.get_hud_state());
      setHud(hudData);

      // Tower viz needs the live runner / cache / rack state from the
      // tower snapshot. The bridge call is small and the prep flow
      // already accepts it at 30hz, so we mirror that here.
      try {
        const towerData: TowerSnapshot = JSON.parse(bridge.get_tower_state());
        setTower(towerData);
        towerRef.current = towerData;
        const heroData: HeroSnapshot = JSON.parse(bridge.get_hero_state());
        setHeroPosition(heroData.position);
        heroPositionRef.current = heroData.position;
      } catch {
        // bridge may not have tower/hero data on first frame
      }

      const encounterJson = bridge.get_encounter_state();
      const encounter: EncounterSnapshot | null = encounterJson ? JSON.parse(encounterJson) : null;

      const phase = bridge.get_phase();
      if (phase !== "Encounter") {
        refreshPhaseRef.current();
        return;
      }

      const ctx = canvasRef.current?.getContext("2d");
      if (ctx && encounter) {
        const towerData = towerRef.current;
        const heroPos = heroPositionRef.current;
        const draw = drawStartRef.current;
        const drawProgress =
          draw === null ? 0 : Math.min(1, (performance.now() - draw) / MAX_DRAW_MS);
        render(ctx, {
          encounter,
          hud: hudData,
          tower: towerData,
          heroPosition: heroPos,
          drawProgress,
          aimWorld: aimWorldRef.current,
        });
      }

      rafRef.current = requestAnimationFrame(gameLoop);
    };

    tickRef.current = gameLoop;
    rafRef.current = requestAnimationFrame(gameLoop);
    return () => cancelAnimationFrame(rafRef.current);
  }, []);

  // Compute the hero's screen position from the tower snapshot so the
  // aim direction is anchored at the visible hero, not a fixed point.
  const heroFloorIdx = tower?.balconies.find((b) => b.id === heroPosition)?.floor ?? 0;
  const heroSimY = (heroFloorIdx + 1) * SIM_FLOOR_HEIGHT;
  const heroScreenX = TOWER_W / 2;
  const heroScreenY = GROUND_SCREEN_Y - heroSimY;

  // Mouse-down → start the bow draw timer.
  const drawStartRef = useRef<number | null>(null);

  const aimAtMouse = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = ((e.clientX - rect.left) / rect.width) * CANVAS_W;
      const my = ((e.clientY - rect.top) / rect.height) * CANVAS_H;
      const dir = { x: mx - heroScreenX, y: my - heroScreenY };
      aimWorldRef.current = dir;
      sendCommand({ AimAt: { direction: dir } });
    },
    [sendCommand, heroScreenX, heroScreenY],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      // eslint-disable-next-line react-hooks/immutability -- ref intentionally mutated by mouse handler
      drawStartRef.current = performance.now();
      aimAtMouse(e);
    },
    [aimAtMouse],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      // Only re-aim while a drag is in progress to keep bridge calls
      // out of every idle frame. Mouse-down sets the initial aim too.
      if (drawStartRef.current === null) return;
      aimAtMouse(e);
    },
    [aimAtMouse],
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const start = drawStartRef.current;
      // eslint-disable-next-line react-hooks/immutability -- ref intentionally mutated by mouse handler
      drawStartRef.current = null;
      const drawMs = start === null ? MAX_DRAW_MS : performance.now() - start;
      const power = Math.min(1, Math.max(0.1, drawMs / MAX_DRAW_MS));
      aimAtMouse(e);
      sendCommand({ SetDrawPower: { power } });
      sendCommand("Fire");
    },
    [aimAtMouse, sendCommand],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Tab") {
        e.preventDefault();
        sendCommand("SwitchWeapon");
      } else if (e.key === "q" || e.key === "Q") {
        e.preventDefault();
        sendCommand("UseWeaponAbility");
      } else if (e.key === "e" || e.key === "E") {
        e.preventDefault();
        sendCommand("UseHeroSkill");
      }
    },
    [sendCommand],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="combat-page">
      <div className="combat-stage">
        {tower && (
          <TowerViz tower={tower} heroBalconyId={heroPosition} title="Supply chain (live)" />
        )}
        <canvas
          ref={canvasRef}
          width={CANVAS_W}
          height={CANVAS_H}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
          className="combat-canvas"
        />
      </div>
      {hud && (
        <div className="combat-hud">
          <span>
            {t("combat.wave", { current: hud.current_wave + 1, total: hud.total_waves })} |{" "}
          </span>
          <span>{t("combat.enemies", { count: hud.enemies_remaining })} | </span>
          <span>
            {t("combat.ammo", { ammo: hud.ammo_primary })} |{" "}
            {hud.active_weapon === "Primary"
              ? t("combat.weapon.primary")
              : t("combat.weapon.secondary")}{" "}
            |{" "}
          </span>
          <span>
            Q:{" "}
            {hud.weapon_ability_cooldown > 0
              ? `${hud.weapon_ability_cooldown.toFixed(1)}s`
              : "ready"}{" "}
            | E: {hud.hero_skill_cooldown > 0 ? `${hud.hero_skill_cooldown.toFixed(1)}s` : "ready"}{" "}
            |{" "}
          </span>
          <span>{t("combat.gold", { gold: hud.gold })} | </span>
          <span>{t("combat.tower", { hp: Math.round(hud.tower_hp_fraction * 100) })}</span>
        </div>
      )}
    </div>
  );
}

interface RenderContext {
  encounter: EncounterSnapshot;
  hud: HudSnapshot;
  tower: TowerSnapshot | null;
  heroPosition: number;
  drawProgress: number;
  aimWorld: { x: number; y: number } | null;
}

function render(ctx: CanvasRenderingContext2D, rctx: RenderContext) {
  const w = CANVAS_W;
  const h = CANVAS_H;
  const { encounter, hud, tower, heroPosition, drawProgress, aimWorld } = rctx;

  ctx.fillStyle = "#1e1a16";
  ctx.fillRect(0, 0, w, h);

  // Tower
  ctx.fillStyle = "#4a3d30";
  ctx.fillRect(0, 0, TOWER_W, h);

  // Tower HP bar
  const hpFrac = hud.tower_hp_fraction;
  ctx.fillStyle = hpFrac > 0.5 ? "#3daa5e" : hpFrac > 0.25 ? "#d4922a" : "#c9433a";
  ctx.fillRect(0, h - h * hpFrac, 8, h * hpFrac);

  // Ground line
  ctx.strokeStyle = "#3a3028";
  ctx.beginPath();
  ctx.moveTo(TOWER_W, h * 0.75);
  ctx.lineTo(w, h * 0.75);
  ctx.stroke();

  // Hero + companions on the tower wall — sim y for each fighter is
  // (floor + 1) * SIM_FLOOR_HEIGHT, mapped through the same screen-y
  // anchor used by projectiles so they line up visually.
  const groundScreenYLocal = h * 0.75;
  if (tower) {
    const heroBalcony = tower.balconies.find((b) => b.id === heroPosition);
    if (heroBalcony) {
      const sy = groundScreenYLocal - (heroBalcony.floor + 1) * SIM_FLOOR_HEIGHT;
      drawFighter(ctx, TOWER_W / 2, sy, "#ffd770", "hero");
    }
    for (const companion of tower.companions) {
      if (companion.position === null || companion.position === heroPosition) continue;
      const balcony = tower.balconies.find((b) => b.id === companion.position);
      if (!balcony) continue;
      const sy = groundScreenYLocal - (balcony.floor + 1) * SIM_FLOOR_HEIGHT;
      const color = companion.injured ? "#c9433a" : "#4a8fd4";
      drawFighter(ctx, TOWER_W / 2, sy, color, companion.name);
    }

    // Aim line — visible cue for what the player is aiming at
    if (aimWorld) {
      const heroSimY = heroBalcony ? (heroBalcony.floor + 1) * SIM_FLOOR_HEIGHT : 0;
      const heroSx = TOWER_W / 2;
      const heroSy = groundScreenYLocal - heroSimY;
      ctx.save();
      ctx.strokeStyle = "rgba(255, 215, 112, 0.4)";
      ctx.setLineDash([6, 4]);
      ctx.beginPath();
      ctx.moveTo(heroSx, heroSy);
      ctx.lineTo(heroSx + aimWorld.x, heroSy + aimWorld.y);
      ctx.stroke();
      ctx.restore();
    }
  }

  // Draw strength bar (visible while charging the bow)
  if (drawProgress > 0) {
    const barX = TOWER_W + 16;
    const barY = 16;
    const barW = 140;
    const barH = 10;
    ctx.fillStyle = "rgba(0,0,0,0.5)";
    ctx.fillRect(barX, barY, barW, barH);
    ctx.fillStyle = drawProgress >= 0.99 ? "#3daa5e" : "#d4922a";
    ctx.fillRect(barX, barY, barW * drawProgress, barH);
    ctx.strokeStyle = "#f5e6cf";
    ctx.strokeRect(barX, barY, barW, barH);
    ctx.fillStyle = "#f5e6cf";
    ctx.font = "11px Lora, serif";
    ctx.fillText(`Draw ${Math.round(drawProgress * 100)}%`, barX + barW + 8, barY + 9);
  }

  // Enemies
  for (const enemy of encounter.enemies) {
    const screenX = TOWER_W + (enemy.x / 800) * (w - TOWER_W);
    const screenY = h * 0.75 - 20;
    const size = enemy.archetype === "Armored" ? 20 : enemy.archetype === "Runner" ? 10 : 14;

    ctx.fillStyle =
      enemy.archetype === "Grunt"
        ? "#c9433a"
        : enemy.archetype === "Runner"
          ? "#4a8fd4"
          : "#8a8a8a";
    ctx.fillRect(screenX - size / 2, screenY - size, size, size);

    // HP bar
    ctx.fillStyle = "#3daa5e";
    ctx.fillRect(screenX - size / 2, screenY - size - 4, size * enemy.hp_fraction, 2);
  }

  // Projectiles — sim y is positive going up (ground at y=0). Map to
  // screen by anchoring the ground to h * 0.75 and subtracting the
  // sim height. This makes arrows visibly arc from the hero height
  // down to the enemies.
  ctx.fillStyle = "#e8a830";
  const groundScreenY = h * 0.75;
  for (const proj of encounter.projectiles) {
    const screenX = TOWER_W + (proj.x / 800) * (w - TOWER_W);
    const screenY = groundScreenY - proj.y;
    ctx.beginPath();
    ctx.arc(screenX, screenY, 3, 0, Math.PI * 2);
    ctx.fill();
  }

  // Wave info
  ctx.fillStyle = "#e8e0d4";
  ctx.font = "14px Lora, serif";
  ctx.fillText(
    t("combat.canvas.wave_line", {
      current: encounter.current_wave + 1,
      total: encounter.total_waves,
      enemies: encounter.enemies_remaining,
    }),
    TOWER_W + 10,
    20,
  );
}

/** Draw a small fighter glyph on the tower wall with a name label. */
function drawFighter(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  color: string,
  label: string,
) {
  ctx.save();
  // Body
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(x, y - 10, 5, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillRect(x - 3, y - 6, 6, 8);
  // Outline
  ctx.strokeStyle = "#1a1612";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.arc(x, y - 10, 5, 0, Math.PI * 2);
  ctx.stroke();
  ctx.strokeRect(x - 3, y - 6, 6, 8);
  // Label
  ctx.fillStyle = color;
  ctx.font = "9px Lora, serif";
  ctx.textAlign = "center";
  ctx.fillText(label, x, y + 12);
  ctx.restore();
}
