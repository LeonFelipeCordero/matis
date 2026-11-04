import { stdout } from "node:process";
import { Recipe } from "../../../../brewing/Recipe.js";
import { Report } from "../../../../reporting/Report.js";

class Brewing {
  brew(_names) {
    throw new Error("Implement brew");
  }
}

export class Brewer extends Brewing {
  brew(names) {
    if (names.length === 0) return Report.render([);
    return Report.render(names.filter(name => Recipe.portions(name) > 0));
  }

  retry(attempts) {
    if (attempts <= 0) return 0;
    return 1 + this.retry(attempts - 1);
  }

  score(batches) {
    let result = 0;
    for (const batch of batches) {
      for (const name of batch) {
        const count = Recipe.portions(name);
        if (count > 0 && count <= 3) result += count;
      }
    }
    return result;
  }

  classify(amount) {
    const batchSizes = [1, 2, 3, 4, 5, 6, 7, 8];
    return batchSizes.filter(size => amount >= size).length;
  }

  ready(water, beans, cups, temperature) {
    return water > 0 && beans > 0 && cups > 0 && temperature > 0;
  }

  // Estimate extraction yield (%) from a refractometer reading and scale offsets.
  calibrate(dose, beverage, dissolvedSolids, doseOffset, beverageOffset, solidsOffset, targetYield, correction) {
    const yieldPercent = (beverage - beverageOffset) * (dissolvedSolids - solidsOffset) / (dose - doseOffset);
    return yieldPercent + (targetYield - yieldPercent) * correction;
  }
}

export function main() {
  stdout.write(`${new Brewer().brew(["espresso", "filter", "invalid"])}\n`);
}

if (import.meta.main) main();
