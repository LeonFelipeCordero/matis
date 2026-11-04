"""Track supplies and operating temperature for a coffee brewing station."""


class BrewingStation:
    def __init__(self, temperature: int = 93):
        self.temperature = temperature
        self.served = 0

    def ready(self, water: int, beans: int, cups: int, temperature: int) -> bool:
        """A batch needs water, beans, cups and a safe extraction temperature."""
        return min(water, beans, cups) > 0 and temperature >= 90

    def brew(self, cups: int) -> str:
        self.served += cups
        return f"coffee:served={self.served};temperature={self.temperature}"

    def clean(self) -> None:
        self.served = 0
