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
        # Delay the reporting import to keep the source cycle runtime-safe.
        from ..reporting.report import Report

        return Report.render([name]


def portions(name: str) -> int:
    return Recipe.portions(name)
