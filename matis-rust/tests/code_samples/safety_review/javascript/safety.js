class Brewing { brew(order) { return order; } }
class Pickup extends Brewing {}
class Bracket extends Brewing {}
Bracket.prototype['brew'] = replacement;
class Dynamic extends Brewing {}
Dynamic.prototype[key] = replacement;
class Replaced extends Brewing {}
Replaced.prototype = unknown;
class Defined extends Brewing {}
Object.defineProperty(Defined.prototype, 'brew', { value: replacement });
class Deleted extends Brewing {}
delete Deleted.prototype.brew;
class Instance extends Brewing { constructor() { super(); this['brew'] = replacement; } }
class Getter extends Brewing { get brew() { return replacement; } }
class Computed extends Brewing { [key](order) { return order; } }
class Menu {
  price(order) { return 1; }
  price(order, size) { return 2; }
}
class Alias extends Brewing {}
const prototypeAlias = Alias.prototype;
prototypeAlias.brew = replacement;
class DefinedInstance extends Brewing {
  constructor() { super(); Object.defineProperty(this, 'brew', { value: replacement }); }
}
class ParentWrite { constructor() { this.brew = replacement; } }
class OwnOverride extends ParentWrite { brew(order) { return order; } }
class AliasWrite extends Brewing {
  constructor() { super(); const alias = this; const chained = alias; chained.brew = replacement; }
}
class AliasMutator extends Brewing {
  constructor() { super(); const alias = this; Object.assign(alias, { brew: replacement }); }
}
class AliasOverride extends AliasWrite { brew(order) { return order; } }
class SafeAlias extends Brewing {
  constructor() { super(); const alias = this; alias.other = 0; }
  unrelated() { const alias = {}; alias.brew = replacement; }
}
class FieldBase { brew = () => 0; }
class FieldMiddle extends FieldBase {}
class FieldOverride extends FieldMiddle { brew(order) { return order; } }
class StaticFieldBase { static brew = () => 0; }
class StaticFieldOverride extends StaticFieldBase { brew(order) { return order; } }
function serve(order) {
  new Pickup().brew(order);
  new Bracket().brew(order);
  new Dynamic().brew(order);
  new Replaced().brew(order);
  new Defined().brew(order);
  new Deleted().brew(order);
  new Instance().brew(order);
  new Getter().brew(order);
  new Computed().brew(order);
  new Menu().price(order);
  new Alias().brew(order);
  new DefinedInstance().brew(order);
  new OwnOverride().brew(order);
  new AliasWrite().brew(order);
  new AliasMutator().brew(order);
  new AliasOverride().brew(order);
  new SafeAlias().brew(order);
  new FieldOverride().brew(order);
  new StaticFieldOverride().brew(order);
}
