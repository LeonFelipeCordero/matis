class Brewing { brew(order: string) { return order; } }
class Pickup extends Brewing {}
abstract class Abstract extends Brewing { abstract brew(order: string): string; }
class SignatureOnly extends Brewing { brew(order: string): string; }
class Getter extends Brewing { get brew() { return replacement; } }
class Menu {
  price(order: string): number;
  price(order: number): number;
  price(order: string | number) { return 1; }
}
class Duplicate {
  price(order: string) { return 1; }
  price(order: string, size: number) { return 2; }
}
function quote(order: string): number;
function quote(order: number): number;
function quote(order: string | number) { return 1; }
interface Port { brew: (order: string) => string; }
function invoke(port: Port) { port.brew('coffee'); }
class AliasWrite extends Brewing {
  constructor() { super(); const alias = this; const chained = alias; chained.brew = replacement; }
}
class AliasMutator extends Brewing {
  constructor() { super(); const alias = this; Reflect.set(alias, 'brew', replacement); }
}
class AliasOverride extends AliasWrite { brew(order: string) { return order; } }
class SafeAlias extends Brewing {
  constructor() { super(); const alias = this; alias.other = 0; }
  unrelated() { const alias = {}; alias.brew = replacement; }
}
class FieldBase { brew = () => 0; }
class FieldMiddle extends FieldBase {}
class FieldOverride extends FieldMiddle { brew(order: string) { return order; } }
class StaticFieldBase { static brew = () => 0; }
class StaticFieldOverride extends StaticFieldBase { brew(order: string) { return order; } }
function serve(order: string) {
  new Pickup().brew(order);
  new Abstract().brew(order);
  new SignatureOnly().brew(order);
  new Getter().brew(order);
  new Menu().price(order);
  new Duplicate().price(order);
  quote(order);
  new AliasWrite().brew(order);
  new AliasMutator().brew(order);
  new AliasOverride().brew(order);
  new SafeAlias().brew(order);
  new FieldOverride().brew(order);
  new StaticFieldOverride().brew(order);
}
