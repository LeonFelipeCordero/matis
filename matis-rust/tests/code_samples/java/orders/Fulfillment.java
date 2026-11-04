package coffee.orders;

import coffee.brewing.Recipe;

public class Fulfillment {
    public String route(boolean active, int amount) {
        if (active) {
            if (amount > 8) return "courier";
            else if (amount > 3) return "table";
            else if (amount > 0) return "counter";
        }
        return "hold";
    }

    public int quote(String name) {
        return Recipe.portions(name);
    }
}
