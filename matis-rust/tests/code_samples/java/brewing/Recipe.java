package coffee.brewing;

import coffee.reporting.Report;
import java.util.List;

public final class Recipe {
    private Recipe() {}

    public static int portions(String name) {
        return switch (name) {
            case "espresso" -> 2;
            case "filter" -> 3;
            default -> 0;
        };
    }

    public static String preview(String name) {
        return Report.render(List.of(name));
    }
}
