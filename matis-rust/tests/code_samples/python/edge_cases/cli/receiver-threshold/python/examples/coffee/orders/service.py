from abc import ABC, abstractmethod
from functools import wraps
from typing import Awaitable, Callable

from coffee.brewing.brewer import Brewer
from coffee.inventory.stock import Inventory
from .models import Order, OrderLine, OrderStatus


def audited(action: str):
    """Label service actions without changing their call signatures."""
    def decorate(operation):
        @wraps(operation)
        def wrapped(service, *args, **kwargs):
            service.events.append(action)
            return operation(service, *args, **kwargs)
        return wrapped
    return decorate


class CoffeeOrders(ABC):
    @abstractmethod
    def fulfill(self, order: Order) -> str:
        raise NotImplementedError


class AuditTrail:
    def audit(self, message: str) -> str:
        return "coffee-order:" + message


class OrderService(CoffeeOrders, AuditTrail):
    def __init__(self, inventory: Inventory, brewer: Brewer):
        self.inventory: Inventory = inventory
        self.brewer = brewer
        self.orders: list[Order] = []
        self.events: list[str] = []

    @audited("place")
    def place(
        self, customer: str, /, drink: str = "espresso",
        *extras: OrderLine, **notes: str,
    ) -> Order:
        order = Order(customer, [OrderLine(drink, 1), *extras], notes=notes)
        self.orders.append(order)
        self.fulfill(order)
        return order

    def fulfill(self, order: Order) -> str:
        if not self.inventory.reserve(order):
            return ""
        receipt = self.brewer.brew(order.names())
        order.transition(OrderStatus.BREWED)
        self.events.append(self.audit(receipt))
        return receipt

    def preview(self, brewer: Brewer, order: Order, copies: int = 1) -> str:
        return brewer.brew(order.names()) * copies

    def receipts(self, formatter: Callable[[Order], str]) -> list[str]:
        def format_ready(order: Order) -> str:
            return formatter(order) if order.status == OrderStatus.BREWED else "pending"
        return list(map(format_ready, self.orders))

    async def notify(self, send: Callable[[str], Awaitable[None]]) -> None:
        for receipt in self.receipts(lambda order: order.customer + ":" + str(order.total)):
            await send(receipt)

    @staticmethod  # Delivery estimates do not depend on service state.
    def estimate(queue: int, seconds: int = 30, *, stations: int = 1, cleanup: int = 10) -> int:
        return queue * (seconds + cleanup) // stations

    @classmethod
    def stocked(klass, espresso: int = 40, filter: int = 60, decaf: int = 20) -> "OrderService":  # pyright: ignore[reportSelfClsParameterName]
        stock = Inventory()
        stock.receive(espresso=espresso, filter=filter, decaf=decaf)
        return klass(stock, Brewer())
