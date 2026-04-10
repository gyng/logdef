import en from "./en.json";

type Messages = typeof en;
type MessageKey = keyof Messages;

const messages: Messages = en;

export function t(key: MessageKey, vars?: Record<string, string | number>): string {
  const template = messages[key];
  if (!vars) {
    return template;
  }

  return template.replace(/\{(\w+)\}/g, (_match, token: string) => {
    const value = vars[token];
    return value == null ? `{${token}}` : String(value);
  });
}
