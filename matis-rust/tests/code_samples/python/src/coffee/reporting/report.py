from ..brewing.recipe import Recipe


class Report:
    @staticmethod
    def total(names: list[str]) -> int:
        return sum(Recipe.portions(name) for name in names)

    @staticmethod
    def render(names: list[str]) -> str:
        entries = ",".join(f"{name}={Recipe.portions(name)}" for name in names)
        return f"coffee:{entries};total={Report.total(names)}"


def render(names: list[str]) -> str:
    return Report.render(names)
