from collections import defaultdict
from typing import Callable, Iterable

from coffee.orders.models import Order, OrderStatus
from coffee.inventory.stock import Inventory
from .report import Report as CoffeeReport


class SalesReport:
    def __init__(self, inventory: Inventory):
        self.inventory: Inventory = inventory
        self.include: Callable[[Order], bool] = lambda order: order.status == OrderStatus.BREWED

    def totals(self, orders: Iterable[Order], *, minimum: int = 0) -> dict[str, int]:
        totals: dict[str, int] = defaultdict(int)

        def collect(order: Order) -> None:
            if not self.include(order):
                return
            for line in order.lines:
                totals[line.drink] += line.subtotal()

        for order in orders:
            collect(order)
        return {drink: amount for drink, amount in totals.items() if amount >= minimum}

    def discounts(self, orders: list[Order], percent: int) -> list[list[int]]:
        # Outer and inner callbacks retain their separate order/line scopes.
        return list(map(
            lambda order: list(map(
                lambda line: line.subtotal() * (100 - percent) // 100, order.lines,
            )), orders,
        ))

    def render(self, orders: list[Order]) -> str:
        names = [name for order in orders if self.include(order) for name in order.names()]
        stock = self.inventory.snapshot()
        return CoffeeReport.render(names) + ";espresso-stock=" + str(stock.units("espresso"))
