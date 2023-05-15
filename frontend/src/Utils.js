export function classNames(...classes) {
  return classes.filter(Boolean).join(" ");
}

export const MS_TO_NS_SCALE = 1000000;

// return human readable duration string
export function durationString(durationNs) {
  const durationMs = durationNs / MS_TO_NS_SCALE;
  const durationS = durationMs / 1000;
  const durationM = durationS / 60;
  const durationH = durationM / 60;
  const durationD = durationH / 24;
  const durationY = durationD / 365;

  if (durationY >= 2) {
    return `${Math.round(durationY)} years`;
  } else if (durationY >= 1) {
    return `1 year`;
  } else if (durationD >= 2) {
    return `${Math.round(durationD)} days`;
  } else if (durationD >= 1) {
    return `1 day`;
  } else if (durationH >= 2) {
    return `${Math.round(durationH)} hours`;
  } else if (durationH >= 1) {
    return `1 hour`;
  } else if (durationM >= 2) {
    return `${Math.round(durationM)} minutes`;
  } else if (durationM >= 1) {
    return `1 minute`;
  } else if (durationS >= 2) {
    return `${Math.round(durationS)} seconds`;
  } else if (durationS >= 1) {
    return `1 second`;
  }
}

// return human readable date time string
export function dateTimeString(tsNs) {
  const tsMs = tsNs / MS_TO_NS_SCALE;
  const date = new Date(tsMs);
  return date.toLocaleString();
}