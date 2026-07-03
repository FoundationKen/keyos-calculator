mod calculator;
mod theme;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use calculator::Calculator;
use slint_keyos_platform::{app_minimal, slint::SharedString};

app_minimal!("Calculator");

fn app_main(_cx: AppContext, ui: AppWindow) {
    log_server::init_wait(env!("CARGO_CRATE_NAME")).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    theme::init(&ui);

    let calculator = Rc::new(RefCell::new(Calculator::new()));
    sync_ui(&ui, &calculator.borrow());

    let ui_weak = ui.as_weak();
    ui.global::<Callbacks>().on_input(move |input| {
        let total_start = Instant::now();
        let mut calculator = calculator.borrow_mut();
        let calc_start = Instant::now();
        calculator.press(input.as_str());
        let calc_us = calc_start.elapsed().as_micros();

        if let Some(ui) = ui_weak.upgrade() {
            let sync_start = Instant::now();
            let changed = sync_ui(&ui, &calculator);
            let sync_us = sync_start.elapsed().as_micros();
            log::info!(
                "calc-perf input={} calc_us={} sync_us={} total_us={} changed_props={}",
                input,
                calc_us,
                sync_us,
                total_start.elapsed().as_micros(),
                changed
            );
        } else {
            log::info!(
                "calc-perf input={} calc_us={} total_us={} ui_gone=true",
                input,
                calc_us,
                total_start.elapsed().as_micros()
            );
        }
    });

    ui.run().expect("UI running");
}

fn sync_ui(ui: &AppWindow, calculator: &Calculator) -> usize {
    let mut changed = 0;
    let formula_text = calculator.formula_text();
    let answer_text = calculator.answer_text();
    let result_mode = calculator.result_mode();
    let primary_text = if result_mode {
        answer_text.to_string()
    } else if formula_text.is_empty() {
        answer_text.to_string()
    } else {
        formula_text.clone()
    };
    let secondary_text = if result_mode {
        formula_text.clone()
    } else {
        String::new()
    };

    let primary_count = primary_text.chars().count() as i32;
    let secondary_count = secondary_text.chars().count() as i32;

    if ui.get_primary_count() != primary_count {
        ui.set_primary_count(primary_count);
        changed += 1;
    }
    if ui.get_secondary_count() != secondary_count {
        ui.set_secondary_count(secondary_count);
        changed += 1;
    }
    if set_string_if_changed(ui.get_primary_text(), &primary_text, |value| {
        ui.set_primary_text(value);
    }) {
        changed += 1;
    }
    if set_string_if_changed(ui.get_secondary_text(), &secondary_text, |value| {
        ui.set_secondary_text(value);
    }) {
        changed += 1;
    }
    if ui.get_scientific_mode() != calculator.scientific_mode {
        ui.set_scientific_mode(calculator.scientific_mode);
        changed += 1;
    }
    if ui.get_second_mode() != calculator.second_mode {
        ui.set_second_mode(calculator.second_mode);
        changed += 1;
    }
    if ui.get_radians_mode() != calculator.radians_mode {
        ui.set_radians_mode(calculator.radians_mode);
        changed += 1;
    }
    if set_string_if_changed(ui.get_clear_label(), calculator.clear_label(), |value| {
        ui.set_clear_label(value);
    }) {
        changed += 1;
    }
    changed
}

fn set_string_if_changed(
    current: SharedString,
    next: &str,
    setter: impl FnOnce(SharedString),
) -> bool {
    if current.as_str() != next {
        setter(next.into());
        true
    } else {
        false
    }
}
