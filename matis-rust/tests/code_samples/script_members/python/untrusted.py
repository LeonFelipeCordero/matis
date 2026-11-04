from vendor import overload


class Untrusted:
    @overload
    def price(self, order: str) -> int: ...

    @overload
    def price(self, order: int) -> int: ...

    def price(self, order):
        return 1


def serve(order):
    Untrusted().price(order)
