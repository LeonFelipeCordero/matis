mod brewing;
mod reporting;
#[path = "receipt.rs"]
mod layout_receipt;

mod counter {
    use crate::layout_receipt::print as printer;

    pub fn close(names: &[&str]) {
        println!("{}", printer::render(names));
    }

    pub mod nested {
        pub fn label(names: &[&str]) -> String { names.join(", ") }
    }
}

use crate::brewing::brewer;
use crate::brewing::brewer::Brewer;
use crate::brewing::inventory::Inventory;
use crate::brewing::recipe::Recipe;
use crate::reporting::daily;

fn main() {
    let mut stock: Inventory = Inventory::new(&[("espresso", 12), ("filter", 9)]);
    stock.restock(&[("filter", 3)], |count| count);
    let mut maker: Brewer = Brewer::new(stock);
    let names = ["espresso", "filter", "unknown"];
    crate::layout_receipt::close(&names);
    counter::close(&names);
    println!("{}", crate::layout_receipt::print::render(&names));
    println!("{}", crate::print::render(&names));
    println!("{}", printer::render(&names));
    let receipt = maker.brew(&names);
    println!("{}", receipt);
    println!("{}", brewer::brew(&[]));
    println!("{}", crate::brewing::recipe::preview("espresso"));
    println!("{}", daily::close(&names));
    let drink: Recipe = Recipe::new("filter", 1);
    let refill: Recipe = drink.scale(2);
    let label = refill.label(("takeaway:", ""));
    let dose = refill.portions();
    let remaining = maker.remaining("filter");
    println!("{}={}", label, dose);
    println!("{}", remaining);
    println!(
        "{} {} {}",
        brewer::retry(2),
        brewer::score(&[names.to_vec()]),
        brewer::classify(3)
    );
    println!(
        "{} {}",
        brewer::ready(600, 40, 3, 93),
        brewer::calibrate(40, 15, 0, 1, 93, 93, 6, 6)
    );
}
