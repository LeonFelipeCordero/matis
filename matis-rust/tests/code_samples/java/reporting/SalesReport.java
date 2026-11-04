package coffee.reporting;

import coffee.inventory.Inventory;
import coffee.orders.Order;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Predicate;
import static coffee.reporting.Report.render;

public final class SalesReport {
    private final Inventory inventory;
    private final Predicate<Order> include = order -> order.status() == Order.Status.BREWED;

    public SalesReport(Inventory inventory) {
        this.inventory = inventory;
    }

    public Map<String, Integer> totals(List<Order> orders, int minimum) {
        Map<String, Integer> totals = new HashMap<>();
        orders.stream().filter(include).forEach(order -> {
            order.lines.forEach(line -> {
                totals.merge(line.drink, line.subtotal(), Integer::sum);
            });
        });
        totals.entrySet().removeIf(entry -> entry.getValue() < minimum);
        return Map.copyOf(totals);
    }

    public List<List<Integer>> discounts(List<Order> orders, int percent) {
        return orders.stream().map(order -> order.lines.stream()
            .map(line -> line.subtotal() * (100 - percent) / 100).toList()).toList();
    }

    public String summary(List<Order> orders) {
        List<String> names = orders.stream().filter(include)
            .flatMap(order -> order.names().stream()).toList();
        return render(names) + ";espresso-stock=" + inventory.units("espresso");
    }
}
