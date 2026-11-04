from functools import wraps
from typing import Callable

from coffee.orders.models import Order, OrderStatus

DISPATCHERS: dict[str, type] = {}


def register(name: str):
    def attach(dispatcher):
        DISPATCHERS[name] = dispatcher
        return dispatcher
    return attach


def annotate(*, level: int):
    def decorate(operation):
        @wraps(operation)
        def wrapped(*args, **kwargs):
            return operation(*args, **kwargs)
        wrapped.__doc__ = f"Dispatch level {level}"
        return wrapped
    return decorate


@register('café')
class Dispatch:
    @staticmethod
    @annotate(level=1)
    def receipts(orders: list[Order], formatter: Callable[[Order], str]) -> list[str]:
        def format_ready(order: Order) -> str:
            return formatter(order) if order.status == OrderStatus.BREWED else "pending"
        return [format_ready(order) for order in orders]

    def deliver(self, orders: list[Order], send: Callable[[str], None]) -> None:
        for receipt in self.receipts(orders, lambda order: order.customer + ":" + str(order.total)):
            send(receipt)
