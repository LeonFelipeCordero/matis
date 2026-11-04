package coffee.orders;

import java.util.*;
import static coffee.brewing.Recipe.portions;

public final class Scheduling {
    private final Queue<Order> waiting = new ArrayDeque<>();

    public void enqueue(Order order) {
        waiting.add(order);
    }

    public int requiredPortions(String drink) {
        return waiting.size() * portions(drink);
    }

    public Order next() {
        return waiting.poll();
    }
}
