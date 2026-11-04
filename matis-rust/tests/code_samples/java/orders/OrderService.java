package coffee.orders;

import coffee.brewing.Brewer;
import coffee.inventory.Inventory;
import coffee.inventory.Inventory.Change;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;

public final class OrderService implements CoffeeOrders {
    private final Inventory inventory;
    private final Brewer brewer;
    private final List<Order> orders = new ArrayList<>();
    private final List<String> events = new ArrayList<>();
    private final Consumer<Order> audit = order -> events.add(order.customer);

    public OrderService(Inventory inventory, Brewer brewer) {
        this.inventory = inventory;
        this.brewer = brewer;
    }

    public OrderService(Inventory inventory) {
        this(inventory, new Brewer());
    }

    public Order place(String customer, Order.Line... lines) {
        return place(customer, Map.of(), audit, lines);
    }

    public Order place(String customer, Map<String, String> notes,
                       Consumer<Order> onReady, Order.Line... lines) {
        Order order = new Order(customer, List.of(lines), notes);
        orders.add(order);
        if (!fulfill(order).isEmpty()) onReady.accept(order);
        return order;
    }

    @Override
    public String fulfill(Order order) {
        if (!inventory.reserve(order)) return "";
        String receipt = brewer.brew(order.names());
        order.transition(Order.Status.BREWED);
        return receipt;
    }

    public String preview(Brewer station, Order order) {
        return station.brew(order.names());
    }

    public List<Order> orders(OrderService this) {
        return List.copyOf(orders);
    }

    @Deprecated
    public Order place(String customer, String drink) {
        return place(customer, new Order.Line(drink, 1, 300));
    }

    public static OrderService stocked(int espresso, int filter) {
        var inventory = new Inventory();
        inventory.receive(new Change("espresso", espresso, "opening"),
                          new Change("filter", filter, "opening"));
        return new OrderService(inventory);
    }

    public static final class Factory {
        private final int openingStock;

        public Factory(int openingStock) {
            this.openingStock = openingStock;
        }

        public OrderService create() {
            return OrderService.stocked(openingStock, openingStock);
        }
    }
}
