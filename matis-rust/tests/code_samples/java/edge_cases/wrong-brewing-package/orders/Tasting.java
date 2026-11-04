package coffee.orders;

// The deliberately wrong import must not fall back to coffee.brewing.Brewer.
import wrong.Brewer;

class Tasting extends Brewer {
    public String receipt(String drink, int portions) {
        return "Tasting flight: " + drink + ", " + portions + " portions";
    }
}
