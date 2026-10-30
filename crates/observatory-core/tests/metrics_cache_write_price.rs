//! `PriceTable::cache_write_usd`: the cache-write class at the table's rate times the role's
//! multiplier.

mod metrics_support;

use metrics_support::prices;
use observatory_core::metrics::PriceTable;
use observatory_core::model::Role;

const SONNET: &str = "claude-sonnet-5-5";

#[test]
fn cache_write_usd_is_the_rate_times_the_role_multiplier() {
    let table = prices();
    let rate = table.rates(SONNET).expect("a row").cache_write_per_m;

    let plain = table
        .cache_write_usd(Role::Worker, Some(SONNET), 400_000)
        .expect("priced");
    assert!((plain - 400_000.0 * rate / 1e6).abs() < 1e-12);

    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/prices/2026-10-22.toml"),
    )
    .expect("the price table exists");
    let doubled = PriceTable::parse(&text.replace("worker = 1.0", "worker = 2.0")).expect("parses");
    let weighted = doubled
        .cache_write_usd(Role::Worker, Some(SONNET), 400_000)
        .expect("priced");
    assert!((weighted - 2.0 * plain).abs() < 1e-12);
}

#[test]
fn cache_write_usd_is_none_without_a_row_or_a_model() {
    let table = prices();
    assert_eq!(
        table.cache_write_usd(Role::Worker, Some("kimi-k3"), 1),
        None
    );
    assert_eq!(table.cache_write_usd(Role::Worker, None, 1), None);
}
