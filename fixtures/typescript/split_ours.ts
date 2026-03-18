function buildLeft(value: number): number {
  return value + 1;
}

function buildRight(value: number): number {
  return value * 2 - 3;
}

export function big(value: number): number {
  return buildRight(buildLeft(value));
}
