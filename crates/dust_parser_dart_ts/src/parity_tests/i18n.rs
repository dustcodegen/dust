//! Translation entries and warnings must survive the grammar migration exactly.

use crate::{I18nScanResult, scan_i18n_source};
use dust_text::{FileId, SourceText};

/// Scans and emits every entry, warning, label and byte span for comparison.
fn contract(case: &str, text: &str, entries: usize, warnings: usize) {
    let result = scan_i18n_source(&SourceText::new(FileId::new(0), text));
    assert_eq!(result.entries.len(), entries, "{result:?}");
    assert_eq!(result.diagnostics.len(), warnings, "{result:?}");
    println!("DUST_PARSER_PARITY:{case}:{result:?}");
}

#[test]
fn static_keys_fallbacks_and_placeholder_order() {
    contract(
        "i18n-static",
        r#"
Widget build(BuildContext context) {
  return Column(children: [
    const TranslatedText('home_title'),
    TranslatedText('home.greeting', /* keep named arguments */
      defaultText: 'Hello 🌿 {name}',
      args: const {'name': user.name, 'count': cart.count, 'name': other}),
    Text(context.tr("settings_title", defaultText: "Settings")),
  ]);
}
"#,
        3,
        0,
    );
}

#[test]
fn runtime_keys_warn_and_dynamic_opt_out_does_not() {
    contract(
        "i18n-runtime",
        r#"
Widget build(BuildContext context) {
  return Column(children: [
    TranslatedText.dynamic(product.titleKey, fallback: product.title),
    const TranslatedText.dynamic('literal_opt_out'),
    TranslatedText(product.titleKey),
    Text(context.tr(settingsKey)),
    TranslatedText('home_' + runtimeKey),
  ]);
}
"#,
        0,
        3,
    );
}

#[test]
fn hardcoded_text_warnings_and_nested_translations() {
    contract(
        "i18n-hardcoded",
        r#"
Widget build(BuildContext context) {
  return Column(children: [
    const Text('Checkout 🌿'), Text("Cart"),
    Text(context.tr('shop_title')),
  ]);
}
"#,
        1,
        2,
    );
}

#[test]
fn comments_strings_other_receivers_and_runtime_text_are_ignored() {
    contract(
        "i18n-ignored",
        r#"
Widget build(BuildContext context) {
  // TranslatedText('commented'); context.tr('commented_too');
  final sample = 'TranslatedText("inside_string")';
  return Column(children: [
    TranslatedText.dynamic(runtimeKey),
    ui.TranslatedText('prefixed'), const ui.TranslatedText('const_prefixed'),
    myTranslatedText('different_name'), other.tr('different_receiver'),
    Text(runtimeValue),
  ]);
}
"#,
        0,
        0,
    );
}

#[test]
fn shopping_parts_keep_complete_entries_and_diagnostics() {
    let parts = [
        include_str!(
            "../../../../examples/shopping_app/lib/features/products/views/products_screen.dart"
        ),
        include_str!(
            "../../../../examples/shopping_app/lib/features/products/views/products_screen_chrome.dart"
        ),
        include_str!(
            "../../../../examples/shopping_app/lib/features/products/views/products_screen_list.dart"
        ),
        include_str!(
            "../../../../examples/shopping_app/lib/features/products/views/products_screen_card.dart"
        ),
    ];
    let mut combined = I18nScanResult::default();
    for (index, part) in parts.iter().enumerate() {
        let result = scan_i18n_source(&SourceText::new(FileId::new(index as u32), *part));
        assert!(result.diagnostics.is_empty(), "{result:?}");
        combined.entries.extend(result.entries);
        combined.diagnostics.extend(result.diagnostics);
    }
    assert!(!combined.entries.is_empty());
    println!("DUST_PARSER_PARITY:i18n-shopping:{combined:?}");
}
