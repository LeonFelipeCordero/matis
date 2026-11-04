from coffee.brewing.brewer import Brewing as Parent
from coffee.brewing.recipe import portions
from coffee.reporting.report import render


class Tasting(Parent):
    def brew(self, names: list[str]) -> str:
        selected = [name for name in names if portions(name) > 0]
        return render(selected)
