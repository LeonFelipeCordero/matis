from typing import overload
from builtins import staticmethod as static

class Brewing:
    def brew(self, order):
        return order
    def sip(self, order):
        return order

class Pickup(Brewing):
    def serve(self, order):
        return self.sip(order)

class FalseSelf(Brewing):
    @staticmethod
    def serve(self, order):
        return self.sip(order)

class AliasedStatic(Brewing):
    @static
    def serve(self, order):
        return self.sip(order)

class WrongParameter(Brewing):
    def serve(other, self, order):
        return self.sip(order)

class ReboundSelf(Brewing):
    def serve(self, order):
        self = unknown()
        return self.sip(order)

class AliasedSelf(Brewing):
    def __init__(receiver):
        receiver.brew = replacement

class Deleted(Brewing):
    def __init__(self):
        del self.brew

class Other:
    def brew(self, order):
        return order

class Ambiguous(Brewing, Other):
    pass

class Override(Brewing, Other):
    def brew(self, order):
        return order

@replacement
class Decorated(Brewing):
    pass

class Menu:
    @overload
    def price(self, order: str): ...
    @overload
    def price(self, order: int): ...
    def price(self, order):
        return order

class DynamicWrite(Brewing):
    def __init__(receiver):
        setattr(receiver, 'brew', replacement)

class OwnOverride(AliasedSelf):
    def brew(self, order):
        return order

class AliasWrite(Brewing):
    def __init__(self):
        alias = self
        chained = alias
        chained.brew = replacement

class AliasMutator(Brewing):
    def __init__(receiver):
        alias = receiver
        setattr(alias, 'brew', replacement)

class AliasOverride(AliasWrite):
    def brew(self, order):
        return order

class SafeAlias(Brewing):
    def __init__(self):
        alias = self
        alias.other = 0
    def unrelated(self):
        alias = unknown()
        alias.brew = replacement

@replacement
class DecoratedBase:
    def brew(self, order):
        return order

class DecoratedChild(DecoratedBase):
    pass

class DecoratedGrandchild(DecoratedChild):
    pass

class PlainBase:
    def brew(self, order):
        return order

class PlainChild(PlainBase):
    pass

class PlainGrandchild(PlainChild):
    pass

def serve(order):
    Pickup().brew(order)
    AliasedSelf().brew(order)
    Deleted().brew(order)
    Ambiguous().brew(order)
    Override().brew(order)
    Decorated().brew(order)
    Menu().price(order)
    DynamicWrite().brew(order)
    OwnOverride().brew(order)
    AliasWrite().brew(order)
    AliasMutator().brew(order)
    AliasOverride().brew(order)
    SafeAlias().brew(order)
    DecoratedBase().brew(order)
    DecoratedChild().brew(order)
    DecoratedGrandchild().brew(order)
    PlainGrandchild().brew(order)
