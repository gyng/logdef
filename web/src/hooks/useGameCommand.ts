import { useCallback } from "react";
import { getBridge } from "../bridge";

/**
 * Hook to send a GameCommand to the Rust engine.
 * Returns the parsed result (Ok or Error).
 */
export function useGameCommand() {
  return useCallback((command: Record<string, unknown>) => {
    const bridge = getBridge();
    const json = JSON.stringify(command);
    const resultJson = bridge.send_command(json);
    return JSON.parse(resultJson) as { Ok?: null; Error?: unknown };
  }, []);
}
