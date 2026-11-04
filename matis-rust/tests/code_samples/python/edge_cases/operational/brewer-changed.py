from abc import ABC, abstractmethod
import sys

from .recipe import Recipe
from ..reporting.report import Report

# Exercise absolute imports from src/, aliases, and comma-separated modules.
import coffee.brewing.recipe as recipe, coffee.reporting.report as module_report  # noqa: E401 - comma-import fixture
import coffee.brewing.recipe
from coffee.brewing.recipe import portions as portion_count
from coffee.reporting.report import render as render_coffee

# Child-module imports must bind namespaces as well as create import edges.
from . import (
    recipe as relative_recipe,
)
from ..reporting import report as relative_report
from coffee.reporting import report as absolute_report


class Brewing(ABC):
    @abstractmethod
    def brew(self, names: list[str]) -> str:
        raise NotImplementedError


class Brewer(Brewing):
    def brew(self, names: list[str]) -> str:
        if not names:
            return Report.render([])
        valid = [name for name in names if Recipe.portions(name) > 0]
        return Report.render(valid)

    def retry(self, attempts: int) -> int:
        if attempts <= 0:
            return 0
        return 2 + self.retry(attempts - 1)

    def score(self, batches: list[list[str]]) -> int:
        result = 0
        for batch in batches:
            for name in batch:
                count = Recipe.portions(name)
                if count > 0 and count <= 3:
                    result += count
        return result

    # Deliberately exceed the cyclomatic-complexity limit without deep nesting.
    def classify(self, amount: int) -> int:
        category = 0
        if amount > 0:
            category += 1
        if amount > 1:
            category += 1
        if amount > 2:
            category += 1
        if amount > 3:
            category += 1
        if amount > 4:
            category += 1
        if amount > 5:
            category += 1
        if amount > 6:
            category += 1
        if amount > 7:
            category += 1
        return category

    def ready(self, water: int, beans: int, cups: int, temperature: int) -> bool:
        return water > 0 and beans > 0 and cups > 0 and temperature > 0

    def calibrate(
        self, dose: int, water: int, seconds: int, temperature: int,
        grind: int, pressure: int, humidity: int, altitude: int,
    ) -> int:
        extraction = water * pressure // max(dose * seconds, 1)
        return extraction + temperature - grind - humidity // 10 + altitude // 100


def import_styles(name: str) -> str:
    recipe.portions(name)
    coffee.brewing.recipe.portions(name)
    relative_recipe.portions(name)
    portion_count(name)
    return (
        module_report.render([name])
        + absolute_report.render([name])
        + relative_report.render([name])
        + render_coffee([name])
    )


def main() -> None:
    sys.stdout.write(Brewer().brew(["espresso", "filter", "invalid"]) + "\n")
    import_styles("espresso")


if __name__ == "__main__":
    main()
