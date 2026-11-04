from dataclasses import dataclass, field
from enum import Enum


class OrderStatus(Enum):
    QUEUED = "queued"
    BREWED = "brewed"
    CANCELLED = "cancelled"


@dataclass(frozen=True)
class OrderLine:
    drink: str
    quantity: int = 1
    price: int = 300

    def subtotal(self) -> int:
        return self.quantity * self.price

    @classmethod
    def espresso(klass, quantity: int = 1) -> "OrderLine":  # pyright: ignore[reportSelfClsParameterName]
        return klass("espresso", quantity, 250)


@dataclass
class Order:
    customer: str
    lines: list[OrderLine]
    status: OrderStatus = OrderStatus.QUEUED
    notes: dict[str, str] = field(default_factory=dict)

    @property
    def total(self) -> int:
        return sum(line.subtotal() for line in self.lines)

    def names(self) -> list[str]:
        return [line.drink for line in self.lines for _ in range(line.quantity)]

    def transition(self, status: OrderStatus) -> str:
        match self.status, status:
            case OrderStatus.QUEUED, OrderStatus.BREWED:
                self.status = status
                return "collected"
            case OrderStatus.QUEUED, OrderStatus.CANCELLED:
                self.status = status
                return "refunded"
            case _:
                return "unchanged"
