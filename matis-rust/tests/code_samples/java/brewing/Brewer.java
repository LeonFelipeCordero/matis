package coffee.brewing;

import coffee.reporting.Report;
import java.util.List;

interface Brewing {
    String brew(List<String> names);
}

public final class Brewer implements Brewing {
    @Override
    public String brew(List<String> names) {
        if (names.isEmpty()) return Report.render(List.of());
        List<String> valid = names.stream()
            .filter(name -> Recipe.portions(name) > 0).toList();
        return Report.render(valid);
    }

    public int retry(int attempts) {
        if (attempts <= 0) return 0;
        return 1 + retry(attempts - 1);
    }

    public int score(List<List<String>> batches) {
        int result = 0;
        for (List<String> batch : batches) {
            for (String name : batch) {
                int count = Recipe.portions(name);
                if (count > 0 && count <= 3) {
                    result += count;
                }
            }
        }
        return result;
    }

    // Deliberately exceed the cyclomatic-complexity limit without deep nesting.
    public int classify(int amount) {
        int category = 0;
        if (amount > 0) category++;
        if (amount > 1) category++;
        if (amount > 2) category++;
        if (amount > 3) category++;
        if (amount > 4) category++;
        if (amount > 5) category++;
        if (amount > 6) category++;
        if (amount > 7) category++;
        return category;
    }

    public boolean ready(int water, int beans, int cups, int temperature) {
        return water > 0 && beans > 0 && cups > 0 && temperature > 0;
    }

    public int calibrate(int dose, int water, int seconds, int temperature,
                         int grind, int pressure, int humidity, int altitude) {
        int extraction = water * pressure / Math.max(dose * seconds, 1);
        return extraction + temperature - grind - humidity / 10 + altitude / 100;
    }

    public static void main(String[] args) {
        System.out.println(new Brewer().brew(List.of("espresso", "filter", "invalid")));
    }
}
