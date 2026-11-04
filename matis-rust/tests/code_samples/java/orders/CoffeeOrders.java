package coffee.orders;

public interface CoffeeOrders {
    String fulfill(Order order);

    default boolean ready(Order order) {
        return order.status() == Order.Status.BREWED;
    }
}
