package coffee.inventory;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Function;

public abstract class StockLedger<T> {
    protected final List<T> entries = new ArrayList<>();

    public void record(T entry) {
        entries.add(entry);
    }

    public <R> List<R> snapshot(Function<? super T, ? extends R> transform) {
        List<R> result = new ArrayList<>();
        for (T entry : entries) result.add(transform.apply(entry));
        return List.copyOf(result);
    }
}
