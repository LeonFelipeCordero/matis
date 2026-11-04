import { Recipe } from "../brewing/Recipe.ts";

/** Dispatch coffee orders and recover failed pickup notifications. */
export class Fulfillment {
  dispatch(name: string, paid: boolean, ready: boolean): string {
    if (paid) {
      if (ready) {
        if (Recipe.portions(name) > 0) return name;
      }
    }
    return "pending";
  }

  release(name: string, paid: boolean, ready: boolean, packed: boolean, courier: boolean): string {
    if (paid) {
      if (ready) {
        if (packed) {
          if (courier) {
            if (Recipe.portions(name) > 0) return name;
          }
        }
      }
    }
    return "pending";
  }

  audit(paid: boolean, ready: boolean, packed: boolean, labelled: boolean): number {
    let issues = 0;
    if (paid) {
      if (ready) {
        if (!packed) issues++;
        if (!labelled) issues++;
      }
    }
    return issues;
  }

  pack(batches: string[][], active: boolean, ready: boolean): number {
    let total = 0;
    for (const batch of batches) {
      if (active) {
        for (const name of batch) {
          while (!ready) return total;
          total += Recipe.portions(name);
        }
      }
    }
    return total;
  }

  countQueues(waiting: string[], ready: string[]): number {
    let total = 0;
    for (const name of waiting) total += Recipe.portions(name);
    for (const name of ready) total += Recipe.portions(name);
    return total;
  }

  label(name: string, ready: boolean, paid: boolean): string {
    return ready && paid || name === "espresso" ? name : "pending";
  }

  notify(name: string, send: (name: string) => string): string {
    const fallback = name ? "retry" : "cancel";
    try {
      return send(name);
    } catch {
      return fallback;
    }
  }

  priority(amount: number): number {
    if (amount > 8) return 4;
    else if (amount > 3) return 3;
    else if (amount > 1) return 2;
    else if (amount > 0) return 1;
    return 0;
  }

  route(active: boolean, amount: number): string {
    if (active) {
      if (amount > 8) return "courier";
      else if (amount > 3) return "table";
      else if (amount > 0) return "counter";
    }
    return "hold";
  }

  reassign(active: boolean, ready: boolean, courier: boolean): string {
    if (active) {
      if (ready) return "counter";
      else {
        if (courier) return "courier";
      }
    }
    return "hold";
  }

  quote(amount = Recipe.portions("espresso") > 0 ? 2 : 1): number {
    if (amount > 0) return amount;
    return 0;
  }

  schedule(active: boolean, names: string[]): string[] {
    function eligible(name: string): boolean {
      if (name) return Recipe.portions(name) > 0;
      return false;
    }
    const select = (name: string) => name ? eligible(name) : false;
    class Pickup {
      accept(name: string): string {
        if (name) return name;
        return "pending";
      }
    }
    if (active) return names.filter(select).map(name => new Pickup().accept(name));
    return [];
  }
}
