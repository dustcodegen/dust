use dust_plugin_api::DustPlugin;
use dust_server_plugin::register_plugin;

#[test]
fn registration_exposes_the_exact_phase_zero_contract() {
    let plugin = register_plugin();
    assert_eq!(plugin.plugin_name(), "Server");
    assert!(plugin.claimed_traits().is_empty());
    assert_eq!(plugin.claimed_configs().len(), 24);
    assert_eq!(plugin.supported_annotations().len(), 24);
    assert_eq!(plugin.annotation_origins().len(), 24);
    assert_eq!(plugin.partless_configs(), plugin.claimed_configs());
    assert_eq!(
        plugin
            .annotation_origins()
            .iter()
            .map(|origin| origin.symbol)
            .collect::<Vec<_>>(),
        plugin.claimed_configs()
    );
}
