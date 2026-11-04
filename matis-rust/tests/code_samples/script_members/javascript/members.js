import { ImportedBrewing as VendorBrewing } from './vendor.js';

class ImportedPickup extends VendorBrewing {}
class Brewing {
  brew(order) { return order; }
}
class Pickup extends Brewing {}
class Delivery extends Brewing {
  brew(order) { return order; }
}
class Unknown extends Missing {}
class Dynamic extends chooseBase() {}
class CalledBase extends Brewing() {}
class Mutated extends Brewing {}
Mutated.prototype.brew = replacement;
class Menu {
  price(order) { return 1; }
  price(order, size) { return 2; }
}
class Window extends Pickup {}
class Masked extends Brewing { brew = replacement; }
class Patched { brew(order) { return order; } }
class PatchedPickup extends Patched {}
Patched.prototype.brew = replacement;
class CycleA extends CycleB {}
class CycleB extends CycleA {}
class Initialized extends Brewing { constructor() { super(); this.brew = replacement; } }
function serve(order) {
  new Pickup().brew(order);
  new Delivery().brew(order);
  new Unknown().brew(order);
  new Dynamic().brew(order);
  new CalledBase().brew(order);
  new Mutated().brew(order);
  new Menu().price();
  new Menu().price(order);
  new Menu().price(order, 1, 2);
  new ImportedPickup().brew(order);
  new Window().brew(order);
  new Masked().brew(order);
  new PatchedPickup().brew(order);
  new CycleA().brew(order);
  new Initialized().brew(order);
  let ledger = new Pickup();
  ledger.brew = replacement;
  ledger.brew(order);
  ledger = unknown();
  ledger.brew(order);
}
