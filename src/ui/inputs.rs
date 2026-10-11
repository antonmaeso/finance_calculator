//! Left-hand panel: every variable you can play with.

use eframe::egui::{self, CollapsingHeader, RichText, Ui};

use super::{help, money, money_input, pct_input, years_input, MUTED};
use house_calc::model::{DepositMode, Inputs, Owner};
use house_calc::tax::Band;

pub fn inputs_panel(ui: &mut Ui, i: &mut Inputs) {
    ui.spacing_mut().slider_width = 150.0;

    CollapsingHeader::new(RichText::new("👥 You & income").strong()).default_open(true).show(ui, |ui| {
        help(ui, "Income sets your tax band for rent, savings interest and CGT.");
        let mut remove = None;
        let n = i.owners.len();
        for (k, o) in i.owners.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut o.name).desired_width(90.0));
                if n > 1 && ui.small_button("✖").on_hover_text("Remove owner").clicked() {
                    remove = Some(k);
                }
            });
            money_input(ui, "income / yr", &mut o.income, 200_000.0, "Gross salary, pension and other non-property income");
            pct_input(ui, "share of first home", &mut o.share_pct, 0.0, 100.0, "Ownership share: rent profit and gains are split this way");
            ui.separator();
        }
        if let Some(k) = remove {
            i.owners.remove(k);
        }
        if i.owners.len() < 2 && ui.button("➕ Add joint owner").clicked() {
            i.owners.push(Owner { name: "Partner".into(), income: 40_000.0, share_pct: 50.0 });
        }
        pct_input(ui, "income growth / yr", &mut i.income_growth_pct, 0.0, 8.0, "Used to move your tax band over time");
    });

    CollapsingHeader::new(RichText::new("🏠 Current home").strong()).default_open(true).show(ui, |ui| {
        let c = &mut i.current;
        money_input(ui, "value today", &mut c.value, 1_000_000.0, "What it would sell for now");
        money_input(ui, "mortgage owed", &mut c.mortgage_balance, 800_000.0, "Outstanding balance on the current mortgage");
        pct_input(ui, "current rate", &mut c.rate_pct, 0.0, 10.0, "Your current mortgage rate (used if you keep it without remortgaging)");
        years_input(ui, "fix left", &mut c.fixed_years_left, 10.0, "Years left on the current fixed rate");
        pct_input(ui, "rate after fix", &mut c.revert_rate_pct, 0.0, 10.0, "Expected rate once the fix ends");
        years_input(ui, "term left", &mut c.term_years_left, 40.0, "Years left on the mortgage term");
        pct_input(ui, "price growth / yr", &mut c.growth_pct, -5.0, 10.0, "Expected annual house price growth for this property");
        ui.label(RichText::new("For capital gains tax").small().strong());
        money_input(ui, "price you paid", &mut c.purchase_price, 1_000_000.0, "Original purchase price (CGT base cost)");
        money_input(ui, "buying costs + improvements", &mut c.purchase_costs, 100_000.0, "LBTT, legal fees when bought, capital improvements – reduce the taxable gain");
        years_input(ui, "years owned so far", &mut c.years_owned, 40.0, "");
        years_input(ui, "years lived in it", &mut c.years_lived_in, 40.0, "Main-residence years count towards private residence relief");
        c.years_lived_in = c.years_lived_in.min(c.years_owned);
        ui.label(RichText::new("Selling costs").small().strong());
        pct_input(ui, "estate agent", &mut c.selling_agent_pct, 0.0, 4.0, "Agent fee as % of sale price (incl. VAT)");
        money_input(ui, "legal, Home Report etc.", &mut c.selling_fixed_costs, 10_000.0, "Solicitor, Home Report, marketing");
    });

    CollapsingHeader::new(RichText::new("🏡 New home").strong()).default_open(true).show(ui, |ui| {
        let n = &mut i.new_home;
        money_input(ui, "price", &mut n.price, 1_500_000.0, "Purchase price of the new home");
        pct_input(ui, "price growth / yr", &mut n.growth_pct, -5.0, 10.0, "");
        money_input(ui, "fees (legal, survey…)", &mut n.purchase_fees, 20_000.0, "Solicitor, survey, lender arrangement fee, removals");
        ui.checkbox(&mut n.first_time_buyer, "First-time buyer (only if you've never owned)")
            .on_hover_text("FTB relief raises the LBTT nil band. Not available if you already own a home.");
        ui.label(RichText::new("Mortgage").small().strong());
        pct_input(ui, "initial rate", &mut n.rate_pct, 0.0, 10.0, "");
        years_input(ui, "fixed for", &mut n.fixed_years, 10.0, "");
        pct_input(ui, "rate after fix", &mut n.revert_rate_pct, 0.0, 10.0, "What you expect to remortgage at");
        years_input(ui, "term", &mut n.term_years, 40.0, "");
        money_input(ui, "overpay / month", &mut n.overpayment_monthly, 3_000.0, "Extra capital paid each month");
    });

    CollapsingHeader::new(RichText::new("💰 Deposits & cash").strong()).default_open(true).show(ui, |ui| {
        let s = &mut i.savings;
        money_input(ui, "savings today", &mut s.cash, 500_000.0, "Cash available before the move");
        money_input(ui, "keep as buffer", &mut s.cash_buffer, 100_000.0, "Emergency fund that never goes into a deposit");
        money_input(ui, "monthly housing budget", &mut s.monthly_budget, 8_000.0,
            "Money from your salary each month for housing. Whatever isn't spent on mortgages / letting is saved. Same in both strategies.");
        pct_input(ui, "budget growth / yr", &mut s.budget_growth_pct, 0.0, 8.0, "");
        pct_input(ui, "savings rate", &mut s.rate_pct, 0.0, 10.0, "Interest on spare cash (AER)");
        pct_input(ui, "share in ISAs", &mut s.isa_share_pct, 0.0, 100.0, "Interest on this share is tax-free (ISA allowance £20k/person/yr)");
        ui.separator();
        deposit_row(ui, "Deposit if selling", &mut i.sell_deposit_mode, &mut i.sell_deposit);
        deposit_row(ui, "Deposit if keeping", &mut i.keep_deposit_mode, &mut i.keep_deposit);
    });

    CollapsingHeader::new(RichText::new("🔑 Keep & let options").strong()).default_open(true).show(ui, |ui| {
        let l = &mut i.letting;
        ui.checkbox(&mut l.remortgage_to_btl, "Remortgage first home to buy-to-let")
            .on_hover_text("Otherwise the existing residential mortgage stays (you need the lender's consent to let).");
        if l.remortgage_to_btl {
            let max_release = (i.current.value * 0.75 - i.current.mortgage_balance).max(0.0);
            money_input(ui, "equity to release", &mut l.release_equity, max_release.max(10_000.0),
                &format!("Extra borrowing against the first home. At 75% LTV you could release up to {}.", money(max_release)));
            pct_input(ui, "BTL rate", &mut l.btl_rate_pct, 0.0, 10.0, "");
            years_input(ui, "BTL fixed for", &mut l.btl_fixed_years, 10.0, "");
            pct_input(ui, "BTL rate after fix", &mut l.btl_revert_rate_pct, 0.0, 10.0, "");
            years_input(ui, "BTL term", &mut l.btl_term_years, 40.0, "");
            ui.checkbox(&mut l.btl_interest_only, "Interest-only");
            money_input(ui, "remortgage fees", &mut l.remortgage_fees, 10_000.0, "Arrangement, valuation, legal");
            pct_input(ui, "lender stress rate", &mut l.lender_stress_rate_pct, 0.0, 10.0, "Rate lenders use to test affordability");
            pct_input(ui, "lender ICR", &mut l.lender_icr_pct, 100.0, 200.0, "Rent must cover stressed interest by this much (125% basic-rate, 145% higher-rate)");
        }
        ui.label(RichText::new("Rent & running costs").small().strong());
        money_input(ui, "rent / month", &mut l.monthly_rent, 4_000.0, "");
        pct_input(ui, "rent growth / yr", &mut l.rent_growth_pct, -2.0, 8.0, "Note: Scottish rent control areas may cap increases");
        ui.add(egui::Slider::new(&mut l.void_weeks_per_year, 0.0..=12.0).text("void weeks / yr").step_by(0.5));
        pct_input(ui, "letting agent", &mut l.letting_agent_pct, 0.0, 20.0, "Full management typically 10–15% incl. VAT");
        pct_input(ui, "maintenance", &mut l.maintenance_pct_of_rent, 0.0, 25.0, "Repairs as % of rent");
        money_input(ui, "fixed costs / yr", &mut l.fixed_costs_annual, 5_000.0, "Landlord insurance, gas & electrical safety, EPC, landlord registration, deposit scheme");
        money_input(ui, "setup costs", &mut l.setup_costs, 20_000.0, "One-off: repairs, compliance work, furnishing before first tenant");
        ui.separator();
        ui.checkbox(&mut l.plan_sale, "Sell the first home later");
        if l.plan_sale {
            years_input(ui, "sell after", &mut l.sale_after_years, i.horizon_years as f64, "Sell within 3 years to reclaim ADS");
            if l.sale_after_years <= 3.0 {
                ui.label(RichText::new("✔ within 36 months – ADS refunded").small().color(super::GOOD));
            }
        }
    });

    CollapsingHeader::new(RichText::new("⚖ Tax rules").strong()).default_open(false).show(ui, |ui| {
        tax_rules(ui, i);
    });
}

fn deposit_row(ui: &mut Ui, label: &str, mode: &mut DepositMode, amount: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.selectable_value(mode, DepositMode::MaxAvailable, "Max").on_hover_text("Everything except the buffer");
        ui.selectable_value(mode, DepositMode::Fixed, "Fixed");
    });
    if *mode == DepositMode::Fixed {
        money_input(ui, "amount", amount, 1_000_000.0, "");
    }
}

fn tax_rules(ui: &mut Ui, i: &mut Inputs) {
    let t = &mut i.tax;
    ui.horizontal(|ui| {
        ui.label("Tax year");
        ui.text_edit_singleline(&mut t.year_label);
    });
    if ui.button("Reset to 2026/27 rules").clicked() {
        *t = Default::default();
    }
    ui.label(RichText::new("LBTT bands (price from → rate)").small().strong());
    bands_editor(ui, "lbtt", &mut t.lbtt_bands, 0.0);
    money_input(ui, "FTB nil band", &mut t.lbtt_ftb_nil_band, 300_000.0, "");
    let mut ads = t.ads_rate * 100.0;
    if pct_input(ui, "ADS rate", &mut ads, 0.0, 15.0, "Additional Dwelling Supplement") {
        t.ads_rate = ads / 100.0;
    }
    ui.label(RichText::new("Scottish income tax (gross income from → rate)").small().strong());
    let pa = t.personal_allowance;
    bands_editor(ui, "sit", &mut t.scottish_bands, pa);
    money_input(ui, "personal allowance", &mut t.personal_allowance, 20_000.0, "");
    ui.label(RichText::new("CGT").small().strong());
    money_input(ui, "annual exempt amount", &mut t.cgt_annual_exempt, 20_000.0, "");
    let mut b = t.cgt_basic_rate * 100.0;
    if pct_input(ui, "basic rate", &mut b, 0.0, 40.0, "") {
        t.cgt_basic_rate = b / 100.0;
    }
    let mut h = t.cgt_higher_rate * 100.0;
    if pct_input(ui, "higher rate", &mut h, 0.0, 40.0, "") {
        t.cgt_higher_rate = h / 100.0;
    }
    ui.label(RichText::new("Savings interest uses UK bands; PSA £1,000 basic / £500 higher.").small().color(MUTED));
}

/// Edit marginal bands. `offset` converts between stored taxable thresholds
/// and the gross figures people recognise.
fn bands_editor(ui: &mut Ui, id: &str, bands: &mut [Band], offset: f64) {
    egui::Grid::new(id).num_columns(2).show(ui, |ui| {
        for b in bands.iter_mut() {
            let mut gross = b.threshold + offset;
            if ui.add(egui::DragValue::new(&mut gross).prefix("£").speed(100.0).range(0.0..=f64::MAX)).changed() {
                b.threshold = (gross - offset).max(0.0);
            }
            let mut r = b.rate * 100.0;
            if ui.add(egui::DragValue::new(&mut r).suffix("%").speed(0.1).range(0.0..=100.0)).changed() {
                b.rate = r / 100.0;
            }
            ui.end_row();
        }
    });
}
