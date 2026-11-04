from ..brewing import recipe


QUEUE: list[dict[str, str | int]] = []


def enqueue(order: dict[str, str | int]) -> int:
    drink = str(order['drink'])
    if not recipe.ingredients(drink)['beans']:
        return 0
    QUEUE.append(order)
    return len(QUEUE)


def collect(ticket: int) -> str:
    order = QUEUE[ticket - 1]
    order['status'] = 'collected'
    return str(order['drink'])
