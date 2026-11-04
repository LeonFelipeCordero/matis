from coffee.brewing.recipe import portions
from coffee.reporting.report import render


class Registry(type):
    flights: dict[str, type] = {}

    def register(cls) -> None:
        Registry.flights[cls.__name__] = cls


class Tasting:
    def servings(self, names: list[str]) -> int:
        return sum(portions(name) for name in names)


class Cupping:
    def receipt(self, names: list[str]) -> str:
        return render(names)


class Flight(Tasting, Cupping, metaclass=Registry):
    def brew(self, names: list[str]) -> str:
        if self.servings(names) == 0:
            return "No tasting portions available"
        return self.receipt(names)


Flight.register()
