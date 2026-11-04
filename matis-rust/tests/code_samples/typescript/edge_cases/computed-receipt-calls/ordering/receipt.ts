export function helper(): string {
  return "Coffee tasting: espresso, 18g";
}

export function factory(): { helper(): string } {
  return { helper };
}

export function render(handlers: any, key: string, argument: any): string {
  const line = handlers[key](argument);
  const sample = factory().helper();
  const supplier = handlers.helper();
  return `${helper()}; ${sample}; ${supplier}; ${line}`;
}
