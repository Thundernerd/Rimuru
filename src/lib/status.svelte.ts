export const status = $state({ text: "" });

export function setStatus(text: string) {
  status.text = text;
}
