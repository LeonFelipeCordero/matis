package coffee.orders;

import coffee.suppliers.ReceiptRecipe;

public class OverloadedOrders {
    public int total(String name) { return ReceiptRecipe.portions(name); }
}
