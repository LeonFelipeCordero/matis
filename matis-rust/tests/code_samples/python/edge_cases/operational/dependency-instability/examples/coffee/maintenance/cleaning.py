CLEANING_INTERVALS = {"espresso": 50, "filter": 100, "grinder": 200}


def due(stations: dict[str, int]) -> list[str]:
    return [
        station for station, servings in stations.items()
        if servings >= CLEANING_INTERVALS.get(station, 100)
    ]


def complete(stations: dict[str, int], station: str) -> str:
    servings = stations.get(station, 0)
    stations[station] = 0
    return f"{station}: cleaned after {servings} servings"
