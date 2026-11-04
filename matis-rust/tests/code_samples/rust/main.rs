mod brewing;
mod reporting;
#[path = "Dispatch.rs"]
mod dispatch;

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
    let mut schedule = brewing::scheduling::Scheduling::new();
    schedule.enqueue(refill);
    if let Some(scheduled) = schedule.next() {
        println!("{}", scheduled.portions());
    }
    let mut pickups = brewing::tracking::PickupQueue::default();
    let order = brewing::tracking::Order {
        customer: String::from("León"),
        names: names.iter().map(|name| String::from(*name)).collect(),
        total: 9,
        status: "queued",
    };
    println!("{}", pickups.record(order, 1));
    let (ready, pending) = Box::new(pickups).take("ready");
    println!("ready={};pending={}", ready.len(), pending.orders.len());
    println!("{}={}", brewing::fulfillment::route(true, 4), brewing::fulfillment::quote("filter"));
    dispatch::Dispatch::new(1).deliver(&names, |receipt| println!("{}", receipt));
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
