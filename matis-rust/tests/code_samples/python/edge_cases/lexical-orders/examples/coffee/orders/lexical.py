"""Menu callbacks may supply seasonal recipes without changing the house recipe."""

from coffee.brewing.recipe import Recipe, Recipe as Drink
from coffee.reporting.report import Report


def conditional(name, override):
    if override:
        Drink = override
    return Drink.portions(name)


def loop(name, recipes):
    for Drink in recipes:
        print(Drink.preview(name))
    return Drink.portions(name)


def with_stock(name, stock):
    with stock.recipe() as Drink:
        print(Drink.preview(name))
    return Drink.portions(name)


def exception(name, supplier):
    try:
        supplier.deliver(name)
    except LookupError as Drink:
        print(Drink)
    return Drink.portions(name)


def future_assignment(name, override):
    initial = Drink.portions(name)
    if override:
        Drink = override
    return initial


def conditional_callable(name, seasonal):
    if seasonal:
        def Drink(names):
            return Report.render(names)
    return Drink.portions(name)


def conditional_class(name, seasonal):
    if seasonal:
        class Drink:
            def preview(self, names):
                return Report.render(names)
    return Drink.portions(name)


def outside(name):
    return Drink.portions(name)


def import_order(before, after, seasonal):
    initial = Drink.portions(before)
    if seasonal:
        from seasonal_supplier import Recipe as Drink
    return initial + Drink.portions(after)


def local_import(name, available):
    if available:
        from coffee.brewing.recipe import Recipe as LocalDrink
    return LocalDrink.portions(name)


def missing_import(name):
    return LocalDrink.portions(name)


name = "espresso"
names = [name, "filter"]
if names:
    from coffee.brewing.recipe import Recipe as ConditionalDrink
print(ConditionalDrink.portions(name))


class Menu:
    if names:
        from coffee.brewing.recipe import Recipe as MenuDrink
    label = MenuDrink.preview(name)

    def receipt(self, name):
        return MenuDrink.preview(name)

    def portions(self, name):
        return Drink.portions(name)


class SeasonalMenu:
    if names:
        Drink = Report
    portion_count = Drink.portions(name)

    def receipt(self, name):
        return Drink.preview(name)

    def default_receipt(self, names=Drink.preview(name)):
        return Report.render(names)

    counts = [Drink.portions(drink_name) for drink_name in Drink.preview(name)]


class Receipt:
    from coffee.reporting.report import Report as Drink
    label = Drink.render(names)

    def portions(self, name):
        return Drink.portions(name)


def lambda_single(recipes, name):
    counts = map(lambda Drink: Drink.portions(name), recipes)
    return list(counts), Drink.preview(name)


def lambda_default(recipes, name):
    counts = map(lambda Drink=Recipe: Drink.portions(name), recipes)
    return list(counts), Drink.preview(name)


def lambda_rest(recipes, name):
    counts = map(lambda *Drink: Drink.portions(name), recipes)
    return list(counts), Drink.preview(name)


def lambda_keyword(recipes, name):
    count = (lambda *, Drink: Drink.portions(name))(Drink=recipes[0])
    return count, Drink.preview(name)


def lambda_keywords(recipes, name):
    count = (lambda **Drink: Drink.portions(name))(**recipes[0])
    return count, Drink.preview(name)


def list_orders(recipes, name):
    counts = [Drink.portions(name) for Drink in recipes]
    return counts, Drink.preview(name)


def generator_orders(recipes, name):
    counts = (Drink.portions(name) for Drink in recipes)
    return list(counts), Drink.preview(name)


def set_orders(recipes, name):
    counts = {Drink.portions(name) for Drink in recipes}
    return counts, Drink.preview(name)


def dict_orders(recipes, name):
    counts = {Drink: Drink.portions(name) for Drink in recipes}
    return counts, Drink.preview(name)


def nested_orders(batches, name):
    counts = [Drink.portions(name) for batch in batches for Drink in batch]
    return counts, Drink.preview(name)


def paired_orders(recipes, name):
    counts = [Drink.portions(name) for label, Drink in recipes]
    return counts, Drink.preview(name)


def filtered_orders(recipes, name):
    counts = [Drink.portions(name) for Drink in recipes if name in Drink.names]
    return counts, Drink.preview(name)


def first_iterable(name):
    counts = [Drink.portions(name) for Drink in Drink.preview(name)]
    return counts


def walrus_order(recipes, name):
    labels = [(Drink := recipe).preview(name) for recipe in recipes]
    return labels, Drink.portions(name)


def receipt(names):
    return Report.render(names)


def custom_receipt(receipt, names):
    return receipt(names)


def standard_receipt(names):
    return receipt(names)


class SeasonalRecipe(Recipe):
    Recipe = Report


class ImportedRecipe(Recipe):
    from coffee.reporting.report import Report as Recipe


def unlinked(names):
    return Brewer().brew(names)


class RetryOrders:
    def retry(self, names, attempts, replacement):
        self = replacement
        return self.retry(names, attempts - 1, replacement)
