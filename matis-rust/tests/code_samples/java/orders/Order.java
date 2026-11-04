package coffee.orders;

import java.util.List;
import java.util.Map;

public final class Order {
    public enum Status { QUEUED, BREWED, CANCELLED }

    public static final class Line {
        public final String drink;
        public final int quantity;
        public final int price;

        public Line(String drink, int quantity, int price) {
            this.drink = drink;
            this.quantity = quantity;
            this.price = price;
        }

        public int subtotal() {
            return quantity * price;
        }
    }

    public final String customer;
    public final List<Line> lines;
    public final Map<String, String> notes;
    private Status status = Status.QUEUED;

    public Order(String customer, List<Line> lines, Map<String, String> notes) {
        this.customer = customer;
        this.lines = List.copyOf(lines);
        this.notes = Map.copyOf(notes);
    }

    public Status status() {
        return status;
    }

    public int total() {
        return lines.stream().mapToInt(Line::subtotal).sum();
    }

    public List<String> names() {
        return lines.stream().flatMap(line ->
            java.util.Collections.nCopies(line.quantity, line.drink).stream()).toList();
    }

    public String transition(Status next) {
        if (status != Status.QUEUED) return "unchanged";
        String result = switch (next) {
            case BREWED -> "collected";
            case CANCELLED -> "refunded";
            default -> "unchanged";
        };
        status = next;
        return result;
    }
}
