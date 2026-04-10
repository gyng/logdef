import { useCallback, useEffect, useRef, useState } from "react";
import { getBridge } from "../../bridge";
import type { EncounterSnapshot, HudSnapshot } from "../../bridge/types";
import { t } from "../../i18n";

interface Props {
  sendCommand: (cmd: Record<string, unknown> | string) => { Ok?: null; Error?: unknown };
  refreshPhase: () => void;
}

const CANVAS_W = 800;
const CANVAS_H = 400;
const TOWER_W = 60;

export function CombatPage({ sendCommand, refreshPhase }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [hud, setHud] = useState<HudSnapshot | null>(null);
  const rafRef = useRef<number>(0);
  const lastTimeRef = useRef<number>(0);
  const tickRef = useRef<(() => void) | null>(null);

  const refreshPhaseRef = useRef(refreshPhase);
  useEffect(() => {
    refreshPhaseRef.current = refreshPhase;
  }, [refreshPhase]);

  useEffect(() => {
    lastTimeRef.current = performance.now();

    const gameLoop = () => {
      const bridge = getBridge();
      const now = performance.now();
      const dt = (now - lastTimeRef.current) / 1000;
      lastTimeRef.current = now;

      bridge.tick(dt);

      const hudData: HudSnapshot = JSON.parse(bridge.get_hud_state());
      setHud(hudData);

      const encounterJson = bridge.get_encounter_state();
      const encounter: EncounterSnapshot | null = encounterJson ? JSON.parse(encounterJson) : null;

      const phase = bridge.get_phase();
      if (phase !== "Encounter") {
        refreshPhaseRef.current();
        return;
      }

      const ctx = canvasRef.current?.getContext("2d");
      if (ctx && encounter) {
        render(ctx, encounter, hudData);
      }

      rafRef.current = requestAnimationFrame(gameLoop);
    };

    tickRef.current = gameLoop;
    rafRef.current = requestAnimationFrame(gameLoop);
    return () => cancelAnimationFrame(rafRef.current);
  }, []);

  const handleClick = useCallback(() => {
    sendCommand("Fire");
  }, [sendCommand]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Tab") {
        e.preventDefault();
        sendCommand("SwitchWeapon");
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
      <canvas
        ref={canvasRef}
        width={CANVAS_W}
        height={CANVAS_H}
        onClick={handleClick}
        className="combat-canvas"
      />
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
          <span>{t("combat.gold", { gold: hud.gold })} | </span>
          <span>{t("combat.tower", { hp: Math.round(hud.tower_hp_fraction * 100) })}</span>
        </div>
      )}
    </div>
  );
}

function render(ctx: CanvasRenderingContext2D, encounter: EncounterSnapshot, hud: HudSnapshot) {
  const w = CANVAS_W;
  const h = CANVAS_H;

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

  // Projectiles
  ctx.fillStyle = "#e8a830";
  for (const proj of encounter.projectiles) {
    const screenX = TOWER_W + (proj.x / 800) * (w - TOWER_W);
    const screenY = h * 0.75 - 15;
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
