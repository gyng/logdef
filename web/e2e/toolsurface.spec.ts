import { expect, test } from "@playwright/test";

/**
 * **Every tool, called at least once, twice.**
 *
 * The dogfood plays a run and therefore exercises the tools a *sensible*
 * agent reaches for. That leaves the rest untouched: `understory_chain`
 * had never been invoked by anything, and neither had half the verbs
 * added for M6. A tool that has never run is where a typo, a bad cast or
 * a wrong field name sits — and none of them show up in a typecheck,
 * because `a.floor as number` type-checks perfectly and throws at
 * runtime on `undefined`.
 *
 * Two passes, and the second is the interesting one:
 *
 * 1. **Plausible arguments**, synthesised from each tool's own
 *    `inputSchema`. Refusals are fine and expected — a tower on turn one
 *    cannot afford anything. What is not fine is an exception.
 * 2. **No arguments at all**, which is what a model does when it has
 *    misread a schema. Every tool has to answer that with a refusal
 *    rather than a stack trace, because `__webmcp.call` does not catch:
 *    a throw here reaches the agent as a broken tool rather than as a
 *    "no", and a model cannot correct itself against a stack trace.
 *
 * Fast on purpose — no play, no waiting — so it can sit in the ordinary
 * suite next to the smoke test.
 */
interface Tool {
  name: string;
  description: string;
  inputSchema: {
    type?: string;
    properties?: Record<string, { type?: string; enum?: string[]; description?: string }>;
    required?: string[];
  };
}

/**
 * Something plausible for each declared property, from the schema
 * itself — an `enum` names its own legal values, and the rest get the
 * smallest thing of the right type. Zero and "" are legal inputs a
 * player could produce by clicking the first of everything.
 */
function plausible(tool: Tool): Record<string, unknown> {
  const args: Record<string, unknown> = {};
  for (const [key, spec] of Object.entries(tool.inputSchema.properties ?? {})) {
    if (spec.enum && spec.enum.length > 0) args[key] = spec.enum[0];
    else if (spec.type === "number") args[key] = 0;
    else if (spec.type === "boolean") args[key] = true;
    else if (spec.type === "array") args[key] = [];
    else if (key === "room") args[key] = "room.garden";
    else if (key === "shaft") args[key] = "shaft.elevator";
    else args[key] = "";
  }
  // A wait is capped at 30s and this spec is not a play session.
  if ("seconds" in args) args.seconds = 1;
  return args;
}

test("every tool answers, and answers a mistake", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/?seed=7001");
  await page.waitForFunction(
    () => (window as unknown as Record<string, unknown>).__webmcp !== undefined,
    null,
    { timeout: 20_000 },
  );

  const tools = await page.evaluate(
    () => (window as unknown as { __webmcp: { tools: Tool[] } }).__webmcp.tools,
  );
  expect(tools.length, "no tools registered at all").toBeGreaterThan(10);

  // **A tool a model cannot read is as broken as one that throws.**
  for (const tool of tools) {
    expect(tool.description.length, `${tool.name} has no description`).toBeGreaterThan(40);
    expect(tool.inputSchema, `${tool.name} has no input schema`).toBeTruthy();
  }

  const call = async (name: string, args: Record<string, unknown>) =>
    page.evaluate(
      ([n, a]) =>
        (
          window as unknown as {
            __webmcp: {
              call: (
                n: string,
                a: Record<string, unknown>,
              ) => Promise<{ content: { text: string }[]; isError?: boolean }>;
            };
          }
        ).__webmcp
          .call(n as string, a as Record<string, unknown>)
          .then((out) => ({ ok: true as const, text: out.content.map((c) => c.text).join("\n") }))
          .catch((error: unknown) => ({ ok: false as const, text: String(error) })),
      [name, args] as const,
    );

  const threw: string[] = [];
  const log: string[] = [];
  for (const tool of tools) {
    for (const [what, args] of [
      ["plausible", plausible(tool)],
      ["empty", {}],
    ] as const) {
      const out = await call(tool.name, args);
      if (!out.ok) threw.push(`${tool.name} (${what} args) THREW: ${out.text}`);
      else log.push(`  ${tool.name} (${what}) → ${out.text.split("\n")[0]?.slice(0, 90) ?? ""}`);
    }
  }

  console.log(`${String(tools.length)} tools:\n${log.join("\n")}`);
  expect(
    threw,
    "a tool threw instead of refusing — an agent cannot correct against a stack trace",
  ).toEqual([]);
});
