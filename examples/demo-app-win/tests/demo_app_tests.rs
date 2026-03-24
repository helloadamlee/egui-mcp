//! Example tests using egui_kittest for the demo app
//!
//! These are IN-PROCESS tests - they run the UI directly in the test harness.
//! This is different from the MCP server which controls the app from an external process.

use egui::accesskit::Toggled;
use egui_kittest::{kittest::Queryable, Harness};

#[test]
fn test_slider_interaction() {
    // Create a test harness with just the UI logic
    let mut slider_value = 50.0;

    let app = move |ui: &mut egui::Ui| {
        ui.add(egui::Slider::new(&mut slider_value, 0.0..=100.0).text("Slider"));
        ui.label(format!("Slider value: {:.1}", slider_value));
    };

    let mut harness = Harness::new_ui(app);

    // Find the slider by its label
    let slider = harness.get_by_role(egui::accesskit::Role::Slider);
    assert_eq!(slider.role(), egui::accesskit::Role::Slider);

    // In a real test, you'd interact with it
    // Note: Slider interaction in kittest may require different approach
    // This is just an example structure

    harness.run();
}

#[test]
fn test_checkbox_interaction() {
    let mut checked = false;

    let app = move |ui: &mut egui::Ui| {
        ui.checkbox(&mut checked, "Test checkbox");
    };

    let mut harness = Harness::new_ui(app);

    // Find checkbox by label
    let checkbox = harness.get_by_label("Test checkbox");

    // Verify initial state
    assert_eq!(checkbox.toggled(), Some(Toggled::False));

    // Click it
    checkbox.click();

    // Run the UI to process the event
    harness.run();

    // Verify it's now checked
    let checkbox = harness.get_by_label("Test checkbox");
    assert_eq!(checkbox.toggled(), Some(Toggled::True));
}

#[test]
fn test_find_by_role() {
    let app = |ui: &mut egui::Ui| {
        let _ = ui.button("Click me");
        ui.label("Some text");
    };

    let harness = Harness::new_ui(app);

    // Find all buttons
    let buttons = harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .collect::<Vec<_>>();
    assert_eq!(buttons.len(), 1);

    let button = buttons[0];
    assert_eq!(button.label(), Some("Click me".to_string()));
}
