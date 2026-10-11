//! Print a text summary of the default (or a saved JSON) scenario:
//! `cargo run --example summary [scenario.json]`

use house_calc::analysis::break_evens;
use house_calc::model::Inputs;
use house_calc::scenario::{letting_year_one, Comparison};

fn main() {
    let inputs: Inputs = match std::env::args().nth(1) {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path).expect("read file")).expect("parse JSON"),
        None => Inputs::default(),
    };
    let c = Comparison::run(&inputs);
    for (name, r) in [("SELL", &c.sell), ("KEEP & LET", &c.keep)] {
        let u = &r.upfront;
        println!("== {name} ==");
        println!("  deposit £{:.0}  new mortgage £{:.0} (LTV {:.0}%)  LBTT £{:.0}  ADS £{:.0}  cash left £{:.0}",
            u.deposit, u.new_mortgage, u.ltv_pct, u.lbtt, u.ads, u.cash_left);
        println!("  year  net worth   liquid    cash   rent   rentTax  £/month");
        for y in &r.years {
            println!("  {:>4} {:>9.0} {:>9.0} {:>8.0} {:>6.0} {:>8.0} {:>8.0}",
                y.year, y.net_worth, y.net_worth_liquid, y.cash, y.rent_received, y.rental_tax, y.monthly_net_cost);
        }
        println!("  total interest £{:.0}, total tax £{:.0}", r.total_interest(), r.total_tax());
        for w in &r.warnings { println!("  ! {w}"); }
    }
    println!("Keep - Sell (liquid): £{:.0}", c.advantage(true));
    println!("{:#?}", letting_year_one(&inputs, inputs.letting.monthly_rent));
    println!("{:#?}", break_evens(&inputs, true));
}
