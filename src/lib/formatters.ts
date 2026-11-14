export function formatPomos(val: number): string {
  return Number.isInteger(val) ? String(val) : val.toFixed(1);
}

export function formatShortTime(iso: string): string {
  if (!iso) return "";
  if (iso.length >= 16 && iso.includes("T")) {
    return iso.slice(11, 16);
  }
  return iso.slice(0, 10);
}

export function formatShortDate(iso: string): string {
  if (!iso) return "";
  return iso.slice(0, 10);
}
