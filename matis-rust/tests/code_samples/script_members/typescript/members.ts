import { ImportedBrewing as VendorBrewing } from './vendor';

class ImportedPickup extends VendorBrewing {}
class Brewing {
  brew(order: string) { return order; }
}
class Pickup extends Brewing {}
class Delivery extends Brewing {
  brew(order: string) { return order; }
}
class Unknown extends Missing {}
class Dynamic extends chooseBase() {}
class CalledBase extends Brewing() {}
class Mutated extends Brewing {}
Mutated.prototype.brew = replacement;
class Menu {
  price(order: string): number;
  price(order: number): number;
  price(order: string | number) { return 1; }
}
function quote(order: string): number;
function quote(order: number): number;
function quote(order: string | number) { return 1; }
class Window extends Pickup {}
class Masked extends Brewing { brew = replacement; }
class Patched { brew(order: string) { return order; } }
class PatchedPickup extends Patched {}
Patched.prototype.brew = replacement;
class CycleA extends CycleB {}
class CycleB extends CycleA {}
class Repeated {
  price(order: string) { return 1; }
  price(order: string, size: number) { return 2; }
}
abstract class SignaturesOnly {
  abstract price(order: string): number;
}
class Initialized extends Brewing { constructor() { super(); this.brew = replacement; } }
function serve(order: string) {
  new Pickup().brew(order);
  new Delivery().brew(order);
  new Unknown().brew(order);
  new Dynamic().brew(order);
  new CalledBase().brew(order);
  new Mutated().brew(order);
  new Menu().price(order);
  quote(order);
  new ImportedPickup().brew(order);
  new Window().brew(order);
  new Masked().brew(order);
  new PatchedPickup().brew(order);
  new CycleA().brew(order);
  new Repeated().price(order);
  new SignaturesOnly().price(order);
  new Initialized().brew(order);
  let ledger = new Pickup();
  ledger.brew = replacement;
  ledger.brew(order);
  ledger = unknown();
  ledger.brew(order);
}
