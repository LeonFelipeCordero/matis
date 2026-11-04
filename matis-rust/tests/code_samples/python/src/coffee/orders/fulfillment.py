"""Route coffee orders, interpret pickup events, and schedule paid batches."""

from ..brewing.recipe import Recipe


class Fulfillment:
    def reserve(self, name, cups):
        if cups > 0:
            return Recipe.portions(name) * cups
        return 0

    def priority(self, amount: int) -> int:
        if amount > 3:
            return 3
        elif amount > 1:
            return 2
        elif amount > 0:
            return 1
        return 0

    def route(self, active: bool, amount: int) -> str:
        if active:
            if amount > 8:
                return "courier"
            elif amount > 3:
                return "table"
            elif amount > 0:
                return "counter"
        return "hold"

    def quote(self, amount=2 if Recipe.portions("espresso") > 0 else 1):
        if amount > 0:
            return amount
        return 0

    def event(self, event):
        match event:
            case "paid" | "ready":
                return "queue"
            case {"cancelled": True}:
                return "cancel"
            case ["collect", customer]:
                return customer
            case _:
                return "hold"

    def ignore_event(self, event):
        match event:
            case _:
                return "hold"

    def retain_event(self, event):
        match event:
            case captured:
                return captured

    def record_event(self, event):
        match event:
            case _ as captured:
                return {"event": captured}

    def wrap_event(self, event):
        match event:
            case (captured):
                return [captured]

    def normalize_event(self, event):
        match event:
            case "unknown" | _:
                return "hold"

    def accept_event(self, event, paid, ready):
        match event:
            case _ if paid and ready:
                return "queue"
        return "hold"

    def retain_paid_event(self, event, paid, ready):
        match event:
            case captured if paid and ready:
                return captured
        return None

    def record_paid_event(self, event, paid, ready):
        match event:
            case _ as captured if paid and ready:
                return {"event": captured}
        return {}

    def dispatch(self, active, event, paid, ready):
        if active:
            match event:
                case "pickup" if paid:
                    if Recipe.portions("espresso") > 0:
                        return "counter"
                case _:
                    if ready:
                        return "table"
        return "hold"

    def scan(self, values, active, enabled, ready, valid):
        while active:
            if enabled:
                match values[0]:
                    case "pickup":
                        if ready and valid:
                            return values[0]

                        def collect():
                            for name in values:
                                if name:
                                    return Recipe.portions(name)
                            return 0

                        return collect()
                    case _:
                        return 0
        return 0
