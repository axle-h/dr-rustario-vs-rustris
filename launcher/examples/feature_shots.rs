//! Draws every input of the Dr. Rustario network: the bottle before the pill, and the bottles
//! the placements that score highest and lowest on that input leave behind. The positions are
//! [`dr_rustario::game::ai::explain::scenarios`]'s, the same ones `ga dr explain` reports.
//!
//! `cargo run -p dr-rustario-vs-rustris --example feature_shots -- out/ [theme]`

#[path = "common/bottle_shots.rs"]
mod bottle_shots;

use bottle_shots::Shooter;
use dr_rustario::game::ai::explain::{scenarios, INPUTS};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| ".".to_string());
    let theme = args.get(2).cloned().unwrap_or_else(|| "nes".to_string());
    let mut shooter = Shooter::new("feature-shots", &theme)?;

    // the manifest is written from the same loop as the shots, so labels cannot drift from
    // their pictures
    let mut manifest = vec![];
    let mut count = 0;
    for scenario in scenarios() {
        let name = INPUTS[scenario.input].name.replace('.', "-");
        // the bottle, then each placement as it lands (clears popping) and after it settles
        let shots = [
            ("before", &scenario.before, None, [].as_slice()),
            (
                "a-landed",
                &scenario.landed[0],
                Some(scenario.placed[0]),
                scenario.destroyed[0].as_slice(),
            ),
            ("a-after", &scenario.after[0], None, [].as_slice()),
            (
                "b-landed",
                &scenario.landed[1],
                Some(scenario.placed[1]),
                scenario.destroyed[1].as_slice(),
            ),
            ("b-after", &scenario.after[1], None, [].as_slice()),
        ];
        for (what, bottle, placed, destroyed) in shots {
            let path = format!("{out}/{:02}-{name}-{what}.png", scenario.input);
            shooter.shoot(bottle, placed, destroyed, &path)?;
            count += 1;
        }
        let viruses_before = scenario.before.virus_count();
        let popped = |at: usize| {
            scenario.destroyed[at]
                .iter()
                .filter(|point| {
                    scenario.landed[at]
                        .block_at(point.x() as u32, point.y() as u32)
                        .is_virus()
                })
                .count()
        };
        manifest.push(format!(
            r#"  {{"input": {}, "name": "{}", "value": [{}, {}],
   "cells_popped": [{}, {}], "viruses_popped": [{}, {}], "rounds": [{}, {}],
   "found": {}, "viruses_before": {}, "viruses_after": [{}, {}]}}"#,
            scenario.input,
            INPUTS[scenario.input].name,
            scenario.value[0],
            scenario.value[1],
            scenario.destroyed[0].len(),
            scenario.destroyed[1].len(),
            popped(0),
            popped(1),
            scenario.rounds[0],
            scenario.rounds[1],
            scenario.found,
            viruses_before,
            scenario.after[0].virus_count(),
            scenario.after[1].virus_count(),
        ));

        println!(
            "{:<26} {:>8.1} ({} popping) vs {:>8.1} ({} popping){}",
            INPUTS[scenario.input].name,
            scenario.value[0],
            scenario.destroyed[0].len(),
            scenario.value[1],
            scenario.destroyed[1].len(),
            if scenario.separates() {
                ""
            } else {
                "   <- the same value twice"
            }
        );
    }
    let path = format!("{out}/manifest.json");
    std::fs::write(&path, format!("[\n{}\n]\n", manifest.join(",\n")))
        .map_err(|e| e.to_string())?;
    println!("\n{count} shots and {path}");
    Ok(())
}
