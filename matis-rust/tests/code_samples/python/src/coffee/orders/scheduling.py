from .models import Order, OrderStatus


class Scheduling:
    def priority(self, order: Order) -> str:
        match order.status:
            case OrderStatus.QUEUED:
                return "next"
            case _:
                return "closed"

    def barista(self, name: str) -> str:
        match name:
            case (assigned):
                return assigned.strip().title()

    def slot(self, minutes: int) -> str:
        match minutes:
            case 1:
                return "express"
            case remaining:
                return f"{remaining} minutes"

    def single_drink(self, names: tuple[str, ...]) -> str:
        match names:
            case (drink,):
                return drink
            case _:
                return "batch"

    def rush(self, minutes: int) -> bool:
        match minutes:
            case 1 | _:
                return minutes <= 5

    def receipt(self, number: int) -> str:
        match number:
            case 1 as first:
                return f"opening ticket {first}"
            case _ as ticket:
                return f"ticket {ticket}"
