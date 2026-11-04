package coffee.orders;

import coffee.suppliers.SeasonalRecipe;

public class SeasonalOrders {
    public int total(String name) { return SeasonalRecipe.portions(name); }
}
