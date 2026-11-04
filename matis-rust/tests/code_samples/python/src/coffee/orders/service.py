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
        self, customer: str, /, drink: str = "espresso", quantity: int = 1,
        *extras: OrderLine, priority: bool = False,
        on_ready: Callable[[Order], None] | None = None, **notes: str,
    ) -> Order:
        order = Order(customer, [OrderLine(drink, quantity), *extras], notes=notes)
        self.orders.insert(0 if priority else len(self.orders), order)
        if self.fulfill(order) and on_ready is not None:
            on_ready(order)
        return order

    def fulfill(self, order: Order) -> str:
        if not self.inventory.reserve(order):
            return ""
        receipt = self.brewer.brew(order.names())
        order.transition(OrderStatus.BREWED)
        self.events.append(self.audit(receipt))
        return receipt

    def preview(self, brewer: Brewer, order: Order) -> str:
        return brewer.brew(order.names())

    def receipts(self, formatter: Callable[[Order], str]) -> list[str]:
        def format_ready(order: Order) -> str:
            return formatter(order) if order.status == OrderStatus.BREWED else "pending"
        return list(map(format_ready, self.orders))

    async def notify(self, send: Callable[[str], Awaitable[None]]) -> None:
        for receipt in self.receipts(lambda order: order.customer + ":" + str(order.total)):
            await send(receipt)

    @staticmethod  # Delivery estimates do not depend on service state.
    def estimate(queue: int, seconds: int = 30, *, stations: int = 1) -> int:
        return queue * seconds // stations

    @classmethod
    def stocked(klass, espresso: int = 40, filter: int = 60) -> "OrderService":  # pyright: ignore[reportSelfClsParameterName]
        stock = Inventory()
        stock.receive(espresso=espresso, filter=filter)
        return klass(stock, Brewer())
