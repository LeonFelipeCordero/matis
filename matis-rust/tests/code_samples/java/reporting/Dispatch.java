package coffee.reporting;

import coffee.orders.Order;
import java.util.List;
import java.util.function.Consumer;

@Deprecated
public final class Dispatch {
    private final Consumer<String> send;

    public Dispatch(Consumer<String> send) {
        this.send = send;
    }

    private boolean eligible(Order order) {
        return order.status() == Order.Status.BREWED;
    }

    public void deliver(List<Order> orders) {
        orders.stream().filter(this::eligible).forEach(order -> {
            send.accept(Report.render(order.names()));
        });
    }

    public List<String> preview(List<Order> orders) {
        return orders.stream().map(order -> Report.render(order.names())).toList();
    }
}
