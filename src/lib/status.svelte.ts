/** The one-line message in the window's footer. */
export const status = $state({ text: "" });

export function setStatus(text: string) {
  status.text = text;
}
