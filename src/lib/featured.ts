/**
 * The thumbnail row's slots: the featured entries, then empty slots (`null`) up to
 * `count` while more art may still arrive, so the row keeps its size as it fills.
 */
export function thumbSlots<T>(featured: T[], loading: boolean, count: number): (T | null)[] {
  const slots: (T | null)[] = featured.slice(0, count);
  if (loading) while (slots.length < count) slots.push(null);
  return slots;
}
