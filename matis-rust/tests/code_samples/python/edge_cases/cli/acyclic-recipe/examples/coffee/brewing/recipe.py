class Recipe:
    @staticmethod
    def portions(name: str) -> int:
        match name:
            case "espresso":
                return 2
            case "filter":
                return 3
            case _:
                return 0

    @staticmethod
    def preview(name: str) -> str:
        # Preview just one recipe; reporting remains a downstream consumer.
        units = Recipe.portions(name)
        return f"coffee:{name}={units};total={units}"


def portions(name: str) -> int:
    return Recipe.portions(name)
