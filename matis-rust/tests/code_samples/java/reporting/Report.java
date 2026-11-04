package coffee.reporting;

import coffee.brewing.Recipe;
import java.util.List;
import java.util.stream.Collectors;

public final class Report {
    private Report() {}

    public static int total(List<String> names) {
        return names.stream().mapToInt(Recipe::portions).sum();
    }

    public static String render(List<String> names) {
        String entries = names.stream()
            .map(name -> name + "=" + Recipe.portions(name))
            .collect(Collectors.joining(","));
        return "coffee:" + entries + ";total=" + total(names);
    }
}
